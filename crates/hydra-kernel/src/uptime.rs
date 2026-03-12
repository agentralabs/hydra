

use std::time::Instant;

pub fn start() -> Instant {
    Instant::now()
}

pub fn format_uptime(start: Instant) -> String {
    let elapsed = start.elapsed();
    let hours = elapsed.as_secs() / 3600;
    let minutes = (elapsed.as_secs() % 3600) / 60;
    let seconds = elapsed.as_secs() % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

pub fn uptime_secs(start: Instant) -> u64 {
    start.elapsed().as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_uptime_functions() {
        let start_time = start();
        sleep(Duration::new(2, 0)); // Sleep for 2 seconds
        assert_eq!(uptime_secs(start_time), 2);
        let formatted = format_uptime(start_time);
        assert!(formatted.contains("h"));
        assert!(formatted.contains("m"));
        assert!(formatted.contains("s"));
    }
}
