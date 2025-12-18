//! Actor module - core actor types and traits.

#![allow(dead_code)]

pub(crate) mod handler;
pub(crate) mod runner;
pub(crate) mod supervision;

use async_trait::async_trait;
use thiserror::Error;

use tokio::sync::{mpsc, oneshot};
use tokio::time::Duration;

mod path;
pub use path::ActorPath;

use supervision::SupervisionStrategy;

use crate::actor_system::system::{ActorSystem, SystemEvent};

/// The actor context gives a running actor access to its path and the system.
#[derive(Debug)]
pub struct ActorContext<E: SystemEvent> {
    pub path: ActorPath,
    pub system: ActorSystem<E>,
}

impl<E: SystemEvent> ActorContext<E> {
    /// Create a child actor under this actor.
    pub async fn create_child<A: Actor<E>>(
        &self,
        name: &str,
        actor: A,
    ) -> Result<ActorRef<E, A>, ActorError> {
        let path = self.path.clone() / name;
        self.system.create_actor_path(path, actor).await
    }

    /// Retrieve a child actor running under this actor.
    pub async fn get_child<A: Actor<E>>(&self, name: &str) -> Option<ActorRef<E, A>> {
        let path = self.path.clone() / name;
        self.system.get_actor(&path).await
    }

    /// Retrieve or create a new child under this actor if it does not exist yet.
    pub async fn get_or_create_child<A, F>(
        &self,
        name: &str,
        actor_fn: F,
    ) -> Result<ActorRef<E, A>, ActorError>
    where
        A: Actor<E>,
        F: FnOnce() -> A,
    {
        let path = self.path.clone() / name;
        self.system.get_or_create_actor_path(&path, actor_fn).await
    }

    /// Stops the child actor.
    pub async fn stop_child(&self, name: &str) {
        let path = self.path.clone() / name;
        self.system.stop_actor(&path).await;
    }

    pub(crate) async fn restart<A>(
        &mut self,
        actor: &mut A,
        error: Option<&ActorError>,
    ) -> Result<(), ActorError>
    where
        A: Actor<E>,
    {
        actor.pre_restart(self, error).await
    }
}

/// Defines what an actor will receive as its message, and with what it should respond.
pub trait Message: Clone + Send + Sync + 'static {
    /// Response an actor should give when it receives this message.
    type Response: Send + Sync + 'static;
}

/// Basic trait for actors.
#[async_trait]
pub trait Actor<E: SystemEvent>: Send + Sync + 'static {
    /// Defines the timeout to set for this actor.
    fn timeout() -> Option<Duration> {
        None
    }

    /// Defines the supervision strategy to use for this actor.
    fn supervision_strategy() -> SupervisionStrategy {
        SupervisionStrategy::Stop
    }

    /// Override this function to perform initialization of the actor.
    async fn pre_start(&mut self, _ctx: &mut ActorContext<E>) -> Result<(), ActorError> {
        Ok(())
    }

    /// Override this function to define what should happen on restart.
    async fn pre_restart(
        &mut self,
        ctx: &mut ActorContext<E>,
        _error: Option<&ActorError>,
    ) -> Result<(), ActorError> {
        self.pre_start(ctx).await
    }

    /// Override this function to perform work when the actor is stopped.
    async fn post_stop(&mut self, _ctx: &mut ActorContext<E>) {}
}

/// Defines what the actor does with a message.
#[async_trait]
pub trait Handler<E: SystemEvent, M: Message>: Actor<E> {
    async fn handle(&mut self, msg: M, ctx: &mut ActorContext<E>) -> M::Response;
}

/// A clonable actor reference.
pub struct ActorRef<E: SystemEvent, A: Actor<E>> {
    path: ActorPath,
    sender: mpsc::UnboundedSender<handler::BoxedMessageHandler<E, A>>,
}

impl<E: SystemEvent, A: Actor<E>> Clone for ActorRef<E, A> {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<E: SystemEvent, A: Actor<E>> ActorRef<E, A> {
    /// Get the path of this actor.
    pub fn path(&self) -> &ActorPath {
        &self.path
    }

    /// Fire and forget sending of messages to this actor.
    pub fn tell<M>(&self, msg: M) -> Result<(), ActorError>
    where
        M: Message,
        A: Handler<E, M>,
    {
        let message = handler::ActorMessage::<M, E, A>::new(msg, None);
        if let Err(error) = self.sender.send(Box::new(message)) {
            log::error!("Failed to tell message! {}", error.to_string());
            Err(ActorError::SendError(error.to_string()))
        } else {
            Ok(())
        }
    }

    /// Send a message to an actor, expecting a response.
    pub async fn ask<M>(&self, msg: M) -> Result<M::Response, ActorError>
    where
        M: Message,
        A: Handler<E, M>,
    {
        let (response_sender, response_receiver) = oneshot::channel();
        let message = handler::ActorMessage::<M, E, A>::new(msg, Some(response_sender));
        if let Err(error) = self.sender.send(Box::new(message)) {
            log::error!("Failed to ask message! {}", error.to_string());
            Err(ActorError::SendError(error.to_string()))
        } else {
            response_receiver
                .await
                .map_err(|error| ActorError::SendError(error.to_string()))
        }
    }

    /// Checks if the actor mailbox is still open.
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }

    pub(crate) fn new(path: ActorPath, sender: handler::MailboxSender<E, A>) -> Self {
        ActorRef { path, sender }
    }
}

impl<E: SystemEvent, A: Actor<E>> std::fmt::Debug for ActorRef<E, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

#[derive(Error, Debug)]
pub enum ActorError {
    #[error("Actor exists")]
    Exists(ActorPath),

    #[error("Actor creation failed")]
    CreateError(String),

    #[error("Sending message failed")]
    SendError(String),

    #[error("Actor runtime error")]
    RuntimeError(anyhow::Error),
}

impl ActorError {
    pub fn new<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::RuntimeError(anyhow::Error::new(error))
    }
}
