//! An instant, parsed from the RFC 3339 profile of ISO 8601.
//!
//! §5.2 types `generated.at` and `verified[].at` as "an ISO 8601 datetime",
//! which names a family of formats rather than one. Its week date is the member
//! that bites: `2026-W01-1T00:00:00Z` denotes 2025-12-29, matches the calendar
//! form's length and separator positions, and sorts after every calendar date
//! because `W` exceeds every digit — so comparing the two fields as strings is
//! inverted for input the spec permits. okf-graph narrows to RFC 3339, the
//! profile every §5.2 example already uses; `docs/okf-friction.md` records the
//! narrowing as a decision the spec text has not made.

/// A point in time, read from an RFC 3339 `date-time`.
///
/// Ordering and equality are by the instant, not by the text: an offset is
/// normalized away, so `2026-01-01T00:00:00Z` and `2026-01-01T02:00:00+02:00`
/// compare equal. That is what makes the type worth having — comparing the
/// documents' own strings is the trap it exists to close.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    /// Seconds from 1970-01-01T00:00:00Z, ignoring leap seconds. A `:60` leap
    /// second therefore lands on the following minute and compares equal to it,
    /// which is the standard trade: no table of leap seconds to keep current,
    /// and the ordering a consumer needs is unaffected.
    seconds: i64,
    /// Fractional seconds, truncated to nanoseconds. Digits beyond the ninth are
    /// dropped rather than rounded, so two timestamps that differ only there
    /// compare equal.
    nanoseconds: u32,
}

impl Timestamp {
    /// Read an RFC 3339 `date-time`, or `None` when `text` is not one.
    ///
    /// `T` and `Z` are accepted in either case, because RFC 3339's ABNF literals
    /// are case-insensitive. The space separator its §5.6 note allows "by mutual
    /// agreement" is rejected: OKF makes no such agreement, and accepting it
    /// would admit a form no §5.2 example uses.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() < 20 {
            return None;
        }
        if bytes[4] != b'-' || bytes[7] != b'-' || bytes[13] != b':' || bytes[16] != b':' {
            return None;
        }
        if !matches!(bytes[10], b'T' | b't') {
            return None;
        }

        let year = digits(text.get(0..4)?)?;
        let month = digits(text.get(5..7)?)?;
        let day = digits(text.get(8..10)?)?;
        let hour = digits(text.get(11..13)?)?;
        let minute = digits(text.get(14..16)?)?;
        let second = digits(text.get(17..19)?)?;

        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        // 23:59:60 is a legal leap second (RFC 3339 §5.6); 61 is not.
        if hour > 23 || minute > 59 || second > 60 {
            return None;
        }

        let (nanoseconds, offset_text) = match text.get(19..)? {
            fraction if fraction.starts_with('.') => {
                let taken = fraction[1..].bytes().take_while(u8::is_ascii_digit).count();
                if taken == 0 {
                    return None;
                }
                (
                    nanoseconds_from(&fraction[1..=taken]),
                    &fraction[1 + taken..],
                )
            }
            rest => (0, rest),
        };
        let offset = offset_seconds(offset_text)?;

        Some(Timestamp {
            seconds: days_from_civil(year, month, day) * 86_400
                + hour * 3_600
                + minute * 60
                + second
                - offset,
            nanoseconds,
        })
    }
}

/// The value of a run of ASCII digits, or `None` for anything else. Rejects the
/// signs and whitespace `str::parse` would accept, which is what keeps `+1` and
/// ` 12` out of a field the grammar says is `2DIGIT`.
fn digits(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// The first nine fractional digits as nanoseconds, zero-padded on the right.
fn nanoseconds_from(fraction: &str) -> u32 {
    let mut nanoseconds = 0;
    for position in 0..9 {
        let digit = fraction
            .as_bytes()
            .get(position)
            .map_or(0, |byte| byte - b'0');
        nanoseconds = nanoseconds * 10 + u32::from(digit);
    }
    nanoseconds
}

/// The `time-offset` in seconds: `Z` for UTC, or `±hh:mm`.
fn offset_seconds(text: &str) -> Option<i64> {
    if matches!(text, "Z" | "z") {
        return Some(0);
    }
    let bytes = text.as_bytes();
    if bytes.len() != 6 || bytes[3] != b':' {
        return None;
    }
    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours = digits(text.get(1..3)?)?;
    let minutes = digits(text.get(4..6)?)?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(sign * (hours * 3_600 + minutes * 60))
}

/// Days from 1970-01-01 to a proleptic-Gregorian civil date, by Hinnant's
/// algorithm. The negative-year branch cannot fire on a four-digit year, and is
/// kept so the algorithm reads as published.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Days in a month of a proleptic-Gregorian year.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The anchors come from `date -u --date=<text> +%s` (GNU coreutils 9.11,
    /// 2026-08-08), so the civil-date arithmetic is checked against something
    /// other than itself.
    #[test]
    fn the_epoch_and_two_later_instants_land_where_date_puts_them() {
        assert_eq!(Timestamp::parse("1970-01-01T00:00:00Z").unwrap().seconds, 0);
        assert_eq!(
            Timestamp::parse("2026-06-20T22:53:05Z").unwrap().seconds,
            1_781_995_985
        );
        assert_eq!(
            Timestamp::parse("2024-02-29T12:00:00Z").unwrap().seconds,
            1_709_208_000
        );
    }

    #[test]
    fn an_offset_is_normalized_away_so_one_instant_has_one_value() {
        let utc = Timestamp::parse("2026-01-01T00:00:00Z").unwrap();
        assert_eq!(Timestamp::parse("2026-01-01T02:00:00+02:00").unwrap(), utc);
        assert_eq!(Timestamp::parse("2025-12-31T23:00:00-01:00").unwrap(), utc);
        assert_eq!(Timestamp::parse("2026-01-01T00:00:00-00:00").unwrap(), utc);
    }

    /// The failure the rule exists for: a week date and a calendar date that
    /// compare the wrong way round as strings.
    #[test]
    fn a_week_date_is_rejected_rather_than_ordered_wrongly() {
        assert!("2026-W01-1T00:00:00Z" > "2026-01-01T00:00:00Z");
        assert!(Timestamp::parse("2026-W01-1T00:00:00Z").is_none());
    }

    #[test]
    fn ordering_follows_the_instant_not_the_text() {
        let older = Timestamp::parse("2025-12-29T00:00:00Z").unwrap();
        let newer = Timestamp::parse("2026-01-01T00:00:00Z").unwrap();
        assert!(older < newer);
        assert!(Timestamp::parse("2026-01-01T00:00:00.5Z").unwrap() > newer);
    }

    #[test]
    fn the_case_insensitive_literals_parse() {
        let upper = Timestamp::parse("2026-06-20T22:53:05Z").unwrap();
        assert_eq!(Timestamp::parse("2026-06-20t22:53:05z").unwrap(), upper);
    }

    #[test]
    fn a_fraction_is_read_to_nanoseconds_and_truncated_there() {
        assert_eq!(
            Timestamp::parse("2026-06-20T22:53:05.5Z")
                .unwrap()
                .nanoseconds,
            500_000_000
        );
        assert_eq!(
            Timestamp::parse("2026-06-20T22:53:05.000000001Z")
                .unwrap()
                .nanoseconds,
            1
        );
        assert_eq!(
            Timestamp::parse("2026-06-20T22:53:05.0000000019Z")
                .unwrap()
                .nanoseconds,
            1
        );
    }

    #[test]
    fn a_leap_second_parses_and_lands_on_the_following_minute() {
        assert_eq!(
            Timestamp::parse("2016-12-31T23:59:60Z").unwrap(),
            Timestamp::parse("2017-01-01T00:00:00Z").unwrap()
        );
        assert!(Timestamp::parse("2016-12-31T23:59:61Z").is_none());
    }

    #[test]
    fn a_date_that_no_calendar_has_is_rejected() {
        assert!(Timestamp::parse("2026-02-30T00:00:00Z").is_none());
        assert!(Timestamp::parse("2023-02-29T00:00:00Z").is_none());
        assert!(Timestamp::parse("2100-02-29T00:00:00Z").is_none());
        assert!(Timestamp::parse("2026-13-01T00:00:00Z").is_none());
        assert!(Timestamp::parse("2026-00-01T00:00:00Z").is_none());
        assert!(Timestamp::parse("2026-01-00T00:00:00Z").is_none());
        assert!(Timestamp::parse("2026-06-20T24:00:00Z").is_none());
    }

    /// Every one of these is ISO 8601 or nearly so, which is the point: the
    /// checker's job is to reject what §5.2's own examples never use.
    #[test]
    fn the_forms_rfc_3339_excludes_are_rejected() {
        for text in [
            "2026-06-20",                // date only
            "20260620T225305Z",          // ISO 8601 basic format
            "2026-06-20T22:53:05",       // no offset
            "2026-06-20 22:53:05Z",      // the space separator §5.6 only allows by agreement
            "2026-06-20T22:53:05+2:00",  // one-digit offset hour
            "2026-06-20T22:53:05+0200",  // offset without the colon
            "2026-06-20T22:53:05.Z",     // a fraction with no digits
            "2026-06-20T22:53:05Z ",     // trailing space
            "2026-06-20T22:53:05Z0",     // trailing junk
            "+026-06-20T22:53:05Z",      // a sign where a digit belongs
            "2026-06-20T22:53:05+24:00", // offset hour out of range
            "2026-06-20T22:53:05+02:60", // offset minute out of range
            "",
        ] {
            assert!(Timestamp::parse(text).is_none(), "{text} should not parse");
        }
    }
}
