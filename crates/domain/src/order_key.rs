//! Clé d'ordre « fractionnaire » : la position d'une tâche parmi ses sœurs.
//!
//! Pourquoi pas un simple numéro (1, 2, 3…) ? Pour insérer entre 1 et 2, il faudrait renuméroter
//! toutes les tâches suivantes — donc réécrire autant de fichiers, sur un drive synchronisé.
//! Avec une clé fractionnaire, on fabrique toujours une nouvelle clé **entre** deux clés
//! existantes : un déplacement ne modifie qu'**un** fichier (ADR-0004).
//!
//! Les clés sont des chaînes en base 62 (`0-9A-Za-z`, dans l'ordre ASCII) comparées comme du
//! texte : `"V" < "k"`, et entre `"V"` et `"W"` on peut toujours créer `"VV"`.
//! Invariant : une clé n'est jamais vide et ne se termine jamais par `0` (sinon il n'existerait
//! pas toujours de clé plus petite qu'elle).

use crate::DomainError;
use std::fmt;

const DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE: u8 = 62;

/// Position d'une tâche parmi ses sœurs ; l'ordre des clés est l'ordre d'affichage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OrderKey(String);

impl OrderKey {
    /// Valide une clé lue depuis un fichier.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidOrderKey`] si la clé est vide, contient un caractère hors base 62
    /// ou se termine par `0`.
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let is_valid = !text.is_empty()
            && text.bytes().all(|byte| DIGITS.contains(&byte))
            && !text.ends_with('0');
        if is_valid {
            Ok(Self(text.to_string()))
        } else {
            Err(DomainError::InvalidOrderKey(text.to_string()))
        }
    }

    /// Clé d'une liste vide (`"V"`, le milieu de la base).
    pub fn first() -> Self {
        Self::from_digits(&midpoint(&[], None))
    }

    /// Nouvelle clé strictement entre `before` et `after` (bornes facultatives).
    ///
    /// - `between(None, None)` : première clé d'une liste vide ;
    /// - `between(Some(a), None)` : après `a` (fin de liste) ;
    /// - `between(None, Some(b))` : avant `b` (début de liste).
    ///
    /// # Erreurs
    /// [`DomainError::OrderKeysNotIncreasing`] si `before >= after`.
    ///
    /// ```
    /// use tq_domain::OrderKey;
    /// let a = OrderKey::first();
    /// let c = OrderKey::between(Some(&a), None).unwrap();
    /// let b = OrderKey::between(Some(&a), Some(&c)).unwrap();
    /// assert!(a < b && b < c);
    /// ```
    pub fn between(before: Option<&Self>, after: Option<&Self>) -> Result<Self, DomainError> {
        if let (Some(before), Some(after)) = (before, after)
            && before >= after
        {
            return Err(DomainError::OrderKeysNotIncreasing);
        }
        let low = before.map(Self::to_digits).unwrap_or_default();
        let high = after.map(Self::to_digits);
        Ok(Self::from_digits(&midpoint(&low, high.as_deref())))
    }

    /// Le texte de la clé.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn to_digits(&self) -> Vec<u8> {
        // Les octets sont garantis dans DIGITS par la construction : `position` trouve toujours.
        self.0
            .bytes()
            .map(|byte| DIGITS.iter().position(|d| *d == byte).unwrap_or(0) as u8)
            .collect()
    }

    fn from_digits(digits: &[u8]) -> Self {
        Self(
            digits
                .iter()
                .map(|digit| DIGITS[usize::from(*digit)] as char)
                .collect(),
        )
    }
}

impl fmt::Display for OrderKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Chiffres (base 62) d'une clé strictement entre `low` et `high` (`None` = +∞).
///
/// Principe : on avance tant que les deux bornes ont le même chiffre ; au premier chiffre
/// différent, on prend le chiffre du milieu s'il existe, sinon on descend d'un rang.
/// Algorithme de « fractional indexing » (David Greenspan).
fn midpoint(low: &[u8], high: Option<&[u8]>) -> Vec<u8> {
    if let Some(high) = high {
        // Longueur du préfixe commun (un chiffre absent de `low` vaut 0).
        let common = (0..high.len())
            .take_while(|&i| low.get(i).copied().unwrap_or(0) == high[i])
            .count();
        if common > 0 {
            let mut key = high[..common].to_vec();
            key.extend(midpoint(
                low.get(common..).unwrap_or(&[]),
                Some(&high[common..]),
            ));
            return key;
        }
    }
    let low_digit = low.first().copied().unwrap_or(0);
    let high_digit = high.and_then(|h| h.first().copied()).unwrap_or(BASE);
    if high_digit - low_digit > 1 {
        return vec![(low_digit + high_digit) / 2];
    }
    // Chiffres consécutifs : pas de place à ce rang.
    match high {
        Some(high) if high.len() > 1 => vec![high[0]],
        _ => {
            let mut key = vec![low_digit];
            key.extend(midpoint(low.get(1..).unwrap_or(&[]), None));
            key
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> OrderKey {
        OrderKey::parse(text).unwrap()
    }

    #[test]
    fn first_keys() {
        assert_eq!(OrderKey::first().as_str(), "V");
        assert_eq!(
            OrderKey::between(None, Some(&key("V"))).unwrap().as_str(),
            "F"
        );
        assert_eq!(
            OrderKey::between(Some(&key("V")), None).unwrap().as_str(),
            "k"
        );
    }

    #[test]
    fn appending_many_keys_stays_ordered_and_valid() {
        let mut keys = vec![OrderKey::first()];
        for _ in 0..500 {
            let next = OrderKey::between(keys.last(), None).unwrap();
            keys.push(next);
        }
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(keys.iter().all(|k| OrderKey::parse(k.as_str()).is_ok()));
    }

    #[test]
    fn prepending_many_keys_stays_ordered_and_valid() {
        let mut keys = vec![OrderKey::first()];
        for _ in 0..500 {
            let previous = OrderKey::between(None, keys.first()).unwrap();
            keys.insert(0, previous);
        }
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(keys.iter().all(|k| OrderKey::parse(k.as_str()).is_ok()));
    }

    #[test]
    fn repeated_insertion_in_the_middle() {
        let low = key("V");
        let mut high = key("W");
        for _ in 0..200 {
            let middle = OrderKey::between(Some(&low), Some(&high)).unwrap();
            assert!(low < middle && middle < high, "{low} < {middle} < {high}");
            assert!(OrderKey::parse(middle.as_str()).is_ok());
            high = middle;
        }
    }

    #[test]
    fn bounds_in_wrong_order_are_refused() {
        assert_eq!(
            OrderKey::between(Some(&key("k")), Some(&key("V"))),
            Err(DomainError::OrderKeysNotIncreasing)
        );
        assert_eq!(
            OrderKey::between(Some(&key("V")), Some(&key("V"))),
            Err(DomainError::OrderKeysNotIncreasing)
        );
    }

    #[test]
    fn invalid_keys_are_refused() {
        for text in ["", "V0", "a-b", "é"] {
            assert!(
                OrderKey::parse(text).is_err(),
                "devrait être refusé : {text:?}"
            );
        }
    }
}
