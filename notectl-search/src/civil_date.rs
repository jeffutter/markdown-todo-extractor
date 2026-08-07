use chrono::Datelike;

/// Calendar date conversions using chrono.
///
/// Wraps chrono's well-tested date arithmetic for Unix epoch ↔ civil date
/// conversion, eliminating the risk of hand-rolled leap-year bugs.
///
/// Days from chrono's CE epoch (0000-01-01) to the Unix epoch (1970-01-01).
const UNIX_EPOCH_DAYS_CE: i32 = 719_163;

/// Convert days since Unix epoch (1970-01-01) to a civil date (year, month, day).
///
/// `n` is signed days relative to 1970-01-01 (negative = before epoch).
pub(crate) fn civil_from_days(n: i64) -> (i32, u32, u32) {
    let dt = chrono::NaiveDate::from_num_days_from_ce_opt(UNIX_EPOCH_DAYS_CE + n as i32)
        .expect("date out of range");
    (dt.year(), dt.month(), dt.day())
}

/// Convert a civil date to days since Unix epoch (1970-01-01).
///
/// Returns signed days: positive for dates after 1970-01-01, negative for
/// dates before.
pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let dt = chrono::NaiveDate::from_ymd_opt(year, month, day).expect("invalid calendar date");
    dt.num_days_from_ce() as i64 - UNIX_EPOCH_DAYS_CE as i64
}

/// Alias for clarity: convert a civil date to days since Unix epoch.
#[inline]
pub(crate) fn civil_to_days_since_epoch(year: i32, month: u32, day: u32) -> i64 {
    days_from_civil(year, month, day)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_boundary() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_to_days_since_epoch(1970, 1, 1), 0);
    }

    #[test]
    fn test_y2k_boundary() {
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        assert_eq!(civil_to_days_since_epoch(2000, 1, 1), 10_957);
    }

    #[test]
    fn test_leap_year_feb29() {
        let days = civil_to_days_since_epoch(2000, 2, 29);
        assert_eq!(civil_from_days(days), (2000, 2, 29));
    }

    #[test]
    fn test_century_non_leap() {
        assert_eq!(
            civil_to_days_since_epoch(2100, 3, 1) - civil_to_days_since_epoch(2100, 2, 28),
            1
        );
        assert_eq!(
            civil_from_days(civil_to_days_since_epoch(2100, 3, 1)),
            (2100, 3, 1)
        );
    }

    #[test]
    fn test_pre_epoch_dates() {
        assert_eq!(civil_to_days_since_epoch(1969, 12, 31), -1);
        assert!(civil_to_days_since_epoch(1900, 1, 1) < 0);
    }

    #[test]
    fn test_reciprocal() {
        for n in [0, 10_957, 19_723, 47_540] {
            let (y, m, d) = civil_from_days(n);
            assert_eq!(days_from_civil(y, m, d), n, "Round-trip failed for n={}", n);
        }
    }

    #[test]
    fn test_known_dates() {
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_to_days_since_epoch(2024, 1, 1), 19_723);
    }

    #[test]
    fn test_february_round_trips() {
        for (y, m, d) in [(2000, 2, 28), (2000, 2, 29), (2100, 2, 28), (1996, 2, 29)] {
            let days = days_from_civil(y, m, d);
            assert_eq!(
                civil_from_days(days),
                (y, m, d),
                "Round-trip failed for ({},{},{})",
                y,
                m,
                d
            );
        }
    }

    #[test]
    fn test_march_round_trips() {
        for (y, m, d) in [(2000, 3, 1), (2000, 3, 15), (2100, 3, 1)] {
            let days = days_from_civil(y, m, d);
            assert_eq!(
                civil_from_days(days),
                (y, m, d),
                "Round-trip failed for ({},{},{})",
                y,
                m,
                d
            );
        }
    }

    #[test]
    fn test_day_difference_across_years() {
        let d1 = civil_to_days_since_epoch(1970, 1, 1);
        let d2 = civil_to_days_since_epoch(2000, 1, 1);
        assert_eq!(d2 - d1, 10_957);
        let d3 = civil_to_days_since_epoch(2024, 1, 1);
        assert_eq!(d3 - d1, 19_723);
    }

    #[test]
    fn test_negative_days_for_pre_epoch() {
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(-365), (1969, 1, 1));
    }
}
