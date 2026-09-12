use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Hidden,
    Showing,
    Shown,
    Hiding,
}

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Hidden,
    Showing {
        start: Instant,
        start_y: f32,
        target_y: f32,
        duration: Duration,
    },
    Shown {
        shown_at: Instant,
        auto_hide_duration: Duration,
        y: f32,
    },
    Hiding {
        start: Instant,
        start_y: f32,
        target_y: f32,
        duration: Duration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationConfig {
    pub hidden_y: f32,
    pub resting_y: f32,
    pub slide_duration: Duration,
    pub auto_hide_duration: Duration,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            hidden_y: 256.0,
            resting_y: 32.0,
            slide_duration: Duration::from_millis(250),
            auto_hide_duration: Duration::from_secs(5),
        }
    }
}

pub struct OverlayStateMachine {
    state: State,
    config: AnimationConfig,
}

impl OverlayStateMachine {
    pub fn new(config: AnimationConfig) -> Self {
        Self {
            state: State::Hidden,
            config,
        }
    }

    pub fn state_kind(&self) -> StateKind {
        match self.state {
            State::Hidden => StateKind::Hidden,
            State::Showing { .. } => StateKind::Showing,
            State::Shown { .. } => StateKind::Shown,
            State::Hiding { .. } => StateKind::Hiding,
        }
    }

    pub fn is_animating(&self) -> bool {
        matches!(self.state, State::Showing { .. } | State::Hiding { .. })
    }

    pub fn is_hidden(&self) -> bool {
        matches!(self.state, State::Hidden)
    }

    pub fn current_y(&self, now: Instant) -> f32 {
        match self.state {
            State::Hidden => self.config.hidden_y,
            State::Shown { y, .. } => y,
            State::Showing {
                start,
                start_y,
                target_y,
                duration,
            } => {
                let elapsed = now.saturating_duration_since(start).as_secs_f32();
                let total = duration.as_secs_f32();
                let progress = (elapsed / total).clamp(0.0, 1.0);
                // Cubic ease-out: 1 - (1 - t)^3
                let eased = 1.0 - (1.0 - progress).powi(3);
                start_y + (target_y - start_y) * eased
            }
            State::Hiding {
                start,
                start_y,
                target_y,
                duration,
            } => {
                let elapsed = now.saturating_duration_since(start).as_secs_f32();
                let total = duration.as_secs_f32();
                let progress = (elapsed / total).clamp(0.0, 1.0);
                // Cubic ease-in: t^3
                let eased = progress.powi(3);
                start_y + (target_y - start_y) * eased
            }
        }
    }

    pub fn show(&mut self, now: Instant) {
        match self.state {
            State::Hidden => {
                self.state = State::Showing {
                    start: now,
                    start_y: self.config.hidden_y,
                    target_y: self.config.resting_y,
                    duration: self.config.slide_duration,
                };
            }
            State::Showing { .. } => {
                // Already showing, no-op
            }
            State::Shown { ref mut shown_at, .. } => {
                // Reset auto-hide countdown
                *shown_at = now;
            }
            State::Hiding { .. } => {
                // Reverse smoothly from current position
                let current_y = self.current_y(now);
                let distance = (current_y - self.config.resting_y).abs();
                let total_distance = (self.config.hidden_y - self.config.resting_y).abs().max(1.0);
                let ratio = distance / total_distance;
                let duration = self.config.slide_duration.mul_f32(ratio.clamp(0.05, 1.0));

                self.state = State::Showing {
                    start: now,
                    start_y: current_y,
                    target_y: self.config.resting_y,
                    duration,
                };
            }
        }
    }

    pub fn hide(&mut self, now: Instant) {
        match self.state {
            State::Hidden => {}
            State::Showing { .. } => {
                // Reverse smoothly from current position
                let current_y = self.current_y(now);
                let distance = (self.config.hidden_y - current_y).abs();
                let total_distance = (self.config.hidden_y - self.config.resting_y).abs().max(1.0);
                let ratio = distance / total_distance;
                let duration = self.config.slide_duration.mul_f32(ratio.clamp(0.05, 1.0));

                self.state = State::Hiding {
                    start: now,
                    start_y: current_y,
                    target_y: self.config.hidden_y,
                    duration,
                };
            }
            State::Shown { y, .. } => {
                self.state = State::Hiding {
                    start: now,
                    start_y: y,
                    target_y: self.config.hidden_y,
                    duration: self.config.slide_duration,
                };
            }
            State::Hiding { .. } => {}
        }
    }

    pub fn toggle(&mut self, now: Instant) {
        match self.state {
            State::Hidden | State::Hiding { .. } => self.show(now),
            State::Showing { .. } | State::Shown { .. } => self.hide(now),
        }
    }

    /// Advance time / tick the state machine.
    /// Returns true if the state changed or an animation frame should be drawn.
    pub fn update(&mut self, now: Instant) -> bool {
        match self.state {
            State::Hidden => false,
            State::Showing {
                start,
                target_y,
                duration,
                ..
            } => {
                if now.saturating_duration_since(start) >= duration {
                    self.state = State::Shown {
                        shown_at: now,
                        auto_hide_duration: self.config.auto_hide_duration,
                        y: target_y,
                    };
                }
                true
            }
            State::Shown {
                shown_at,
                auto_hide_duration,
                y,
            } => {
                if now.saturating_duration_since(shown_at) >= auto_hide_duration {
                    self.state = State::Hiding {
                        start: now,
                        start_y: y,
                        target_y: self.config.hidden_y,
                        duration: self.config.slide_duration,
                    };
                    true
                } else {
                    false
                }
            }
            State::Hiding {
                start, duration, ..
            } => {
                if now.saturating_duration_since(start) >= duration {
                    self.state = State::Hidden;
                }
                true
            }
        }
    }

    /// Returns the deadline when the next event or timeout occurs (e.g. auto-hide),
    /// useful for setting event loop dispatch timeout.
    pub fn next_deadline(&self) -> Option<Instant> {
        match self.state {
            State::Hidden => None,
            State::Showing { start, duration, .. } => Some(start + duration),
            State::Shown {
                shown_at,
                auto_hide_duration,
                ..
            } => Some(shown_at + auto_hide_duration),
            State::Hiding { start, duration, .. } => Some(start + duration),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_hidden() {
        let sm = OverlayStateMachine::new(AnimationConfig::default());
        assert_eq!(sm.state_kind(), StateKind::Hidden);
        assert!(sm.is_hidden());
        assert!(!sm.is_animating());
        assert_eq!(sm.next_deadline(), None);
    }

    #[test]
    fn test_show_and_complete_animation() {
        let config = AnimationConfig {
            hidden_y: 200.0,
            resting_y: 50.0,
            slide_duration: Duration::from_millis(200),
            auto_hide_duration: Duration::from_secs(3),
        };
        let mut sm = OverlayStateMachine::new(config);
        let t0 = Instant::now();

        sm.show(t0);
        assert_eq!(sm.state_kind(), StateKind::Showing);
        assert!(sm.is_animating());
        assert!(!sm.is_hidden());

        // At t0 + 100ms (halfway)
        let t1 = t0 + Duration::from_millis(100);
        let changed = sm.update(t1);
        assert!(changed);
        assert_eq!(sm.state_kind(), StateKind::Showing);
        let y_mid = sm.current_y(t1);
        assert!(y_mid < 200.0 && y_mid > 50.0, "y_mid was {}", y_mid);

        // At t0 + 200ms (animation complete)
        let t2 = t0 + Duration::from_millis(200);
        let changed = sm.update(t2);
        assert!(changed);
        assert_eq!(sm.state_kind(), StateKind::Shown);
        assert_eq!(sm.current_y(t2), 50.0);
        assert!(!sm.is_animating());
    }

    #[test]
    fn test_auto_hide_after_duration() {
        let config = AnimationConfig {
            hidden_y: 200.0,
            resting_y: 50.0,
            slide_duration: Duration::from_millis(100),
            auto_hide_duration: Duration::from_secs(2),
        };
        let mut sm = OverlayStateMachine::new(config);
        let t0 = Instant::now();

        sm.show(t0);
        let t1 = t0 + Duration::from_millis(100);
        sm.update(t1);
        assert_eq!(sm.state_kind(), StateKind::Shown);

        // 1 second later: still shown
        let t2 = t1 + Duration::from_secs(1);
        let changed = sm.update(t2);
        assert!(!changed);
        assert_eq!(sm.state_kind(), StateKind::Shown);

        // 2 seconds later (t1 + 2s): auto-hide triggers
        let t3 = t1 + Duration::from_secs(2);
        let changed = sm.update(t3);
        assert!(changed);
        assert_eq!(sm.state_kind(), StateKind::Hiding);

        // Finish hiding
        let t4 = t3 + Duration::from_millis(100);
        sm.update(t4);
        assert_eq!(sm.state_kind(), StateKind::Hidden);
        assert!(sm.is_hidden());
    }

    #[test]
    fn test_explicit_hide_command() {
        let mut sm = OverlayStateMachine::new(AnimationConfig::default());
        let t0 = Instant::now();

        sm.show(t0);
        sm.update(t0 + Duration::from_millis(250));
        assert_eq!(sm.state_kind(), StateKind::Shown);

        // Explicit hide
        let t1 = t0 + Duration::from_secs(1);
        sm.hide(t1);
        assert_eq!(sm.state_kind(), StateKind::Hiding);

        // Finish hiding
        let t2 = t1 + Duration::from_millis(250);
        sm.update(t2);
        assert_eq!(sm.state_kind(), StateKind::Hidden);
    }

    #[test]
    fn test_show_while_shown_resets_auto_hide() {
        let config = AnimationConfig {
            hidden_y: 200.0,
            resting_y: 50.0,
            slide_duration: Duration::from_millis(100),
            auto_hide_duration: Duration::from_secs(3),
        };
        let mut sm = OverlayStateMachine::new(config);
        let t0 = Instant::now();

        sm.show(t0);
        let t1 = t0 + Duration::from_millis(100);
        sm.update(t1);
        assert_eq!(sm.state_kind(), StateKind::Shown);

        // At t1 + 2s, send show again
        let t2 = t1 + Duration::from_secs(2);
        sm.show(t2);
        assert_eq!(sm.state_kind(), StateKind::Shown);

        // At t1 + 3.5s (1.5s after second show): should still be Shown
        let t3 = t1 + Duration::from_millis(3500);
        sm.update(t3);
        assert_eq!(sm.state_kind(), StateKind::Shown);

        // At t2 + 3s: auto-hide triggers
        let t4 = t2 + Duration::from_secs(3);
        sm.update(t4);
        assert_eq!(sm.state_kind(), StateKind::Hiding);
    }

    #[test]
    fn test_interrupted_slide_in_by_hide() {
        let mut sm = OverlayStateMachine::new(AnimationConfig::default());
        let t0 = Instant::now();
        sm.show(t0);
        assert_eq!(sm.state_kind(), StateKind::Showing);

        // Halfway through show, command hide
        let t1 = t0 + Duration::from_millis(125);
        sm.hide(t1);
        assert_eq!(sm.state_kind(), StateKind::Hiding);
    }

    #[test]
    fn test_interrupted_slide_out_by_show() {
        let mut sm = OverlayStateMachine::new(AnimationConfig::default());
        let t0 = Instant::now();
        sm.show(t0);
        sm.update(t0 + Duration::from_millis(250));
        sm.hide(t0 + Duration::from_millis(500));
        assert_eq!(sm.state_kind(), StateKind::Hiding);

        // Halfway through hide, command show
        let t1 = t0 + Duration::from_millis(625);
        sm.show(t1);
        assert_eq!(sm.state_kind(), StateKind::Showing);
    }
}
