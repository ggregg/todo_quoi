//! **Couche application**, service partagé par plusieurs cas d'usage. Médias des notes
//! **chiffrées** (spec §9, ADR-0010) : leur contenu est chiffré avec la clé du
//! coffre de la base, et leur nom est une empreinte calculée avec cette clé
//! (`<empreinte>.<ext>.tqe`, voir [`MediaRef::sealed_from_parts`]).
//!
//! Ce service est partagé par les cas d'usage qui écrivent ou lisent des médias (stocker, lire,
//! chiffrer / déchiffrer une tâche, la déplacer vers une autre base). Il ne décide rien : on lui
//! dit quoi faire, il le fait avec les ports `MediaStore` et `Cipher`.

use crate::AppError;
use crate::ports::{Cipher, MediaStore, RepositoryError};
use crate::vault::{ContentKey, OpenVaults, VaultError};
use std::sync::Arc;
use tq_domain::rules::trash::uncited_media;
use tq_domain::{
    BaseId, ChangeStamp, MediaRef, NoteBody, TaskId, TaskTree, media_refs_in, replace_media_in,
};

/// Nouvelle note, et les médias qu'elle ne cite plus (à effacer s'ils ne servent plus ailleurs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamedMedia {
    /// La note, dont les liens citent les nouveaux noms.
    pub note: NoteBody,
    /// Médias remplacés.
    pub replaced: Vec<MediaRef>,
}

/// Chiffre, déchiffre et renomme les médias des notes chiffrées.
pub struct SealedMedia {
    media: Arc<dyn MediaStore>,
    vaults: Arc<OpenVaults>,
    cipher: Arc<dyn Cipher>,
}

impl SealedMedia {
    /// Assemble le service avec ses ports.
    pub fn new(
        media: Arc<dyn MediaStore>,
        vaults: Arc<OpenVaults>,
        cipher: Arc<dyn Cipher>,
    ) -> Self {
        Self {
            media,
            vaults,
            cipher,
        }
    }

    /// Chiffre `bytes` avec la clé du coffre de `base` et le range ; renvoie son nom.
    ///
    /// # Erreurs
    /// [`VaultError::Locked`] si le coffre est fermé ; chiffrement ; stockage.
    pub fn store_sealed(
        &self,
        base: &BaseId,
        bytes: &[u8],
        extension: &str,
    ) -> Result<MediaRef, AppError> {
        let (_, key) = self.vaults.key_of(base).ok_or(VaultError::Locked)?;
        let fingerprint = self.cipher.fingerprint(&key, bytes)?;
        let media = MediaRef::sealed_from_parts(&fingerprint, extension)?;
        let sealed = self.cipher.seal_bytes(&key, bytes, &context(&media))?;
        self.media.put(base, &media, &sealed)?;
        Ok(media)
    }

    /// Contenu **en clair** d'un média (déchiffré s'il est chiffré).
    ///
    /// # Erreurs
    /// [`VaultError::Locked`] si aucun coffre ouvert ne l'ouvre ; média introuvable.
    pub fn read(&self, media: &MediaRef) -> Result<Vec<u8>, AppError> {
        let bytes = self.media.read(media)?;
        if !media.is_sealed() {
            return Ok(bytes);
        }
        // On ne sait pas de quelle base il vient : seule la clé de son coffre l'ouvre.
        self.vaults
            .open_keys()
            .iter()
            .find_map(|key| self.open_with(key, media, &bytes))
            .ok_or_else(|| VaultError::Locked.into())
    }

    /// Chiffre les médias en clair cités par `note` (la note d'une tâche qu'on chiffre).
    ///
    /// # Erreurs
    /// Coffre de `base` fermé, chiffrement, stockage.
    pub fn seal_note(&self, base: &BaseId, note: &NoteBody) -> Result<RenamedMedia, AppError> {
        self.rename(
            note,
            |media| !media.is_sealed(),
            |media, bytes| self.store_sealed(base, bytes, media.extension()),
        )
    }

    /// Remet en clair les médias chiffrés cités par `note` (la note d'une tâche qu'on déchiffre).
    ///
    /// # Erreurs
    /// Coffre fermé, stockage.
    pub fn unseal_note(&self, base: &BaseId, note: &NoteBody) -> Result<RenamedMedia, AppError> {
        self.rename(note, MediaRef::is_sealed, |media, bytes| {
            Ok(self.media.store(base, bytes, media.extension())?)
        })
    }

    /// Rechiffre avec la clé de `base` les médias chiffrés cités par `note` (une tâche chiffrée
    /// qui change de base : l'autre coffre a une autre clé, donc d'autres noms).
    ///
    /// # Erreurs
    /// Coffre de la source ou de `base` fermé, stockage.
    pub fn reseal_note(&self, base: &BaseId, note: &NoteBody) -> Result<RenamedMedia, AppError> {
        self.rename(note, MediaRef::is_sealed, |media, bytes| {
            self.store_sealed(base, bytes, media.extension())
        })
    }

    /// Applique `rename` (une des méthodes `*_note`) à la note de chaque tâche `ids` de l'arbre ;
    /// renvoie les médias remplacés, avec leur base.
    ///
    /// # Erreurs
    /// Celles de `rename` ; tâche introuvable ou scellée.
    // [rust] `impl Fn(…)` : paramètre « fonction » (≈ un callback `(media, base, note) => …` en TS).
    pub fn rename_in_tasks(
        &self,
        tree: &mut TaskTree,
        ids: &[TaskId],
        stamp: &ChangeStamp,
        rename: impl Fn(&Self, &BaseId, &NoteBody) -> Result<RenamedMedia, AppError>,
    ) -> Result<Vec<(BaseId, MediaRef)>, AppError> {
        let mut replaced = Vec::new();
        for id in ids {
            let task = tree.get(id)?;
            let base = task.base().clone();
            let renamed = rename(self, &base, task.note())?;
            if renamed.replaced.is_empty() {
                continue;
            }
            tree.get_mut(id)?.replace_note(renamed.note, stamp)?;
            replaced.extend(
                renamed
                    .replaced
                    .into_iter()
                    .map(|media| (base.clone(), media)),
            );
        }
        Ok(replaced)
    }

    /// Efface ceux des médias `replaced` qu'aucune tâche de leur base ne cite plus (règle
    /// [`uncited_media`]) : une copie en clair laissée derrière une note chiffrée la trahirait.
    ///
    /// # Erreurs
    /// [`RepositoryError`] si l'effacement échoue.
    pub fn erase_uncited(
        &self,
        tree: &TaskTree,
        replaced: &[(BaseId, MediaRef)],
    ) -> Result<(), AppError> {
        for (base, media) in replaced {
            // [rust] `std::slice::from_ref(media)` : une tranche d'un seul élément, sans copie
            //        (≈ `[media]` en TS).
            for uncited in uncited_media(tree, base, std::slice::from_ref(media)) {
                self.media.remove(base, &uncited)?;
            }
        }
        Ok(())
    }

    /// Pour chaque média de la note qui passe le filtre `concerned` : le lire en clair, le
    /// ranger sous un nouveau nom (`store`), et citer ce nom dans la note. Un média absent (pas
    /// encore synchronisé) est laissé tel quel.
    fn rename(
        &self,
        note: &NoteBody,
        concerned: impl Fn(&MediaRef) -> bool,
        store: impl Fn(&MediaRef, &[u8]) -> Result<MediaRef, AppError>,
    ) -> Result<RenamedMedia, AppError> {
        let mut renamed = RenamedMedia {
            note: note.clone(),
            replaced: Vec::new(),
        };
        for media in media_refs_in(note)
            .into_iter()
            .filter(|media| concerned(media))
        {
            let bytes = match self.read(&media) {
                // Absent (pas encore synchronisé) : on passe au suivant ; autre erreur : `?` la
                // renvoie ; succès : `other?` donne les octets.
                Err(AppError::Repository(RepositoryError::NotFound(_))) => continue,
                other => other?,
            };
            let new = store(&media, &bytes)?;
            if new != media {
                renamed.note = replace_media_in(&renamed.note, &media, &new);
                renamed.replaced.push(media);
            }
        }
        Ok(renamed)
    }

    fn open_with(&self, key: &ContentKey, media: &MediaRef, bytes: &[u8]) -> Option<Vec<u8>> {
        self.cipher.open_bytes(key, bytes, &context(media)).ok()
    }
}

/// Contexte authentifié d'un média chiffré : son nom. Un contenu chiffré renommé (ou copié sous
/// le nom d'un autre média) ne se déchiffre pas.
fn context(media: &MediaRef) -> Vec<u8> {
    format!("todoquoi-media:{media}").into_bytes()
}
