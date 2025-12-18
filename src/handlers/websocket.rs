use crate::actor_system::{ActorPath, ActorRef, ActorSystem};
use crate::network::ConnectionManager;
use crate::player::{MovePlayer, PlayerActor};
use crate::types::{Claims, ClientMessage, GameEvent, SessionInfo};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::WebSocket;

const MAX_MOVES_PER_SECOND: u32 = 60;

const RATE_LIMIT_WINDOW_MS: u128 = 1000;

pub async fn handle_connection(
    token: String,
    system: ActorSystem<GameEvent>,
    sessions: Arc<DashMap<String, SessionInfo>>,
    jwt_secret: String,
    websocket: WebSocket,
    debug_mode: bool,
    connection_manager: ConnectionManager,
) {
    let claims = match authenticate(&token, &jwt_secret, debug_mode) {
        Some(c) => c,
        None => return,
    };

    if !debug_mode && !sessions.contains_key(&claims.wallet_address) {
        log::error!("Session not found for wallet: {}", claims.wallet_address);
        return;
    }

    let (mut ws_tx, mut ws_rx) = websocket.split();
    let (sender, receiver) = mpsc::unbounded_channel();
    let mut receiver_stream = UnboundedReceiverStream::new(receiver);

    log::debug!(
        "WebSocket connected - Player: {}, Wallet: {}",
        claims.player_id,
        claims.wallet_address
    );

    let actor_name = format!("player-{}", claims.player_id);
    let actor_path = ActorPath::from(format!("/user/{}", actor_name));

    system.stop_actor(&actor_path).await;
    connection_manager.remove(&claims.player_id);
    connection_manager.add(claims.player_id.clone(), sender.clone());

    tokio::spawn(async move {
        while let Some(msg) = receiver_stream.next().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let actor = PlayerActor::new(
        claims.player_id.clone(),
        claims.wallet_address.clone(),
        claims.nickname.clone(),
        sender,
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let actor_ref = match system.create_actor(&actor_name, actor).await {
        Ok(r) => {
            log::debug!("Created actor for player: {}", claims.player_id);
            r
        }
        Err(e) => {
            log::error!(
                "Failed to create actor for player {}: {:?}",
                claims.player_id,
                e
            );
            return;
        }
    };

    let mut move_count: u32 = 0;
    let mut window_start = std::time::Instant::now();

    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(text) {
                        process_message(
                            client_msg,
                            &actor_ref,
                            &claims.player_id,
                            &mut move_count,
                            &mut window_start,
                        );
                    }
                }
            }
            Err(_) => break,
        }
    }

    log::debug!(
        "WebSocket disconnected - Player: {}, Nickname: {}",
        claims.player_id,
        claims.nickname
    );
    connection_manager.remove(&claims.player_id);
    system.stop_actor(actor_ref.path()).await;
}

fn authenticate(token: &str, jwt_secret: &str, debug_mode: bool) -> Option<Claims> {
    if debug_mode && token.starts_with("debug_") {
        let session_id = rand::random::<u64>().to_string();
        return Some(Claims {
            wallet_address: format!("debug_{}", session_id),
            player_id: format!("player_{}", session_id),
            nickname: format!("Player_{}", session_id),
            exp: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize,
        });
    }

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(token_data) => Some(token_data.claims),
        Err(e) => {
            log::error!("JWT decode error: {}", e);
            None
        }
    }
}

fn process_message(
    msg: ClientMessage,
    actor_ref: &ActorRef<GameEvent, PlayerActor>,
    player_id: &str,
    move_count: &mut u32,
    window_start: &mut std::time::Instant,
) {
    match msg {
        ClientMessage::Move {
            position,
            velocity,
            delta_time,
        } => {
            let now = std::time::Instant::now();
            let elapsed_ms = now.duration_since(*window_start).as_millis();

            if elapsed_ms >= RATE_LIMIT_WINDOW_MS {
                *window_start = now;
                *move_count = 0;
            }

            *move_count += 1;

            if *move_count > MAX_MOVES_PER_SECOND {
                log::debug!(
                    "Rate limited player {}: {} moves/sec",
                    player_id,
                    move_count
                );
                return;
            }

            let _ = actor_ref.tell(MovePlayer {
                position,
                velocity,
                delta_time,
            });
        }
        ClientMessage::GetState => {
            // Handled by broadcast loop
        }
    }
}
