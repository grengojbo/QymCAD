//! THE STAMP OF A DOCUMENT: the current time in ISO-8601, UTC, to the second - what a document keeps as the moment
//! it was started, and what a report of a crash carries.

/// THE CURRENT TIME IN ISO-8601 (UTC, to the second).
///
/// The format chosen is machine-readable and sortable - whoever wants to show it to a person may, but what is stored
/// must be unambiguous.
pub fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    iso8601_from_unix(secs)
}

/// CONVERTING unix SECONDS INTO ISO-8601. Separate from "what time is it", because only this part is testable:
/// the current time has nothing to be compared against in a test, while a known stamp has.
pub fn iso8601_from_unix(secs: u64) -> String {
    // THE LAYOUT IS BUILT HERE, THE CALENDAR IS NOT. Leap years, the rule of centuries and the length of
    // February are a solved problem; the layout is four numbers and two separators and is held by the table
    // of known stamps beside it. Only the hard half was handed over.
    let t = time::OffsetDateTime::from_unix_timestamp(secs as i64).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", t.year(), u8::from(t.month()), t.day(), t.hour(), t.minute(), t.second())
}

#[cfg(test)]
mod tests {
    /// THE CALENDAR RECKONS RIGHTLY — CHECKED AGAINST KNOWN DATES.
    ///
    /// The calendar is written by hand (a date crate was not dragged in for the sake of one line), and
    /// by hand people get exactly two things wrong in it: leap years and crossing February. A range
    /// check does not catch that — "2026-02-30" passes straight through it. So the stamps taken are
    /// ones whose answer is known exactly, including 29 February and the boundaries of a day and a
    /// year.
    #[test]
    fn the_calendar_gets_known_dates_right() {
        for (secs, want) in [
            (0u64, "1970-01-01T00:00:00Z"),
            (86_399, "1970-01-01T23:59:59Z"),
            (86_400, "1970-01-02T00:00:00Z"),
            (951_782_400, "2000-02-29T00:00:00Z"),   // a leap century: 2000 is a leap year
            (1_078_012_800, "2004-02-29T00:00:00Z"), // an ordinary leap year
            (1_709_164_800, "2024-02-29T00:00:00Z"),
            (1_767_225_599, "2025-12-31T23:59:59Z"), // the boundary of a year
            (4_102_444_800, "2100-01-01T00:00:00Z"),
            // THE RULE OF CENTURIES is only exercised AFTER February: 2100 is not a leap year, and
            // without that rule the 1st of March would slide back a day. A stamp on the 1st of January
            // does not touch it at all — that is how it was written at first, and an honesty check
            // showed it.
            (4_107_542_400, "2100-03-01T00:00:00Z"),
            (4_233_772_800, "2104-03-01T00:00:00Z"), // and 2104 is a leap year — the rule must not eat that one too
        ] {
            assert_eq!(super::iso8601_from_unix(secs), want, "the calendar got the stamp {secs} wrong");
        }
    }

    /// THE DATE IS ISO-8601 AND READABLE. The format is deliberately a machine one: it is
    /// unambiguous and it sorts.
    #[test]
    fn the_timestamp_is_iso8601() {
        let s = super::now_iso8601();
        assert_eq!(s.len(), 20, "YYYY-MM-DDTHH:MM:SSZ was expected, and it came out \"{s}\"");
        assert!(s.ends_with('Z') && s.as_bytes()[10] == b'T', "the ISO-8601 layout is broken: \"{s}\"");
        let year: i32 = s[..4].parse().expect("the year as a number");
        assert!((2025..2100).contains(&year), "the year \"{year}\" is outside common sense — the calendar reckons wrongly");
        let month: u32 = s[5..7].parse().expect("the month as a number");
        let day: u32 = s[8..10].parse().expect("the day as a number");
        assert!((1..=12).contains(&month) && (1..=31).contains(&day), "month or day out of range: \"{s}\"");
    }
}
