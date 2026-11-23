use std::time::Duration;

pub fn clock(d: Duration) -> String {
    let total = d.as_secs();
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_minutes_and_hours() {
        assert_eq!(clock(Duration::ZERO), "0:00");
        assert_eq!(clock(Duration::from_millis(65_900)), "1:05");
        assert_eq!(clock(Duration::from_secs(3 * 3600 + 7 * 60 + 9)), "3:07:09");
    }
}
