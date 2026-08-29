use chrono::{DateTime, Duration, Timelike, Utc};

/// Calculates next hourly boundary and time remaining.
pub struct HourlyRotationTrigger {
    last_rotation: DateTime<Utc>,
}

impl HourlyRotationTrigger {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { last_rotation: now }
    }

    /// Calculate the next top-of-the-hour timestamp (e.g. 05:00:00).
    pub fn next_hour_boundary(now: DateTime<Utc>) -> DateTime<Utc> {
        let current_hour = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
        current_hour + Duration::hours(1)
    }

    /// Check if the current time has crossed the top of the hour.
    pub fn should_rotate(&mut self, now: DateTime<Utc>) -> bool {
        let next_boundary = Self::next_hour_boundary(self.last_rotation);
        if now >= next_boundary {
            self.last_rotation = now;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hourly_rotation_boundary() {
        let t0 = Utc::now().with_hour(14).unwrap().with_minute(35).unwrap().with_second(10).unwrap();
        let next = HourlyRotationTrigger::next_hour_boundary(t0);

        assert_eq!(next.hour(), 15);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.second(), 0);

        let mut trigger = HourlyRotationTrigger::new(t0);
        assert!(!trigger.should_rotate(t0 + Duration::minutes(20))); // 14:55 -> false
        assert!(trigger.should_rotate(t0 + Duration::minutes(26)));  // 15:01 -> true
    }
}
