//! Actor message handler implementation.

use std::marker::PhantomData;

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use crate::actor_system::{
    actor::{ActorContext, Handler, Message},
    system::SystemEvent,
};

use super::Actor;

#[async_trait]
pub trait MessageHandler<E: SystemEvent, A: Actor<E>>: Send + Sync {
    async fn handle(&mut self, actor: &mut A, ctx: &mut ActorContext<E>);
}

pub(crate) struct ActorMessage<M, E, A>
where
    M: Message,
    E: SystemEvent,
    A: Handler<E, M>,
{
    payload: M,
    rsvp: Option<oneshot::Sender<M::Response>>,
    _phantom_actor: PhantomData<A>,
    _phantom_event: PhantomData<E>,
}

#[async_trait]
impl<M, E, A> MessageHandler<E, A> for ActorMessage<M, E, A>
where
    M: Message,
    E: SystemEvent,
    A: Handler<E, M>,
{
    async fn handle(&mut self, actor: &mut A, ctx: &mut ActorContext<E>) {
        let result = actor.handle(self.payload.clone(), ctx).await;

        if let Some(rsvp) = self.rsvp.take() {
            rsvp.send(result).unwrap_or_else(|_failed| {
                log::error!("Failed to send back response!");
            })
        }
    }
}

impl<M, E, A> ActorMessage<M, E, A>
where
    M: Message,
    E: SystemEvent,
    A: Handler<E, M>,
{
    pub fn new(msg: M, rsvp: Option<oneshot::Sender<M::Response>>) -> Self {
        ActorMessage {
            payload: msg,
            rsvp,
            _phantom_actor: PhantomData,
            _phantom_event: PhantomData,
        }
    }
}

pub type BoxedMessageHandler<E, A> = Box<dyn MessageHandler<E, A>>;
pub type MailboxReceiver<E, A> = mpsc::UnboundedReceiver<BoxedMessageHandler<E, A>>;
pub type MailboxSender<E, A> = mpsc::UnboundedSender<BoxedMessageHandler<E, A>>;

pub struct ActorMailbox<E: SystemEvent, A: Actor<E>> {
    _phantom_actor: PhantomData<A>,
    _phantom_event: PhantomData<E>,
}

impl<E: SystemEvent, A: Actor<E>> ActorMailbox<E, A> {
    pub fn create() -> (MailboxSender<E, A>, MailboxReceiver<E, A>) {
        mpsc::unbounded_channel()
    }
}
