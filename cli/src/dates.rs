//! Civil-date arithmetic without a calendar dependency: ISO `YYYY-MM-DD`
//! parsing, days-since-epoch conversion (Howard Hinnant's algorithm), and a
//! `TRELLIS_TODAY` override so tests and reruns are hermetic.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub y: i32,
    pub m: u32,
    pub d: u32,
}

impl Date {
    pub fn parse(s: &str) -> Option<Date> {
        let s = s.trim();
        let mut parts = s.splitn(3, '-');
        let y: i32 = parts.next()?.parse().ok()?;
        let m: u32 = parts.next()?.parse().ok()?;
        let d: u32 = parts.next()?.parse().ok()?;
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
            return None;
        }
        Some(Date { y, m, d })
    }

    /// Days since 1970-01-01 (may be negative).
    pub fn days_from_epoch(&self) -> i64 {
        let y = if self.m <= 2 {
            self.y as i64 - 1
        } else {
            self.y as i64
        };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.m as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn from_epoch_days(z: i64) -> Date {
        let z = z + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
        Date {
            y: (if m <= 2 { y + 1 } else { y }) as i32,
            m,
            d,
        }
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

/// Today in UTC, or the `TRELLIS_TODAY=YYYY-MM-DD` override.
pub fn today() -> Date {
    if let Ok(s) = std::env::var("TRELLIS_TODAY") {
        if let Some(d) = Date::parse(&s) {
            return d;
        }
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Date::from_epoch_days(secs.div_euclid(86_400))
}

/// Whole days from `a` to `b` (positive when `b` is later).
pub fn days_between(a: Date, b: Date) -> i64 {
    b.days_from_epoch() - a.days_from_epoch()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for s in ["1970-01-01", "2000-02-29", "2026-08-03", "1999-12-31"] {
            let d = Date::parse(s).unwrap();
            assert_eq!(Date::from_epoch_days(d.days_from_epoch()), d);
            assert_eq!(d.to_string(), s);
        }
        assert_eq!(Date::parse("1970-01-01").unwrap().days_from_epoch(), 0);
        assert_eq!(
            days_between(
                Date::parse("2026-07-01").unwrap(),
                Date::parse("2026-08-03").unwrap()
            ),
            33
        );
    }
}
