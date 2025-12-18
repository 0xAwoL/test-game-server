use crate::actor_system::{Actor, ActorContext, ActorError, Handler, async_trait};
use crate::anticheat::{ValidationResult, validate_movement};
use crate::player::state::{GetState, Kick, MovePlayer, SendMessage};
use crate::types::{GameEvent, MAX_SPEED, MAX_VIOLATIONS, PlayerState, Position, ServerMessage};
use std::time::Instant;
use tokio::sync::mpsc;
use warp::ws::Message as WsMessage;

pub struct PlayerActor {
    pub player_id: String,
    pub wallet: String,
    pub nickname: String,
    pub position: Position,
    pub velocity: Position,
    pub last_update: Instant,
    pub violations: u32,
    ws_sender: mpsc::UnboundedSender<WsMessage>,
}

impl PlayerActor {
    pub fn new(
        player_id: String,
        wallet: String,
        nickname: String,
        ws_sender: mpsc::UnboundedSender<WsMessage>,
    ) -> Self {
        Self {
            player_id,
            wallet,
            nickname,
            position: Position::default(),
            velocity: Position::default(),
            last_update: Instant::now(),
            violations: 0,
            ws_sender,
        }
    }

    fn send_to_client(&self, msg: ServerMessage) {
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = self.ws_sender.send(WsMessage::text(json));
        }
    }

    fn handle_violation(&mut self, violation_type: &str, details: &str) {
        self.violations += 1;
        log::warn!(
            "Player {} {} | {} | Violations: {}/{}",
            self.player_id,
            violation_type,
            details,
            self.violations,
            MAX_VIOLATIONS
        );

        self.send_to_client(ServerMessage::Error {
            message: format!(
                "{} detected. Violations: {}/{}",
                violation_type, self.violations, MAX_VIOLATIONS
            ),
        });

        if self.violations >= MAX_VIOLATIONS {
            log::error!("Player {} KICKED for too many violations", self.player_id);
            self.send_to_client(ServerMessage::Kicked {
                reason: "Too many anti-cheat violations".to_string(),
            });
        }
    }
}

#[async_trait]
impl Actor<GameEvent> for PlayerActor {
    async fn pre_start(&mut self, ctx: &mut ActorContext<GameEvent>) -> Result<(), ActorError> {
        log::debug!(
            "Player {} ({}) joined at ({:.2}, {:.2}, {:.2})",
            self.player_id,
            self.nickname,
            self.position.x,
            self.position.y,
            self.position.z
        );

        ctx.system.publish(GameEvent::PlayerJoined {
            player_id: self.player_id.clone(),
            wallet: self.wallet.clone(),
            position: self.position.clone(),
        });
        Ok(())
    }

    async fn post_stop(&mut self, ctx: &mut ActorContext<GameEvent>) {
        log::debug!(
            "Player {} ({}) left the game",
            self.player_id,
            self.nickname
        );

        ctx.system.publish(GameEvent::PlayerLeft {
            player_id: self.player_id.clone(),
        });
    }
}

#[async_trait]
impl Handler<GameEvent, MovePlayer> for PlayerActor {
    async fn handle(&mut self, msg: MovePlayer, ctx: &mut ActorContext<GameEvent>) {
        let validation = validate_movement(
            &self.position,
            &msg.position,
            &msg.velocity,
            msg.delta_time,
            MAX_SPEED,
        );

        match validation {
            ValidationResult::Valid => {
                self.position = msg.position;
                self.velocity = msg.velocity;
                self.last_update = Instant::now();
                self.violations = 0;

                log::debug!(
                    "Player {} moved to ({:.2}, {:.2}, {:.2})",
                    self.player_id,
                    self.position.x,
                    self.position.y,
                    self.position.z
                );

                ctx.system.publish(GameEvent::PlayerMoved {
                    player_id: self.player_id.clone(),
                    position: self.position.clone(),
                    velocity: self.velocity.clone(),
                });
            }
            ValidationResult::SpeedHack => {
                self.handle_violation(
                    "SPEED HACK",
                    &format!(
                        "({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2})",
                        self.position.x,
                        self.position.y,
                        self.position.z,
                        msg.position.x,
                        msg.position.y,
                        msg.position.z
                    ),
                );
            }
            ValidationResult::Teleport => {
                self.handle_violation(
                    "TELEPORT",
                    &format!("Distance: {:.2}", self.position.distance_to(&msg.position)),
                );
            }
            ValidationResult::OutOfBounds => {
                log::warn!(
                    "Player {} OUT OF BOUNDS: ({:.2}, {:.2}, {:.2})",
                    self.player_id,
                    msg.position.x,
                    msg.position.y,
                    msg.position.z
                );

                self.send_to_client(ServerMessage::Error {
                    message: "Position out of bounds".to_string(),
                });
            }
        }
    }
}

#[async_trait]
impl Handler<GameEvent, GetState> for PlayerActor {
    async fn handle(&mut self, _msg: GetState, _ctx: &mut ActorContext<GameEvent>) -> PlayerState {
        PlayerState {
            player_id: self.player_id.clone(),
            wallet: self.wallet.clone(),
            nickname: self.nickname.clone(),
            position: self.position.clone(),
            velocity: self.velocity.clone(),
            last_update: self.last_update,
            previous_position: self.position.clone(),
            violations: self.violations,
        }
    }
}

#[async_trait]
impl Handler<GameEvent, Kick> for PlayerActor {
    async fn handle(&mut self, msg: Kick, _ctx: &mut ActorContext<GameEvent>) {
        self.send_to_client(ServerMessage::Kicked { reason: msg.reason });
    }
}

#[async_trait]
impl Handler<GameEvent, SendMessage> for PlayerActor {
    async fn handle(&mut self, msg: SendMessage, _ctx: &mut ActorContext<GameEvent>) {
        let _ = self.ws_sender.send(WsMessage::text(msg.message));
    }
}
