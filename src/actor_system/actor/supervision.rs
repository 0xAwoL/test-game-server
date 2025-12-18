//! Actor supervision strategies.

#![allow(dead_code)]

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use backoff::backoff::Backoff as InnerBackoff;

/// Defines what to do when an actor fails at startup.
#[derive(Debug)]
pub enum SupervisionStrategy {
    Stop,
    Retry(Box<dyn RetryStrategy>),
}

/// Trait to define a retry strategy.
pub trait RetryStrategy: std::fmt::Debug + Send + Sync {
    /// Maximum number of tries before permanently failing an actor.
    fn max_retries(&self) -> usize;
    /// Wait duration before retrying.
    fn next_backoff(&mut self) -> Option<Duration>;
}

/// A retry strategy that immediately retries without waiting.
#[derive(Debug, Default)]
pub struct NoIntervalStrategy {
    max_retries: usize,
}

impl NoIntervalStrategy {
    pub fn new(max_retries: usize) -> Self {
        NoIntervalStrategy { max_retries }
    }
}

impl RetryStrategy for NoIntervalStrategy {
    fn max_retries(&self) -> usize {
        self.max_retries
    }

    fn next_backoff(&mut self) -> Option<Duration> {
        None
    }
}

/// A retry strategy with a fixed wait period.
#[derive(Debug, Default)]
pub struct FixedIntervalStrategy {
    max_retries: usize,
    duration: Duration,
}

impl FixedIntervalStrategy {
    pub fn new(max_retries: usize, duration: Duration) -> Self {
        FixedIntervalStrategy {
            max_retries,
            duration,
        }
    }
}

impl RetryStrategy for FixedIntervalStrategy {
    fn max_retries(&self) -> usize {
        self.max_retries
    }

    fn next_backoff(&mut self) -> Option<Duration> {
        Some(self.duration)
    }
}

/// A retry strategy with exponential backoff.
#[derive(Debug, Default)]
pub struct ExponentialBackoffStrategy {
    max_retries: usize,
    inner: Arc<Mutex<backoff::ExponentialBackoff>>,
}

impl ExponentialBackoffStrategy {
    pub fn new(max_retries: usize) -> Self {
        ExponentialBackoffStrategy {
            max_retries,
            inner: Arc::new(Mutex::new(backoff::ExponentialBackoff::default())),
        }
    }
}

impl RetryStrategy for ExponentialBackoffStrategy {
    fn max_retries(&self) -> usize {
        self.max_retries
    }

    fn next_backoff(&mut self) -> Option<Duration> {
        self.inner.lock().ok().and_then(|mut eb| eb.next_backoff())
    }
}
