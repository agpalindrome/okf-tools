//! Structure of a `log.md` reserved file (§9): its date headings must be ISO
//! 8601 `YYYY-MM-DD` (an explicit MUST) and are meant to run newest-first. This
//! module reads only the headings; entry resolution is the bundle's job.

/// What a log's date headings violate (§9): the `## ` headings that are not
/// ISO-8601, and whether the ISO ones run newest-first.
pub(crate) struct HeadingCheck {
    /// The `## ` heading texts that are not `YYYY-MM-DD`.
    pub non_iso: Vec<String>,
    /// Whether the valid dates are out of newest-first (descending) order.
    pub out_of_order: bool,
}

/// Read a log's `## ` date headings, skipping fenced code. ISO dates sort
/// lexicographically, so newest-first is a plain descending check.
pub(crate) fn check_headings(body: &str) -> HeadingCheck {
    let mut non_iso = Vec::new();
    let mut dates: Vec<&str> = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        let line = line.trim_start();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence {
            if let Some(heading) = line.strip_prefix("## ") {
                let heading = heading.trim();
                if is_iso_date(heading) {
                    dates.push(heading);
                } else {
                    non_iso.push(heading.to_string());
                }
            }
        }
    }
    HeadingCheck {
        non_iso,
        out_of_order: dates.windows(2).any(|pair| pair[0] < pair[1]),
    }
}

/// A `YYYY-MM-DD` date: ten characters, hyphens in place, digits elsewhere, and
/// a plausible month and day. Full calendar validity (no Feb 30) is more than
/// §9 needs.
fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: &[u8]| range.iter().all(u8::is_ascii_digit);
    if !digits(&bytes[0..4]) || !digits(&bytes[5..7]) || !digits(&bytes[8..10]) {
        return false;
    }
    let month = text[5..7].parse::<u8>().unwrap_or(0);
    let day = text[8..10].parse::<u8>().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newest_first_iso_log_is_clean() {
        let body = "# Log\n\n## 2026-05-22\n\n- update\n\n## 2026-05-15\n\n- init\n";
        let check = check_headings(body);
        assert!(check.non_iso.is_empty());
        assert!(!check.out_of_order);
    }

    #[test]
    fn a_non_iso_heading_is_flagged() {
        let check = check_headings("# Log\n\n## May 22, 2026\n\n- update\n");
        assert_eq!(check.non_iso, ["May 22, 2026"]);
        assert!(!check.out_of_order);
    }

    #[test]
    fn ascending_dates_are_out_of_order() {
        let check = check_headings("# Log\n\n## 2026-05-15\n\n## 2026-05-22\n");
        assert!(check.non_iso.is_empty());
        assert!(check.out_of_order);
    }

    #[test]
    fn a_date_inside_a_fence_is_not_a_heading() {
        let check = check_headings("# Log\n\n```\n## not-a-date\n```\n");
        assert!(check.non_iso.is_empty());
        assert!(!check.out_of_order);
    }
}
