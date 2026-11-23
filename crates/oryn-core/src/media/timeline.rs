use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timeline {
    length: Duration,
}

impl Timeline {
    pub fn new(length: Duration) -> Self {
        Self { length }
    }

    pub fn length(&self) -> Duration {
        self.length
    }

    pub fn fraction(&self, position: Duration) -> f32 {
        if self.length.is_zero() {
            return 0.0;
        }
        (position.as_secs_f64() / self.length.as_secs_f64()).clamp(0.0, 1.0) as f32
    }

    pub fn at(&self, fraction: f32) -> Duration {
        self.length.mul_f64(f64::from(fraction.clamp(0.0, 1.0)))
    }

    pub fn step(&self, position: Duration, by: Duration, forward: bool) -> Duration {
        let next = if forward {
            position.saturating_add(by)
        } else {
            position.saturating_sub(by)
        };
        next.min(self.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_positions_to_fractions_and_back() {
        let t = Timeline::new(Duration::from_secs(200));
        assert_eq!(t.fraction(Duration::from_secs(50)), 0.25);
        assert_eq!(t.fraction(Duration::from_secs(500)), 1.0);
        assert_eq!(t.at(0.5), Duration::from_secs(100));
        assert_eq!(t.at(-1.0), Duration::ZERO);
        assert_eq!(t.at(4.0), Duration::from_secs(200));
    }

    #[test]
    fn empty_timeline_stays_at_zero() {
        let t = Timeline::new(Duration::ZERO);
        assert_eq!(t.fraction(Duration::from_secs(3)), 0.0);
        assert_eq!(t.at(0.7), Duration::ZERO);
    }

    #[test]
    fn steps_stay_inside_the_track() {
        let t = Timeline::new(Duration::from_secs(10));
        let five = Duration::from_secs(5);
        assert_eq!(
            t.step(Duration::from_secs(8), five, true),
            Duration::from_secs(10)
        );
        assert_eq!(t.step(Duration::from_secs(3), five, false), Duration::ZERO);
        assert_eq!(
            t.step(Duration::from_secs(6), five, false),
            Duration::from_secs(1)
        );
    }
}
