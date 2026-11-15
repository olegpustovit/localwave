//! Swift's Files wording and number formats, prepared on the FilesWorker so frames only borrow
//! finished strings (WaveFilesViews: byteLabel, "%d:%02d", relative scan ages, source stats).

/// "%.1f GB" from one GiB, else "%.1f MB" (Swift byteLabel).
pub(super) fn bytes(count: i64) -> String {
    if count >= 1 << 30 {
        format!("{:.1} GB", count as f64 / f64::from(1 << 30))
    } else {
        megabytes(count)
    }
}

/// "%.1f MB" (a file's Size cell).
pub(super) fn megabytes(count: i64) -> String {
    format!("{:.1} MB", count as f64 / f64::from(1 << 20))
}

/// "%d:%02d" with whole minutes, so an hour reads "75:02" as in Swift.
pub(super) fn duration(seconds: f64) -> String {
    let whole = seconds as i64;
    format!("{}:{:02}", whole / 60, whole % 60)
}

/// "0 kept · 28 on device · 6 in cloud"; the cloud part appears only when nonzero.
pub(super) fn stats(pinned: usize, local: usize, cloud: usize) -> String {
    let mut text = format!("{pinned} kept · {local} on device");
    if cloud > 0 {
        text.push_str(&format!(" · {cloud} in cloud"));
    }
    text
}

/// Foundation's `.relative(presentation: .numeric, unitsStyle: .abbreviated)` in English:
/// "2 min. ago", "1 hr. ago", "3 days ago", "in 5 sec.".
/// shortcut: months and years use 30.44/365.25-day averages, as Foundation's calendar does within a day.
pub(super) fn age(now: f64, then: Option<f64>) -> String {
    let Some(then) = then else {
        return "Not scanned".into();
    };
    let delta = now - then;
    let seconds = delta.abs();
    let days = seconds / 86_400.0;
    let (count, unit) = if seconds.round() < 60.0 {
        (seconds.round(), "sec.")
    } else if (seconds / 60.0).round() < 60.0 {
        ((seconds / 60.0).round(), "min.")
    } else if (seconds / 3600.0).round() < 24.0 {
        ((seconds / 3600.0).round(), "hr.")
    } else if days.round() < 7.0 {
        (days.round(), "day")
    } else if days < 30.44 {
        ((days / 7.0).floor(), "wk.")
    } else if days < 365.25 {
        ((days / 30.44).floor(), "mo.")
    } else {
        ((days / 365.25).floor(), "yr.")
    };
    let unit = if unit == "day" && count != 1.0 { "days" } else { unit };
    if delta < 0.0 {
        format!("in {count} {unit}")
    } else {
        format!("{count} {unit} ago")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_durations_and_stats_follow_swift_formats() {
        assert_eq!(bytes(21_265_280), "20.3 MB");
        assert_eq!(bytes(0), "0.0 MB");
        assert_eq!(bytes((1 << 30) - 1), "1024.0 MB");
        assert_eq!(bytes(3 << 29), "1.5 GB");
        assert_eq!(megabytes(325_976_064), "310.9 MB");
        assert_eq!(duration(149.9), "2:29");
        assert_eq!(duration(4_322.0), "72:02");
        assert_eq!(stats(0, 28, 6), "0 kept · 28 on device · 6 in cloud");
        assert_eq!(stats(10, 17, 0), "10 kept · 17 on device");
    }

    #[test]
    fn ages_match_foundations_abbreviated_relative_style() {
        // Verified against Date.formatted(.relative(presentation: .numeric, unitsStyle: .abbreviated)).
        let at = |seconds: f64| age(1.0e9, Some(1.0e9 - seconds));
        assert_eq!(age(1.0e9, None), "Not scanned");
        assert_eq!(at(1.0), "1 sec. ago");
        assert_eq!(at(59.0), "59 sec. ago");
        assert_eq!(at(119.0), "2 min. ago");
        assert_eq!(at(840.0), "14 min. ago");
        assert_eq!(at(3_599.0), "1 hr. ago");
        assert_eq!(at(27_000.0), "8 hr. ago");
        assert_eq!(at(86_400.0), "1 day ago");
        assert_eq!(at(259_200.0), "3 days ago");
        assert_eq!(at(604_800.0), "1 wk. ago");
        assert_eq!(at(1_728_000.0), "2 wk. ago");
        assert_eq!(at(3_456_000.0), "1 mo. ago");
        assert_eq!(at(34_560_000.0), "1 yr. ago");
        assert_eq!(at(-300.0), "in 5 min.");
    }
}
