//! Instant précis à la milliseconde, en UTC, et sa forme texte RFC 3339
//! (`2026-11-05T16:02:11.120Z`).
//!
//! Écrit à la main plutôt qu'avec `chrono` ou `time` (politique de dépendances, ADR-0006) :
//! il suffit de convertir un nombre de jours en date du calendrier, ce que font les deux
//! fonctions [`days_from_civil`] et [`civil_from_days`] (algorithmes publics de Howard Hinnant).

use crate::DomainError;
use std::fmt;

const MILLIS_PER_DAY: i64 = 86_400_000;

/// Un instant UTC, en millisecondes depuis le 1er janvier 1970 (« époque Unix »).
///
/// Les comparaisons (`<`, `==`…) suivent l'ordre chronologique.
// [rust] `Copy` : la valeur est copiée à chaque affectation (comme un `number` en TS),
//        possible car elle ne contient qu'un entier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Crée un instant à partir de millisecondes depuis l'époque Unix.
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// Millisecondes depuis l'époque Unix.
    pub const fn as_millis(self) -> i64 {
        self.0
    }

    /// Instant décalé de `millis` millisecondes.
    pub const fn plus_millis(self, millis: i64) -> Self {
        Self(self.0 + millis)
    }

    /// Forme RFC 3339 en UTC, toujours avec trois chiffres de millisecondes.
    ///
    /// ```
    /// use tq_domain::Timestamp;
    /// assert_eq!(Timestamp::from_millis(0).to_rfc3339(), "1970-01-01T00:00:00.000Z");
    /// ```
    pub fn to_rfc3339(self) -> String {
        // [rust] `div_euclid` / `rem_euclid` : division « mathématique », correcte aussi
        //        pour les dates avant 1970 (où `/` et `%` arrondiraient dans le mauvais sens).
        let days = self.0.div_euclid(MILLIS_PER_DAY);
        let millis_of_day = self.0.rem_euclid(MILLIS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        let hours = millis_of_day / 3_600_000;
        let minutes = millis_of_day / 60_000 % 60;
        let seconds = millis_of_day / 1_000 % 60;
        let millis = millis_of_day % 1_000;
        format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
    }

    /// Lit une date RFC 3339 : `AAAA-MM-JJTHH:MM:SS`, fraction de seconde facultative
    /// (1 à 9 chiffres, tronquée à la milliseconde), puis `Z` ou un décalage `±HH:MM`.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidTimestamp`] si le texte ne respecte pas ce format.
    pub fn parse_rfc3339(text: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::InvalidTimestamp(text.to_string());
        // Tous les caractères attendus sont ASCII : on peut découper par octets sans risque.
        if !text.is_ascii() || text.len() < 20 {
            return Err(invalid());
        }
        let bytes = text.as_bytes();
        let separators_ok = bytes[4] == b'-'
            && bytes[7] == b'-'
            && matches!(bytes[10], b'T' | b't' | b' ')
            && bytes[13] == b':'
            && bytes[16] == b':';
        if !separators_ok {
            return Err(invalid());
        }
        let number =
            |range: std::ops::Range<usize>| text[range].parse::<i64>().map_err(|_| invalid());
        let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
        let (hours, minutes, seconds) = (number(11..13)?, number(14..16)?, number(17..19)?);
        let in_range = (1..=12).contains(&month)
            && (1..=days_in_month(year, month)).contains(&day)
            && hours < 24
            && minutes < 60
            && seconds < 60;
        if !in_range {
            return Err(invalid());
        }
        let (millis, offset_minutes) =
            parse_fraction_and_offset(&text[19..]).ok_or_else(invalid)?;
        let total = days_from_civil(year, month, day) * MILLIS_PER_DAY
            + ((hours * 60 + minutes - offset_minutes) * 60 + seconds) * 1_000
            + millis;
        Ok(Self(total))
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_rfc3339())
    }
}

/// Lit `.123456Z`, `Z`, `+02:00`… Renvoie (millisecondes, décalage en minutes).
fn parse_fraction_and_offset(rest: &str) -> Option<(i64, i64)> {
    let (fraction, zone) = match rest.strip_prefix('.') {
        Some(after_dot) => {
            let digits = after_dot.chars().take_while(char::is_ascii_digit).count();
            if !(1..=9).contains(&digits) {
                return None;
            }
            (&after_dot[..digits], &after_dot[digits..])
        }
        None => ("", rest),
    };
    // On garde 3 chiffres : « 12 » → 120 ms, « 123456 » → 123 ms.
    let millis_text = format!("{fraction:0<3}");
    let millis = millis_text[..3].parse::<i64>().ok()?;
    let offset = match zone {
        "Z" | "z" => 0,
        _ => parse_offset(zone)?,
    };
    Some((millis, offset))
}

/// `+02:00` → 120, `-05:30` → -330.
fn parse_offset(zone: &str) -> Option<i64> {
    let sign = match zone.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let (hours, minutes) = zone.get(1..)?.split_once(':')?;
    if hours.len() != 2 || minutes.len() != 2 {
        return None;
    }
    let (hours, minutes) = (hours.parse::<i64>().ok()?, minutes.parse::<i64>().ok()?);
    (hours < 24 && minutes < 60).then_some(sign * (hours * 60 + minutes))
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Nombre de jours depuis le 1970-01-01 pour une date du calendrier grégorien.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // L'année « commence » en mars pour que le 29 février soit le dernier jour de l'année.
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Inverse de [`days_from_civil`] : jours depuis 1970 → (année, mois, jour).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dates_are_formatted() {
        assert_eq!(
            Timestamp::from_millis(0).to_rfc3339(),
            "1970-01-01T00:00:00.000Z"
        );
        // 2026-11-05T16:02:11.120Z, calculé indépendamment.
        assert_eq!(
            Timestamp::from_millis(1_793_894_531_120).to_rfc3339(),
            "2026-11-05T16:02:11.120Z"
        );
        assert_eq!(
            Timestamp::from_millis(951_782_400_000).to_rfc3339(),
            "2000-02-29T00:00:00.000Z"
        );
    }

    #[test]
    fn parse_and_format_round_trip() {
        for text in [
            "1970-01-01T00:00:00.000Z",
            "2026-11-05T16:02:11.120Z",
            "2000-02-29T23:59:59.999Z",
        ] {
            assert_eq!(Timestamp::parse_rfc3339(text).unwrap().to_rfc3339(), text);
        }
    }

    #[test]
    fn every_day_over_four_centuries_round_trips() {
        // 1900 → 2300 : couvre les années bissextiles séculaires (1900 non, 2000 oui, 2100 non).
        for day in -25_567..120_000 {
            let timestamp = Timestamp::from_millis(day * MILLIS_PER_DAY + 45_296_789);
            assert_eq!(
                Timestamp::parse_rfc3339(&timestamp.to_rfc3339()),
                Ok(timestamp)
            );
        }
    }

    #[test]
    fn fractions_and_offsets_are_understood() {
        let utc = Timestamp::parse_rfc3339("2026-11-05T16:02:11.120Z").unwrap();
        assert_eq!(Timestamp::parse_rfc3339("2026-11-05T16:02:11.12Z"), Ok(utc));
        assert_eq!(
            Timestamp::parse_rfc3339("2026-11-05T16:02:11.120999Z"),
            Ok(utc)
        );
        assert_eq!(
            Timestamp::parse_rfc3339("2026-11-05T17:02:11.120+01:00"),
            Ok(utc)
        );
        assert_eq!(
            Timestamp::parse_rfc3339("2026-11-05T16:02:11Z")
                .unwrap()
                .plus_millis(120),
            utc
        );
    }

    #[test]
    fn invalid_dates_are_rejected() {
        for text in [
            "",
            "2026-11-05",
            "2026-13-01T00:00:00Z",
            "2026-02-29T00:00:00Z",
            "2026-11-05T24:00:00Z",
            "2026-11-05T16:02:11",
            "2026-11-05T16:02:11.Z",
            "2026/11/05T16:02:11Z",
        ] {
            assert!(
                Timestamp::parse_rfc3339(text).is_err(),
                "devrait être refusé : {text}"
            );
        }
    }

    #[test]
    fn ordering_is_chronological() {
        assert!(Timestamp::from_millis(1) < Timestamp::from_millis(2));
    }
}
