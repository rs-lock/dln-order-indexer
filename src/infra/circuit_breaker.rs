use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct CircuitBreaker {
    state: Mutex<State>,
    failure_threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Mutex::new(State::Closed { failures: 0u32 }),
            failure_threshold,
            timeout,
        }
    }

    pub fn allow_request(&self) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match &mut *state {
            State::Closed { failures: _ } => true,
            State::Open { opened_at } => {
                if opened_at.elapsed() >= self.timeout {
                    *state = State::HalfOpen;
                    true
                } else {
                    false
                }
            }
            State::HalfOpen => true,
        }
    }

    pub fn handle_failure(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match &mut *state {
            State::Closed { failures } => {
                *failures += 1;
                if *failures >= self.failure_threshold {
                    *state = State::Open {
                        opened_at: Instant::now(),
                    };
                }
            }
            State::Open { opened_at: _ } => {}
            State::HalfOpen => {
                *state = State::Open {
                    opened_at: Instant::now(),
                };
            }
        }
    }

    pub fn handle_success(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        match &mut *state {
            State::Closed { failures } => *failures = 0,
            State::Open { opened_at: _ } => {}
            State::HalfOpen => *state = State::Closed { failures: 0 },
        }
    }
}
enum State {
    Closed { failures: u32 },
    Open { opened_at: Instant },
    HalfOpen,
}

//TODO TESTS
