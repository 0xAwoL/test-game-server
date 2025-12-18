mod actor_system;
mod anticheat;
mod config;
mod handlers;
mod network;
mod player;
mod types;

use actor_system::{ActorSystem, EventBus};
use config::ServerConfig;
use dashmap::DashMap;
use handlers::{SolanaVerifier, handle_auth};
use network::{ConnectionManager, broadcast_positions};
use std::collections::HashMap;
use std::sync::Arc;
use types::{AuthRequest, GameEvent, SessionInfo};
use warp::Filter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = ServerConfig::from_env();

    if config.debug_mode {
        log::warn!("DEBUG MODE ENABLED - Wallet verification disabled!");
    }

    let verifier = Arc::new(
        SolanaVerifier::new(&config.rpc_url, &config.token_mint, config.debug_mode)
            .expect("Failed to initialize Solana verifier"),
    );

    let bus = EventBus::<GameEvent>::new(1000);
    let system = ActorSystem::new("game", bus);
    let sessions: Arc<DashMap<String, SessionInfo>> = Arc::new(DashMap::new());
    let connection_manager = ConnectionManager::new();

    let broadcast_system = system.clone();
    let broadcast_manager = connection_manager.clone();
    let broadcast_config = config.clone();
    tokio::spawn(async move {
        broadcast_positions(broadcast_system, broadcast_manager, &broadcast_config).await;
    });

    let verifier_filter = warp::any().map(move || verifier.clone());
    let sessions_filter = warp::any().map(move || sessions.clone());
    let system_filter = warp::any().map(move || system.clone());
    let jwt_secret = config.jwt_secret.clone();
    let jwt_secret_filter = warp::any().map(move || jwt_secret.clone());
    let debug_mode = config.debug_mode;
    let debug_mode_filter = warp::any().map(move || debug_mode);
    let connection_manager_game = connection_manager.clone();
    let connection_manager_debug = connection_manager.clone();
    let connection_manager_filter = warp::any().map(move || connection_manager_game.clone());
    let debug_manager_filter = warp::any().map(move || connection_manager_debug.clone());

    // Auth route
    let auth_route = warp::path("auth")
        .and(warp::post())
        .and(warp::body::json::<AuthRequest>())
        .and(verifier_filter.clone())
        .and(sessions_filter.clone())
        .and(jwt_secret_filter.clone())
        .and_then(handle_auth);

    // Game WebSocket route
    let game_route = warp::path("game")
        .and(warp::query::<HashMap<String, String>>())
        .and(system_filter)
        .and(sessions_filter)
        .and(jwt_secret_filter)
        .and(debug_mode_filter)
        .and(connection_manager_filter)
        .and(warp::ws())
        .map(
            |params: HashMap<String, String>,
             system: ActorSystem<GameEvent>,
             sessions: Arc<DashMap<String, SessionInfo>>,
             jwt_secret: String,
             debug_mode: bool,
             connection_manager: ConnectionManager,
             ws: warp::ws::Ws| {
                let token = params.get("token").cloned().unwrap_or_default();
                ws.on_upgrade(move |websocket| {
                    handlers::handle_connection(
                        token,
                        system,
                        sessions,
                        jwt_secret,
                        websocket,
                        debug_mode,
                        connection_manager,
                    )
                })
            },
        );

    // Debug route
    let debug_route = warp::path("debug")
        .and(warp::path("players"))
        .and(debug_manager_filter)
        .map(|connection_manager: ConnectionManager| {
            let players = connection_manager.get_connected_players();
            warp::reply::json(&serde_json::json!({
                "connected_players": players,
                "count": players.len()
            }))
        });

    let routes = auth_route
        .or(game_route)
        .or(debug_route)
        .with(warp::log("game-server"));

    log::info!("Game server starting on port {}", config.port);
    warp::serve(routes).run(([0, 0, 0, 0], config.port)).await;
}
