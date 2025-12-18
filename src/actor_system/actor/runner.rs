//! Actor runner - manages actor lifecycle.

use crate::actor_system::system::{ActorSystem, SystemEvent};

use super::{
    Actor, ActorContext, ActorPath, ActorRef, SupervisionStrategy,
    handler::{ActorMailbox, MailboxReceiver},
};

pub(crate) struct ActorRunner<E: SystemEvent, A: Actor<E>> {
    path: ActorPath,
    actor: A,
    receiver: MailboxReceiver<E, A>,
}

impl<E: SystemEvent, A: Actor<E>> ActorRunner<E, A> {
    pub fn create(path: ActorPath, actor: A) -> (Self, ActorRef<E, A>) {
        let (sender, receiver) = ActorMailbox::create();
        let actor_ref = ActorRef::new(path.clone(), sender);
        let runner = ActorRunner {
            path,
            actor,
            receiver,
        };
        (runner, actor_ref)
    }

    pub async fn start(&mut self, system: ActorSystem<E>) {
        log::debug!("Starting actor '{}'...", &self.path);

        let mut ctx = ActorContext {
            path: self.path.clone(),
            system: system.clone(),
        };

        // Start the actor
        let mut start_error = self.actor.pre_start(&mut ctx).await.err();

        // Handle supervision strategy if startup failed
        if start_error.is_some() {
            let mut retries = 0;
            match A::supervision_strategy() {
                SupervisionStrategy::Stop => {
                    log::error!("Actor '{}' failed to start!", &self.path);
                }
                SupervisionStrategy::Retry(mut retry_strategy) => {
                    log::debug!(
                        "Restarting actor with retry strategy: {:?}",
                        &retry_strategy
                    );
                    while retries < retry_strategy.max_retries() && start_error.is_some() {
                        log::debug!("retries: {}", &retries);
                        if let Some(duration) = retry_strategy.next_backoff() {
                            log::debug!("Backoff for {:?}", &duration);
                            tokio::time::sleep(duration).await;
                        }
                        retries += 1;
                        start_error = ctx
                            .restart(&mut self.actor, start_error.as_ref())
                            .await
                            .err();
                    }
                }
            }
        }

        // Run the actor if startup succeeded
        if start_error.is_none() {
            log::debug!("Actor '{}' has started successfully.", &self.path);

            if let Some(timeout) = A::timeout() {
                log::debug!("Timeout of {:?} set for actor {}", timeout, &self.path);
                while let Ok(Some(mut msg)) =
                    tokio::time::timeout(timeout, self.receiver.recv()).await
                {
                    msg.handle(&mut self.actor, &mut ctx).await;
                }
                log::debug!("Actor timed out after {:?} of inactivity.", timeout);
            } else {
                while let Some(mut msg) = self.receiver.recv().await {
                    msg.handle(&mut self.actor, &mut ctx).await;
                }
            }

            self.actor.post_stop(&mut ctx).await;
            system.stop_actor(&self.path).await;

            log::debug!("Actor '{}' stopped.", &self.path);
        }

        self.receiver.close();
    }
}
