//! Lifecycle state for asynchronous jobs whose result is delivered on a
//! `std::sync::mpsc` channel.
//!
//! Replaces the previous `Option<Receiver<T>>` pattern, where `None` was
//! overloaded to mean both "no job has ever been started" and "the job
//! finished and its result was already consumed". The two states are now
//! explicit, and `poll()` returns a `PollOutcome<T>` so callers handle
//! `Pending`, `Ready(T)`, and `Disconnected` separately.

use std::sync::mpsc::{Receiver, TryRecvError};

/// Asynchronous job state.
///
/// `Idle` means no job is currently in flight. `Running` means we are
/// awaiting a value on the inner channel. The state transitions back to
/// `Idle` automatically when `poll()` delivers a value or detects that
/// the sender has been dropped.
#[derive(Default)]
pub enum JobState<T> {
    #[default]
    Idle,
    Running(Receiver<T>),
}

/// Outcome of a single non-blocking poll.
pub enum PollOutcome<T> {
    /// No job is in flight, or the job is still running and no value is
    /// ready yet.
    Pending,
    /// A value was received. The job state has transitioned back to `Idle`.
    Ready(T),
    /// The sender was dropped without sending a value (worker thread
    /// panicked or returned early). The job state has transitioned back
    /// to `Idle` so the caller can start a fresh job if desired.
    Disconnected,
}

impl<T> JobState<T> {
    /// Construct a freshly started job.
    pub fn running(rx: Receiver<T>) -> Self {
        Self::Running(rx)
    }

    /// True when `Running` — i.e. a job is in flight and the caller should
    /// keep polling.
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running(_))
    }

    /// Non-blocking poll. Returns the appropriate `PollOutcome` and
    /// transitions the state to `Idle` whenever a terminal outcome
    /// (`Ready` or `Disconnected`) is observed.
    pub fn poll(&mut self) -> PollOutcome<T> {
        let outcome = match self {
            Self::Idle => return PollOutcome::Pending,
            Self::Running(rx) => match rx.try_recv() {
                Ok(value) => PollOutcome::Ready(value),
                Err(TryRecvError::Empty) => PollOutcome::Pending,
                Err(TryRecvError::Disconnected) => PollOutcome::Disconnected,
            },
        };
        // Reset to Idle on any terminal outcome. `Pending` keeps `Running`.
        if !matches!(outcome, PollOutcome::Pending) {
            *self = Self::Idle;
        }
        outcome
    }
}

