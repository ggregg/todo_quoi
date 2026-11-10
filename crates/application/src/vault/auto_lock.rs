//! **Couche application** (valeur pure) : délai du **verrouillage automatique** des coffres (spec §9) : réglage de l'appareil.

use tq_domain::Timestamp;

/// Après combien de minutes sans activité les coffres se verrouillent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoLockDelay {
    /// Jamais (verrouillage à la main seulement).
    Never,
    /// Après ce nombre de minutes (au moins 1).
    AfterMinutes(u32),
}

impl AutoLockDelay {
    /// Délai par défaut : 15 minutes.
    pub const DEFAULT: Self = Self::AfterMinutes(15);

    /// Lit un nombre de minutes ; `0` = jamais.
    pub fn from_minutes(minutes: u32) -> Self {
        if minutes == 0 {
            Self::Never
        } else {
            Self::AfterMinutes(minutes)
        }
    }

    /// Nombre de minutes ; `0` = jamais.
    pub fn minutes(self) -> u32 {
        match self {
            Self::Never => 0,
            Self::AfterMinutes(minutes) => minutes,
        }
    }

    /// Vrai si l'inactivité depuis `last_activity` dépasse le délai à l'instant `now`.
    pub fn is_due(self, last_activity: Timestamp, now: Timestamp) -> bool {
        match self {
            Self::Never => false,
            Self::AfterMinutes(minutes) => {
                let delay_millis = i64::from(minutes) * 60_000;
                now.as_millis() - last_activity.as_millis() >= delay_millis
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_once_the_delay_has_passed() {
        let start = Timestamp::from_millis(1_000_000);
        let cases = [
            (AutoLockDelay::AfterMinutes(15), 14 * 60_000, false),
            (AutoLockDelay::AfterMinutes(15), 15 * 60_000, true),
            (AutoLockDelay::AfterMinutes(1), 61_000, true),
            (AutoLockDelay::Never, 1_000_000_000, false),
        ];
        for (delay, elapsed, expected) in cases {
            let now = start.plus_millis(elapsed);
            assert_eq!(delay.is_due(start, now), expected, "{delay:?} {elapsed}");
        }
    }

    #[test]
    fn zero_minutes_means_never() {
        assert_eq!(AutoLockDelay::from_minutes(0), AutoLockDelay::Never);
        assert_eq!(AutoLockDelay::from_minutes(5).minutes(), 5);
        assert_eq!(AutoLockDelay::Never.minutes(), 0);
    }
}
