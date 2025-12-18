//! Tiny Tokio Actor - Lightweight actor framework.
//!
//! A minimally functioning actor system with a common event bus built on tokio.
//! Provides request/response patterns via `tell` (fire-and-forget) and `ask` (with response).

mod actor;
mod bus;
mod system;

pub use actor::{Actor, ActorContext, ActorError, ActorPath, ActorRef, Handler, Message};

pub use bus::EventBus;
pub use system::{ActorSystem, SystemEvent};

pub use async_trait::async_trait;
