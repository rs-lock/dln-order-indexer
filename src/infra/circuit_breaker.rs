use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use thiserror::Error;

pub struct CircuitBreaker {
    state: Mutex<State>,
    failure_threshold: u32,
    timeout: Duration,
}

#[derive(Error, Debug)]
enum CircuitBreakerError {
    #[error("Circuit breaker is open")]
    Open,
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

#[cfg(test)]
mod tests {
    use std::{thread::sleep, time::Duration};

    use crate::infra::circuit_breaker::CircuitBreaker;

    #[test]
    fn closed_allows_requests() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1)); // Closed
        assert!(cb.allow_request()); // Closed → allows
    }

    #[test]
    fn opens_after_threshold_failures() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(1)); // Closed
        cb.handle_failure(); // Closed (1/3)
        cb.handle_failure(); // Closed (2/3)
        assert!(cb.allow_request()); // Closed → allows
        cb.handle_failure(); // → Open (3/3)
        assert!(!cb.allow_request()); // Open → blocks
    }

    #[test]
    fn from_open_to_halfopen_after_timeout() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(1)); // Closed
        cb.handle_failure(); // Closed (1/2)
        cb.handle_failure(); // → Open (2/2)
        assert!(!cb.allow_request()); // Open → blocks
        sleep(Duration::from_millis(2)); // timeout expires
        assert!(cb.allow_request()); // → HalfOpen → allows
    }

    #[test]
    fn halfopen_to_closed() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(1));
        cb.handle_failure(); // → Open
        assert!(!cb.allow_request()); // Open 
        sleep(Duration::from_millis(2));
        assert!(cb.allow_request()); // HalfOpen
        cb.handle_success(); // → Closed
        assert!(cb.allow_request()); // Closed 
    }

    #[test]
    fn halfopen_to_open() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(1));
        cb.handle_failure(); // → Open
        assert!(!cb.allow_request()); // Open 
        sleep(Duration::from_millis(2));
        assert!(cb.allow_request()); // HalfOpen
        cb.handle_failure(); // → Open
        assert!(!cb.allow_request()); // Open 
    }
}
