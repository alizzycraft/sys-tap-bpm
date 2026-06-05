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
    pub display_bpm: Option<u16>,
    pub tap_count: usize,
    pub is_active: bool,
}

#[derive(Default)]
pub struct TapTempoSession {
    taps: Vec<Instant>,
    bpm: Option<f64>,
    display_bpm: Option<u16>,
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
        self.display_bpm = stabilize_display_bpm(self.display_bpm, calculate_bpm(&self.taps, 1));
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
            display_bpm: self.display_bpm,
            tap_count: self.taps.len(),
            is_active: self.is_active,
        }
    }

    fn clear(&mut self) {
        self.taps.clear();
        self.bpm = None;
        self.display_bpm = None;
        self.is_active = false;
    }

    fn prune_stale_taps(&mut self, now: Instant, averaging_window: Duration) {
        self.taps
            .retain(|tap| now.duration_since(*tap) <= averaging_window);
    }
}

fn stabilize_display_bpm(previous_display_bpm: Option<u16>, raw_bpm: Option<f64>) -> Option<u16> {
    let Some(raw_bpm) = raw_bpm else {
        return previous_display_bpm.or(Some(0));
    };
    let rounded_bpm = raw_bpm.round().clamp(0.0, u16::MAX as f64) as u16;

    let Some(previous_display_bpm) = previous_display_bpm else {
        return Some(rounded_bpm);
    };

    if rounded_bpm == previous_display_bpm {
        return Some(previous_display_bpm);
    }

    let previous = previous_display_bpm as f64;

    if (raw_bpm - previous).abs() >= 1.0 {
        Some(rounded_bpm)
    } else {
        Some(previous_display_bpm)
    }
}

fn calculate_bpm(taps: &[Instant], min_intervals: usize) -> Option<f64> {
    if taps.len() <= min_intervals {
        return None;
    }

    let interval_count = taps.len() - 1;

    if interval_count < min_intervals {
        return None;
    }

    let first_tap = taps[0];
    let samples: Vec<(f64, f64)> = taps
        .iter()
        .enumerate()
        .map(|(index, tap)| {
            (
                index as f64,
                tap.duration_since(first_tap).as_secs_f64() * 1_000.0,
            )
        })
        .collect();

    let sample_count = samples.len() as f64;
    let mean_index = samples.iter().map(|(index, _)| index).sum::<f64>() / sample_count;
    let mean_ms = samples.iter().map(|(_, millis)| millis).sum::<f64>() / sample_count;

    let (numerator, denominator) =
        samples
            .iter()
            .fold((0.0, 0.0), |(numerator, denominator), (index, millis)| {
                let index_delta = index - mean_index;
                let millis_delta = millis - mean_ms;

                (
                    numerator + index_delta * millis_delta,
                    denominator + index_delta * index_delta,
                )
            });

    if denominator <= f64::EPSILON {
        return None;
    }

    let beat_period_ms = numerator / denominator;

    if beat_period_ms <= 0.0 {
        return None;
    }

    Some(60_000.0 / beat_period_ms)
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
        assert_eq!(snapshot.display_bpm, Some(0));
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
        assert_eq!(snapshot.display_bpm, Some(120));
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
    fn estimates_tempo_line_from_changing_intervals() {
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
    fn uses_fractional_milliseconds_for_more_precise_estimates() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(base + Duration::from_micros(342_857), &config);
        let snapshot = session.tap(base + Duration::from_micros(685_714), &config);

        assert_bpm(snapshot.bpm, 175.000175000175);
    }

    #[test]
    fn regression_estimator_reduces_jitter_from_individual_intervals() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        for millis in [0, 350, 680, 1_040, 1_370, 1_710, 2_060, 2_390, 2_740] {
            session.tap(at(base, millis), &config);
        }

        assert_bpm(session.snapshot().bpm, 175.52413456850317);
    }

    #[test]
    fn display_bpm_uses_hysteresis_to_avoid_single_digit_flicker() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(base + Duration::from_micros(342_857), &config);
        let snapshot = session.tap(base + Duration::from_micros(685_714), &config);

        assert_eq!(snapshot.display_bpm, Some(175));

        session.tap(base + Duration::from_micros(1_027_000), &config);
        let snapshot = session.tap(base + Duration::from_micros(1_364_000), &config);

        assert!(
            snapshot.bpm.expect("expected bpm") > 175.5,
            "expected raw bpm near the rounding boundary"
        );
        assert_eq!(snapshot.display_bpm, Some(175));
    }

    #[test]
    fn display_bpm_updates_when_estimate_moves_past_hysteresis() {
        let mut session = TapTempoSession::default();
        let base = Instant::now();
        let config = config();

        session.tap(at(base, 0), &config);
        session.tap(base + Duration::from_micros(342_857), &config);
        session.tap(base + Duration::from_micros(685_714), &config);
        let snapshot = session.tap(base + Duration::from_micros(1_020_000), &config);

        assert_eq!(snapshot.display_bpm, Some(176));
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
