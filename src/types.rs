use crate::actor_system::SystemEvent;
use serde::{Deserialize, Serialize};
use std::time::Instant;

pub const MAX_SPEED: f32 = 100.0;
pub const TELEPORT_THRESHOLD: f32 = 300.0;
pub const MAX_VIOLATIONS: u32 = 10;
pub const WORLD_BOUNDS: f32 = 1000.0;
pub const JWT_EXPIRATION_HOURS: i64 = 24;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    PlayerJoined {
        player_id: String,
        wallet: String,
        position: Position,
    },
    PlayerMoved {
        player_id: String,
        position: Position,
        velocity: Position,
    },
    PlayerLeft {
        player_id: String,
    },
}

impl SystemEvent for GameEvent {}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Move {
        position: Position,
        velocity: Position,
        delta_time: f32,
    },
    GetState,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    StateUpdate { players: Vec<PlayerState> },
    Error { message: String },
    Kicked { reason: String },
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerState {
    pub player_id: String,
    pub wallet: String,
    pub nickname: String,
    pub position: Position,
    pub velocity: Position,
    #[serde(skip)]
    pub last_update: Instant,
    #[serde(skip)]
    pub previous_position: Position,
    pub violations: u32,
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub wallet_address: String,
    pub signature: String,
    pub message: String,
    pub nickname: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub jwt_token: String,
    pub player_id: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub wallet_address: String,
    pub player_id: String,
    pub nickname: String,
    pub exp: usize,
}

#[derive(Clone)]
pub struct SessionInfo {
    pub jwt_token: String,
    pub nickname: String,
    pub created_at: Instant,
}
