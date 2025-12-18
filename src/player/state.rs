use crate::actor_system::Message;
use crate::types::Position;

#[derive(Clone, Debug)]
pub struct MovePlayer {
    pub position: Position,
    pub velocity: Position,
    pub delta_time: f32,
}

impl Message for MovePlayer {
    type Response = ();
}

#[derive(Clone, Debug)]
pub struct GetState;

impl Message for GetState {
    type Response = crate::types::PlayerState;
}

#[derive(Clone, Debug)]
pub struct Kick {
    pub reason: String,
}

impl Message for Kick {
    type Response = ();
}

#[derive(Clone, Debug)]
pub struct SendMessage {
    pub message: String,
}

impl Message for SendMessage {
    type Response = ();
}
