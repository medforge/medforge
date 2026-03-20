//! HL7v2 timestamp parsing utilities.
//!
//! HL7 timestamps follow the format: `YYYY[MM[DD[HH[MM[SS[.S[S[S[S]]]]]]]]][+/-ZZZZ]`
//! This module parses them into `chrono` datetime types.

use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use pyo3::prelude::*;

/// Parse an HL7v2 timestamp string into a Python `datetime.datetime`.
///
/// Handles full and partial timestamps:
/// - `"20230101"` → `datetime(2023, 1, 1)`
/// - `"20230101120000"` → `datetime(2023, 1, 1, 12, 0, 0)`
/// - `"20230101120000.123"` → `datetime(2023, 1, 1, 12, 0, 0, 123000)`
/// - `"20230101120000-0500"` → `datetime(2023, 1, 1, 12, 0, 0, tzinfo=...)`
#[pyfunction]
#[pyo3(signature = (raw))]
pub fn parse_datetime(py: Python<'_>, raw: &str) -> PyResult<PyObject> {
    let (dt_str, tz_offset) = split_timezone(raw);

    let ndt = parse_naive_datetime(dt_str)?;

    if let Some(offset_minutes) = tz_offset {
        let offset = FixedOffset::east_opt(offset_minutes * 60).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid timezone offset: {} minutes",
                offset_minutes
            ))
        })?;
        let aware_dt = offset.from_local_datetime(&ndt).single().ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("Ambiguous datetime with timezone")
        })?;

        // Build Python datetime with timezone
        let datetime_mod = py.import("datetime")?;
        let timezone_cls = datetime_mod.getattr("timezone")?;
        let timedelta_cls = datetime_mod.getattr("timedelta")?;

        let td = timedelta_cls.call1((0, offset_minutes * 60))?;
        let tz = timezone_cls.call1((td,))?;

        let dt = datetime_mod.getattr("datetime")?.call1((
            aware_dt.naive_local().date().year() as i32,
            aware_dt.naive_local().date().month0() as i32 + 1,
            aware_dt.naive_local().date().day() as i32,
            aware_dt.naive_local().time().hour() as i32,
            aware_dt.naive_local().time().minute() as i32,
            aware_dt.naive_local().time().second() as i32,
            ndt.and_utc().timestamp_subsec_micros() as i32,
            tz,
        ))?;

        Ok(dt.into())
    } else {
        // Return naive datetime
        let datetime_mod = py.import("datetime")?;
        let dt = datetime_mod.getattr("datetime")?.call1((
            ndt.date().year() as i32,
            ndt.date().month0() as i32 + 1,
            ndt.date().day() as i32,
            ndt.time().hour() as i32,
            ndt.time().minute() as i32,
            ndt.time().second() as i32,
            (ndt.time().nanosecond() / 1000) as i32,
        ))?;
        Ok(dt.into())
    }
}

/// Parse an HL7v2 timestamp into just a Python `datetime.date`.
#[pyfunction]
#[pyo3(signature = (raw))]
pub fn parse_date(py: Python<'_>, raw: &str) -> PyResult<PyObject> {
    let (dt_str, _) = split_timezone(raw);

    if dt_str.len() < 8 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Timestamp too short for date: '{}'",
            raw
        )));
    }

    let year: i32 = dt_str[..4].parse().map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid year in '{}'", raw))
    })?;
    let month: u32 = dt_str[4..6].parse().map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid month in '{}'", raw))
    })?;
    let day: u32 = dt_str[6..8].parse().map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid day in '{}'", raw))
    })?;

    let datetime_mod = py.import("datetime")?;
    let date = datetime_mod.getattr("date")?.call1((year, month, day))?;
    Ok(date.into())
}

use chrono::Datelike;
use chrono::Timelike;

/// Split timezone suffix from timestamp string.
/// Returns (datetime_part, optional_offset_in_minutes)
fn split_timezone(raw: &str) -> (&str, Option<i32>) {
    let raw = raw.trim();
    let len = raw.len();

    // Check for +HHMM or -HHMM at the end
    if len >= 5 {
        let possible_tz_start = len - 5;
        let tz_part = &raw[possible_tz_start..];
        if tz_part.starts_with('+') || tz_part.starts_with('-') {
            if let (Ok(hours), Ok(mins)) =
                (tz_part[1..3].parse::<i32>(), tz_part[3..5].parse::<i32>())
            {
                let sign = if tz_part.starts_with('-') { -1 } else { 1 };
                let offset_minutes = sign * (hours * 60 + mins);
                return (&raw[..possible_tz_start], Some(offset_minutes));
            }
        }
    }

    (raw, None)
}

/// Parse the datetime portion of an HL7 timestamp (without timezone).
fn parse_naive_datetime(s: &str) -> Result<NaiveDateTime, pyo3::PyErr> {
    let len = s.len();

    if len < 4 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Timestamp too short: '{}'",
            s
        )));
    }

    let year: i32 = s[..4]
        .parse()
        .map_err(|_| pyo3::exceptions::PyValueError::new_err(format!("Invalid year in '{}'", s)))?;
    let month: u32 = if len >= 6 {
        s[4..6].parse().unwrap_or(1)
    } else {
        1
    };
    let day: u32 = if len >= 8 {
        s[6..8].parse().unwrap_or(1)
    } else {
        1
    };

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid date: {}-{}-{}", year, month, day))
    })?;

    let hour: u32 = if len >= 10 {
        s[8..10].parse().unwrap_or(0)
    } else {
        0
    };
    let minute: u32 = if len >= 12 {
        s[10..12].parse().unwrap_or(0)
    } else {
        0
    };
    let second: u32 = if len >= 14 {
        s[12..14].parse().unwrap_or(0)
    } else {
        0
    };

    // Fractional seconds (after the dot)
    let micros: u32 = if len > 15 && s.as_bytes()[14] == b'.' {
        let frac_str = &s[15..];
        // Pad or truncate to 6 digits (microseconds)
        let padded = format!("{:0<6}", frac_str);
        padded[..6].parse().unwrap_or(0)
    } else {
        0
    };

    let time = NaiveTime::from_hms_micro_opt(hour, minute, second, micros).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid time: {}:{}:{}.{}",
            hour, minute, second, micros
        ))
    })?;

    Ok(NaiveDateTime::new(date, time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_timezone_positive() {
        let (dt, tz) = split_timezone("20230101120000+0500");
        assert_eq!(dt, "20230101120000");
        assert_eq!(tz, Some(300));
    }

    #[test]
    fn test_split_timezone_negative() {
        let (dt, tz) = split_timezone("20230101120000-0700");
        assert_eq!(dt, "20230101120000");
        assert_eq!(tz, Some(-420));
    }

    #[test]
    fn test_split_timezone_none() {
        let (dt, tz) = split_timezone("20230101120000");
        assert_eq!(dt, "20230101120000");
        assert_eq!(tz, None);
    }

    #[test]
    fn test_parse_naive_full() {
        let ndt = parse_naive_datetime("20230315143022").unwrap();
        assert_eq!(ndt.date().year(), 2023);
        assert_eq!(ndt.date().month(), 3);
        assert_eq!(ndt.date().day(), 15);
        assert_eq!(ndt.time().hour(), 14);
        assert_eq!(ndt.time().minute(), 30);
        assert_eq!(ndt.time().second(), 22);
    }

    #[test]
    fn test_parse_naive_date_only() {
        let ndt = parse_naive_datetime("20230315").unwrap();
        assert_eq!(ndt.date().year(), 2023);
        assert_eq!(ndt.date().month(), 3);
        assert_eq!(ndt.date().day(), 15);
        assert_eq!(ndt.time().hour(), 0);
    }

    #[test]
    fn test_parse_naive_with_frac() {
        let ndt = parse_naive_datetime("20230315143022.123").unwrap();
        assert_eq!(ndt.time().nanosecond(), 123_000_000);
    }

    #[test]
    fn test_parse_naive_with_frac_4_digits() {
        let ndt = parse_naive_datetime("20230315143022.1234").unwrap();
        assert_eq!(ndt.time().nanosecond(), 123_400_000);
    }
}
