use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// A calendar date, `YYYY-MM-DD`.
///
/// Hand-rolled rather than pulled from a date crate: an entry's date is a day a
/// human wrote down, with no clock, zone or arithmetic anywhere near it, and the
/// format has to stay parseable by anything that ever reads a worklog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// The civil date at a Unix timestamp, in UTC.
    ///
    /// Howard Hinnant's `civil_from_days`, which is a handful of lines and
    /// keeps this crate dependency-free and `wasm32`-clean. The binary reaches
    /// for a real time library to get the *local* date, because a worklog entry
    /// written at eleven at night should not be dated tomorrow.
    pub fn from_unix_seconds(seconds: i64) -> Date {
        let days = seconds.div_euclid(86_400) + 719_468;
        let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
        let doe = days - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = (yoe + era * 400) as i32 + i32::from(month <= 2);
        Date { year, month, day }
    }
}

/// A timestamp for `log.json`, as RFC 3339 in UTC.
pub fn rfc3339(seconds: i64) -> String {
    let date = Date::from_unix_seconds(seconds);
    let time = seconds.rem_euclid(86_400);
    format!(
        "{date}T{:02}:{:02}:{:02}Z",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

impl FromStr for Date {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = || format!("{s:?} is not a YYYY-MM-DD date");
        let (year, rest) = s.split_once('-').ok_or_else(bad)?;
        let (month, day) = rest.split_once('-').ok_or_else(bad)?;
        if year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return Err(bad());
        }
        let date = Date {
            year: year.parse().map_err(|_| bad())?,
            month: month.parse().map_err(|_| bad())?,
            day: day.parse().map_err(|_| bad())?,
        };
        if !(1..=12).contains(&date.month) || !(1..=31).contains(&date.day) {
            return Err(bad());
        }
        Ok(date)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_and_some_dates_after_it() {
        assert_eq!(Date::from_unix_seconds(0).to_string(), "1970-01-01");
        assert_eq!(
            Date::from_unix_seconds(1_788_000_000).to_string(),
            "2026-08-29"
        );
        // A leap day, which is the one the arithmetic is likely to get wrong.
        assert_eq!(
            Date::from_unix_seconds(1_709_164_800).to_string(),
            "2024-02-29"
        );
    }

    #[test]
    fn timestamps_are_rfc3339() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_709_164_800 + 3661), "2024-02-29T01:01:01Z");
    }

    #[test]
    fn dates_round_trip_through_text() {
        let date: Date = "2026-09-20".parse().unwrap();
        assert_eq!(date.to_string(), "2026-09-20");
        assert!("2026-9-20".parse::<Date>().is_err());
        assert!("2026-13-01".parse::<Date>().is_err());
    }
}
