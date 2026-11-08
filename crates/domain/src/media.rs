//! Référence vers un média (image, audio, pièce jointe) stocké à côté des tâches.
//!
//! Un média est nommé par l'**empreinte SHA-256 de son contenu** suivie de son extension
//! (`3f9a…c2.jpg`) : deux fichiers identiques n'occupent qu'une place, et un média n'est jamais
//! réécrit (pas de conflit de synchronisation possible, architecture §4). Dans la note, il est
//! cité par un chemin relatif : `../media/<référence>`.

use crate::DomainError;
use std::fmt;

/// Nom d'un média : 64 chiffres hexadécimaux (SHA-256), un point, une extension (1 à 8
/// caractères alphanumériques en minuscules).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MediaRef(String);

/// Taille maximale d'un média : 50 Mo (spécification §6.3, décision du 2026-11-06).
pub const MAX_MEDIA_BYTES: usize = 50 * 1024 * 1024;

/// Préfixe des liens vers un média dans le Markdown d'une note.
pub const MEDIA_LINK_PREFIX: &str = "../media/";

impl MediaRef {
    /// Construit la référence d'un contenu à partir de son empreinte et de son extension.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidMediaRef`] si l'empreinte ou l'extension est mal formée.
    pub fn from_parts(sha256_hex: &str, extension: &str) -> Result<Self, DomainError> {
        Self::parse(&format!(
            "{}.{}",
            sha256_hex,
            extension.trim_start_matches('.').to_ascii_lowercase()
        ))
    }

    /// Valide un nom de média (`<sha256>.<ext>`). Refuse tout ce qui ressemble à un chemin
    /// (`/`, `..`) : un nom venu de l'interface ne peut pas sortir du dossier des médias.
    ///
    /// # Erreurs
    /// [`DomainError::InvalidMediaRef`].
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::InvalidMediaRef(text.to_string());
        let (hash, extension) = text.split_once('.').ok_or_else(invalid)?;
        let hash_ok = hash.len() == 64
            && hash
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
        let extension_ok = (1..=8).contains(&extension.len())
            && extension
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
        if hash_ok && extension_ok {
            Ok(Self(text.to_string()))
        } else {
            Err(invalid())
        }
    }

    /// Lit la référence dans un lien de note (`../media/<référence>`), s'il en est un.
    pub fn from_note_link(link: &str) -> Option<Self> {
        Self::parse(link.strip_prefix(MEDIA_LINK_PREFIX)?).ok()
    }

    /// Lien à écrire dans la note : `../media/<référence>`.
    pub fn note_link(&self) -> String {
        format!("{MEDIA_LINK_PREFIX}{}", self.0)
    }

    /// Extension (`jpg`, `webm`, `pdf`…).
    pub fn extension(&self) -> &str {
        self.0
            .split_once('.')
            .map_or("", |(_, extension)| extension)
    }

    /// Le nom complet.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for MediaRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Vérifie la taille d'un média avant de le stocker.
///
/// # Erreurs
/// [`DomainError::MediaTooLarge`] au-delà de [`MAX_MEDIA_BYTES`].
pub fn check_media_size(size: usize) -> Result<(), DomainError> {
    if size > MAX_MEDIA_BYTES {
        Err(DomainError::MediaTooLarge {
            max_megabytes: MAX_MEDIA_BYTES / (1024 * 1024),
        })
    } else {
        Ok(())
    }
}

/// Médias cités par une note (liens `../media/<empreinte>.<ext>`), sans doublon, dans l'ordre.
/// Sert à emporter les médias quand une tâche change de base (ADR-0009).
///
/// ```
/// use tq_domain::{NoteBody, media_refs_in};
/// let hash = "a".repeat(64);
/// let note = NoteBody::from_markdown(&format!("![x](../media/{hash}.jpg) et ::file{{src=\"../media/{hash}.jpg\"}}"));
/// assert_eq!(media_refs_in(&note).len(), 1);
/// ```
pub fn media_refs_in(note: &crate::NoteBody) -> Vec<MediaRef> {
    let text = note.as_markdown();
    let mut found: Vec<MediaRef> = Vec::new();
    for (start, _) in text.match_indices(MEDIA_LINK_PREFIX) {
        let link: String = text[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '/'))
            .collect();
        if let Some(media) = MediaRef::from_note_link(&link)
            && !found.contains(&media)
        {
            found.push(media);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "3f9a5e0c2b1d4e6f8a7b9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f";

    #[test]
    fn valid_reference_and_note_link() {
        let media = MediaRef::from_parts(HASH, ".JPG").unwrap();
        assert_eq!(media.extension(), "jpg");
        assert_eq!(media.note_link(), format!("../media/{HASH}.jpg"));
        assert_eq!(MediaRef::from_note_link(&media.note_link()), Some(media));
    }

    #[test]
    fn path_like_or_malformed_names_are_refused() {
        for text in [
            "",
            "abc.jpg",
            HASH,
            &format!("{HASH}.verylongext"),
            &format!("../{HASH}.jpg"),
            &format!("{HASH}.j/pg"),
            &format!("{}.jpg", HASH.to_uppercase()),
        ] {
            assert!(
                MediaRef::parse(text).is_err(),
                "devrait être refusé : {text}"
            );
        }
        assert_eq!(
            MediaRef::from_note_link("https://exemple.fr/image.jpg"),
            None
        );
    }

    #[test]
    fn size_limit_is_fifty_megabytes() {
        assert!(check_media_size(MAX_MEDIA_BYTES).is_ok());
        assert_eq!(
            check_media_size(MAX_MEDIA_BYTES + 1),
            Err(DomainError::MediaTooLarge { max_megabytes: 50 })
        );
    }
}
