use crate::actor_system::ActorSystem;
use crate::config::ServerConfig;
use crate::network::ConnectionManager;
use crate::types::{GameEvent, PlayerState, Position, ServerMessage};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use warp::ws::Message as WsMessage;

pub async fn broadcast_positions(
    system: ActorSystem<GameEvent>,
    connection_manager: ConnectionManager,
    config: &ServerConfig,
) {
    log::info!(
        "Starting broadcast loop: {}ms tickrate (~{:.1} FPS)",
        config.tickrate_ms,
        1000.0 / config.tickrate_ms as f64
    );

    let mut ticker = interval(Duration::from_millis(config.tickrate_ms));
    let player_states: Arc<DashMap<String, PlayerState>> = Arc::new(DashMap::new());

    let mut events = system.events();
    let states_clone = player_states.clone();

    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => handle_game_event(event, &states_clone),
                Err(_) => break,
            }
        }
    });

    let mut tick_count = 0u64;
    let mut last_stats_log = std::time::Instant::now();

    loop {
        ticker.tick().await;
        tick_count += 1;

        let all_players: Vec<PlayerState> = player_states
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        let msg = ServerMessage::StateUpdate {
            players: all_players.clone(),
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            connection_manager.broadcast(WsMessage::text(json));

            // Log stats every 5 seconds
            if last_stats_log.elapsed().as_secs() >= 5 {
                let actual_fps = tick_count as f64 / 5.0;
                log::debug!(
                    "Broadcast: {:.1} FPS, {} players, {} connections",
                    actual_fps,
                    all_players.len(),
                    connection_manager.count()
                );
                tick_count = 0;
                last_stats_log = std::time::Instant::now();
            }
        }
    }
}

fn handle_game_event(event: GameEvent, states: &DashMap<String, PlayerState>) {
    match event {
        GameEvent::PlayerJoined {
            player_id,
            wallet,
            position,
        } => {
            let nickname = format!(
                "Player_{}",
                player_id.strip_prefix("player_").unwrap_or(&player_id)
            );

            log::debug!(
                "Player {} joined at ({:.2}, {:.2}, {:.2})",
                player_id,
                position.x,
                position.y,
                position.z
            );

            states.insert(
                player_id.clone(),
                PlayerState {
                    player_id,
                    wallet,
                    nickname,
                    position: position.clone(),
                    velocity: Position::default(),
                    last_update: std::time::Instant::now(),
                    previous_position: position,
                    violations: 0,
                },
            );
        }
        GameEvent::PlayerMoved {
            player_id,
            position,
            velocity,
        } => {
            if let Some(mut state) = states.get_mut(&player_id) {
                state.previous_position = state.position.clone();
                state.position = position;
                state.velocity = velocity;
                state.last_update = std::time::Instant::now();
            }
        }
        GameEvent::PlayerLeft { player_id } => {
            log::debug!("Player {} left", player_id);
            states.remove(&player_id);
        }
    }
}
