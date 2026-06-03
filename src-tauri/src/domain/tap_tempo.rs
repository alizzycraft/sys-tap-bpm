use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct TapTempoConfig {
    pub reset_after: Duration,
    pub averaging_window: Duration,
    pub min_intervals: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapTempoSnapshot {
    pub bpm: Option<f64>,
    pub tap_count: usize,
    pub is_active: bool,
}

#[derive(Default)]
pub struct TapTempoSession {
    taps: Vec<Instant>,
    bpm: Option<f64>,
    is_active: bool,
}

impl TapTempoSession {
    pub fn tap(&mut self, now: Instant, config: &TapTempoConfig) -> TapTempoSnapshot {
        if self
            .taps
            .last()
            .is_some_and(|last_tap| now.duration_since(*last_tap) > config.reset_after)
        {
            self.clear();
        }

        self.taps.push(now);
        self.prune_stale_taps(now, config.averaging_window);
        self.bpm = calculate_bpm(&self.taps, config.min_intervals);
        self.is_active = true;

        self.snapshot()
    }

    pub fn reset_if_inactive(
        &mut self,
        now: Instant,
        reset_after: Duration,
    ) -> Option<TapTempoSnapshot> {
        let should_reset = self
            .taps
            .last()
            .is_some_and(|last_tap| now.duration_since(*last_tap) >= reset_after);

        if !should_reset {
            return None;
        }

        self.clear();
        Some(self.snapshot())
    }

    pub fn snapshot(&self) -> TapTempoSnapshot {
        TapTempoSnapshot {
            bpm: self.bpm,
            tap_count: self.taps.len(),
            is_active: self.is_active,
        }
    }

    fn clear(&mut self) {
        self.taps.clear();
        self.bpm = None;
        self.is_active = false;
    }

    fn prune_stale_taps(&mut self, now: Instant, averaging_window: Duration) {
        self.taps
            .retain(|tap| now.duration_since(*tap) <= averaging_window);
    }
}

fn calculate_bpm(taps: &[Instant], min_intervals: usize) -> Option<f64> {
    if taps.len() <= min_intervals {
        return None;
    }

    let intervals: Vec<f64> = taps
        .windows(2)
        .map(|pair| pair[1].duration_since(pair[0]).as_millis() as f64)
        .filter(|interval| *interval > 0.0)
        .collect();

    if intervals.len() < min_intervals {
        return None;
    }

    let average_ms = intervals.iter().sum::<f64>() / intervals.len() as f64;
    Some(60_000.0 / average_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> TapTempoConfig {
        TapTempoConfig {
            reset_after: Duration::from_secs(3),
            averaging_window: Duration::from_secs(45),
            min_intervals: 2,
        }
    }

    fn at(base: Instant, millis: u64) -> Instant {
        base + Duration::from_millis(millis)
    }

    fn assert_bpm(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("expected bpm");
        assert!(
            (actual - expected).abs() < 0.001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn first_tap_starts_session_without_bpm() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();

        let snapshot = session.tap(base, &config());

        assert_eq!(snapshot.tap_count, 1);
        assert!(snapshot.is_active);
        assert_eq!(snapshot.bpm, None);
    }

    #[test]
    fn waits_for_minimum_intervals_before_showing_bpm() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        let snapshot = session.tap(at(base, 500), &config);

        assert_eq!(snapshot.tap_count, 2);
        assert_eq!(snapshot.bpm, None);
    }

    #[test]
    fn averages_intervals_before_deriving_bpm() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(at(base, 500), &config);
        let snapshot = session.tap(at(base, 1_000), &config);

        assert_eq!(snapshot.tap_count, 3);
        assert_bpm(snapshot.bpm, 120.0);
    }

    #[test]
    fn averages_changing_tempo_intervals() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(at(base, 500), &config);
        session.tap(at(base, 1_100), &config);
        let snapshot = session.tap(at(base, 1_800), &config);

        assert_bpm(snapshot.bpm, 100.0);
    }

    #[test]
    fn resets_session_when_next_tap_arrives_after_inactivity() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(at(base, 500), &config);
        session.tap(at(base, 1_000), &config);
        let snapshot = session.tap(at(base, 4_500), &config);

        assert_eq!(snapshot.tap_count, 1);
        assert!(snapshot.is_active);
        assert_eq!(snapshot.bpm, None);
    }

    #[test]
    fn reset_if_inactive_clears_active_session() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(at(base, 500), &config);
        session.tap(at(base, 1_000), &config);

        assert!(session
            .reset_if_inactive(at(base, 2_000), config.reset_after)
            .is_none());

        let snapshot = session
            .reset_if_inactive(at(base, 4_000), config.reset_after)
            .expect("expected reset");

        assert_eq!(snapshot.tap_count, 0);
        assert!(!snapshot.is_active);
        assert_eq!(snapshot.bpm, None);
    }

    #[test]
    fn prunes_taps_outside_averaging_window() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = TapTempoConfig {
            reset_after: Duration::from_secs(60),
            averaging_window: Duration::from_millis(1_000),
            min_intervals: 2,
        };

        session.tap(at(base, 0), &config);
        session.tap(at(base, 500), &config);
        session.tap(at(base, 1_000), &config);
        let snapshot = session.tap(at(base, 1_500), &config);

        assert_eq!(snapshot.tap_count, 3);
        assert_bpm(snapshot.bpm, 120.0);
    }
}
