//! L'entité centrale : une tâche.
//!
//! Les champs sont **privés** : on ne modifie une tâche qu'à travers des méthodes qui disent
//! l'intention (`close`, `rename`, `move_to`…) et qui maintiennent ses invariants (date de
//! modification à jour, pas de modification dans la corbeille…).
//!
//! Pour la persistance, [`TaskRecord`] expose tous les champs en lecture/écriture : c'est la
//! « photo » d'une tâche que les adaptateurs savent écrire et relire.

use crate::BaseId;
use crate::Confidentiality;
use crate::view::ListView;
use crate::{DeviceId, DomainError, NoteBody, OrderKey, Tag, TaskId, Timestamp, Title};

/// Statut d'une tâche (spécification : binaire, Ouvert / Terminé).
///
/// Une tâche terminée porte **toujours** sa date de clôture : l'état « terminée sans date »
/// n'est pas représentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// À faire.
    Open,
    /// Terminée, à la date indiquée.
    Done {
        /// Date de clôture.
        at: Timestamp,
    },
}

/// « Qui modifie, et quand » : apposé sur chaque modification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeStamp {
    /// Instant de la modification (horloge hybride, fournie par l'application).
    pub at: Timestamp,
    /// Appareil qui modifie.
    pub by: DeviceId,
}

/// Données nécessaires à la création d'une tâche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTask {
    /// Identifiant (généré par l'application).
    pub id: TaskId,
    /// Tâche parente ; `None` = premier niveau.
    pub parent: Option<TaskId>,
    /// Position parmi les sœurs.
    pub order: OrderKey,
    /// Titre.
    pub title: Title,
}

/// Photo complète d'une tâche, pour l'écrire ou la relire (adaptateurs de stockage).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    /// Identifiant.
    pub id: TaskId,
    /// Tâche parente ; `None` = premier niveau.
    pub parent: Option<TaskId>,
    /// Position parmi les sœurs.
    pub order: OrderKey,
    /// Titre.
    pub title: Title,
    /// Ouverte ou terminée.
    pub status: TaskStatus,
    /// Étiquettes, sans doublon.
    pub tags: Vec<Tag>,
    /// Note (Markdown).
    pub note: NoteBody,
    /// Création.
    pub created_at: Timestamp,
    /// Dernière modification.
    pub updated_at: Timestamp,
    /// Appareil de la dernière modification.
    pub updated_by: DeviceId,
    /// Date de mise à la corbeille ; `None` = active.
    pub deleted_at: Option<Timestamp>,
    /// En clair, chiffrée ou scellée (spec §9, ADR-0010).
    pub confidentiality: Confidentiality,
    /// Présentation de la liste de ses sous-tâches ; `None` = héritée (spec §5.3).
    pub view: Option<ListView>,
}

/// Une tâche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    record: TaskRecord,
    /// Base qui contient la tâche (ADR-0009). Pas dans `record` : le fichier ne l'écrit pas,
    /// c'est le dossier qui la porte.
    base: BaseId,
}

impl Task {
    /// Crée une nouvelle tâche ouverte, sans étiquette ni note.
    pub fn create(new: NewTask, stamp: &ChangeStamp) -> Self {
        Self {
            record: TaskRecord {
                id: new.id,
                parent: new.parent,
                order: new.order,
                title: new.title,
                status: TaskStatus::Open,
                tags: Vec::new(),
                note: NoteBody::empty(),
                created_at: stamp.at,
                updated_at: stamp.at,
                updated_by: stamp.by.clone(),
                deleted_at: None,
                confidentiality: Confidentiality::Plain,
                view: None,
            },
            base: BaseId::main(),
        }
    }

    /// Reconstitue une tâche relue depuis le stockage. Les étiquettes en double sont fusionnées.
    pub fn from_record(mut record: TaskRecord) -> Self {
        record.tags.sort();
        record.tags.dedup();
        Self {
            record,
            base: BaseId::main(),
        }
    }

    /// La même tâche, rangée dans `base` (au chargement : la base du dossier lu ; à la création :
    /// la base choisie).
    pub fn in_base(mut self, base: BaseId) -> Self {
        self.base = base;
        self
    }

    /// Photo de la tâche, pour l'écrire.
    pub fn to_record(&self) -> TaskRecord {
        self.record.clone()
    }

    // ----- Lecture --------------------------------------------------------------------------

    /// Identifiant.
    pub fn id(&self) -> &TaskId {
        &self.record.id
    }
    /// Tâche parente ; `None` = premier niveau.
    pub fn parent(&self) -> Option<&TaskId> {
        self.record.parent.as_ref()
    }
    /// Position parmi les sœurs.
    pub fn order(&self) -> &OrderKey {
        &self.record.order
    }
    /// Titre.
    pub fn title(&self) -> &Title {
        &self.record.title
    }
    /// Statut.
    pub fn status(&self) -> TaskStatus {
        self.record.status
    }
    /// Vrai si la tâche est terminée.
    pub fn is_done(&self) -> bool {
        matches!(self.record.status, TaskStatus::Done { .. })
    }
    /// Étiquettes (triées, sans doublon).
    pub fn tags(&self) -> &[Tag] {
        &self.record.tags
    }
    /// Note.
    pub fn note(&self) -> &NoteBody {
        &self.record.note
    }
    /// Date de création.
    pub fn created_at(&self) -> Timestamp {
        self.record.created_at
    }
    /// Date de dernière modification (sert à départager les conflits).
    pub fn updated_at(&self) -> Timestamp {
        self.record.updated_at
    }
    /// Appareil de la dernière modification.
    pub fn updated_by(&self) -> &DeviceId {
        &self.record.updated_by
    }
    /// Date de mise à la corbeille, si la tâche y est.
    pub fn deleted_at(&self) -> Option<Timestamp> {
        self.record.deleted_at
    }
    /// Date de clôture, si la tâche est terminée.
    pub fn done_at(&self) -> Option<Timestamp> {
        match self.record.status {
            TaskStatus::Open => None,
            TaskStatus::Done { at } => Some(at),
        }
    }

    /// En clair, chiffrée ou scellée.
    pub fn confidentiality(&self) -> &Confidentiality {
        &self.record.confidentiality
    }

    /// Vrai si sa note est illisible (chiffrée, coffre de sa base verrouillé).
    pub fn is_sealed(&self) -> bool {
        self.record.confidentiality.is_sealed()
    }

    /// Base qui contient la tâche.
    pub fn base(&self) -> &BaseId {
        &self.base
    }

    /// Vue propre de la liste de ses sous-tâches (`None` = héritée).
    pub fn view(&self) -> Option<&ListView> {
        self.record.view.as_ref()
    }

    /// Vrai si la tâche est dans la corbeille.
    pub fn is_deleted(&self) -> bool {
        self.record.deleted_at.is_some()
    }

    // ----- Modifications (chacune met à jour `updated_at` / `updated_by`) --------------------

    /// Change le titre.
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`] si la tâche est dans la corbeille.
    pub fn rename(&mut self, title: Title, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.record.title = title;
        self.touch(stamp);
        Ok(())
    }

    /// Clôture la tâche.
    ///
    /// # Erreurs
    /// [`DomainError::AlreadyClosed`] ou [`DomainError::TaskInTrash`].
    pub fn close(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        if self.is_done() {
            return Err(DomainError::AlreadyClosed(self.record.id.clone()));
        }
        self.record.status = TaskStatus::Done { at: stamp.at };
        self.touch(stamp);
        Ok(())
    }

    /// Rouvre la tâche.
    ///
    /// # Erreurs
    /// [`DomainError::AlreadyOpen`] ou [`DomainError::TaskInTrash`].
    pub fn reopen(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        if !self.is_done() {
            return Err(DomainError::AlreadyOpen(self.record.id.clone()));
        }
        self.record.status = TaskStatus::Open;
        self.touch(stamp);
        Ok(())
    }

    /// Remplace toutes les étiquettes (doublons fusionnés, ordre alphabétique).
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`].
    pub fn set_tags(&mut self, mut tags: Vec<Tag>, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        tags.sort();
        tags.dedup();
        self.record.tags = tags;
        self.touch(stamp);
        Ok(())
    }

    /// Remplace la note.
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`] ; [`DomainError::TaskSealed`] si la note est chiffrée et le
    /// coffre verrouillé (on écraserait la vraie note).
    pub fn replace_note(&mut self, note: NoteBody, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.ensure_readable()?;
        self.record.note = note;
        self.touch(stamp);
        Ok(())
    }

    /// Change de parent et/ou de position. **Ne vérifie pas l'absence de cycle** : c'est une
    /// règle qui concerne l'arbre entier, appliquée par [`crate::rules::hierarchy::apply_move`].
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`].
    pub fn move_to(
        &mut self,
        parent: Option<TaskId>,
        order: OrderKey,
        stamp: &ChangeStamp,
    ) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.record.parent = parent;
        self.record.order = order;
        self.touch(stamp);
        Ok(())
    }

    /// Change de base (déplacement vers une autre base, avec tout son sous-arbre : voir
    /// [`crate::rules::hierarchy::apply_move`]).
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`].
    pub fn move_to_base(&mut self, base: BaseId, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.ensure_readable()?;
        self.base = base;
        self.touch(stamp);
        Ok(())
    }

    /// Donne une vue propre à la liste de ses sous-tâches (« Personnaliser cette liste »).
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`].
    pub fn customize_view(
        &mut self,
        view: ListView,
        stamp: &ChangeStamp,
    ) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.record.view = Some(view.normalized());
        self.touch(stamp);
        Ok(())
    }

    /// Retire la vue propre : la liste reprend celle dont elle hérite (« Revenir à l'héritage »).
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`].
    pub fn inherit_view(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.record.view = None;
        self.touch(stamp);
        Ok(())
    }

    /// Demande que la tâche soit écrite chiffrée (bouton 🔒). Le chiffrement lui-même est fait
    /// à l'écriture, par le stockage, avec la clé du coffre de sa base.
    ///
    /// # Erreurs
    /// [`DomainError::AlreadyEncrypted`] ou [`DomainError::TaskInTrash`].
    pub fn encrypt(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        if self.record.confidentiality.is_encrypted() {
            return Err(DomainError::AlreadyEncrypted(self.record.id.clone()));
        }
        self.record.confidentiality = Confidentiality::Encrypted;
        self.touch(stamp);
        Ok(())
    }

    /// Remet la tâche en clair.
    ///
    /// # Erreurs
    /// [`DomainError::NotEncrypted`], [`DomainError::TaskSealed`] (coffre verrouillé : on ne
    /// peut pas lire ce qu'il faudrait écrire en clair) ou [`DomainError::TaskInTrash`].
    pub fn decrypt(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.ensure_readable()?;
        if !self.record.confidentiality.is_encrypted() {
            return Err(DomainError::NotEncrypted(self.record.id.clone()));
        }
        self.record.confidentiality = Confidentiality::Plain;
        self.touch(stamp);
        Ok(())
    }

    /// Met la tâche à la corbeille (sans toucher aux sous-tâches : voir [`crate::rules::trash`]).
    ///
    /// # Erreurs
    /// [`DomainError::TaskInTrash`] si elle y est déjà.
    pub fn move_to_trash(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        self.ensure_active()?;
        self.record.deleted_at = Some(stamp.at);
        self.touch(stamp);
        Ok(())
    }

    /// Sort la tâche de la corbeille.
    ///
    /// # Erreurs
    /// [`DomainError::TaskNotInTrash`].
    pub fn restore_from_trash(&mut self, stamp: &ChangeStamp) -> Result<(), DomainError> {
        if !self.is_deleted() {
            return Err(DomainError::TaskNotInTrash(self.record.id.clone()));
        }
        self.record.deleted_at = None;
        self.touch(stamp);
        Ok(())
    }

    /// Rattache la tâche au premier niveau, à la position donnée (restauration quand le parent
    /// d'origine n'existe plus, règle R7). Autorisé même si la tâche est dans la corbeille.
    pub(crate) fn detach_to_root(&mut self, order: OrderKey, stamp: &ChangeStamp) {
        self.record.parent = None;
        self.record.order = order;
        self.touch(stamp);
    }

    fn ensure_active(&self) -> Result<(), DomainError> {
        if self.is_deleted() {
            Err(DomainError::TaskInTrash(self.record.id.clone()))
        } else {
            Ok(())
        }
    }

    fn ensure_readable(&self) -> Result<(), DomainError> {
        if self.is_sealed() {
            Err(DomainError::TaskSealed(self.record.id.clone()))
        } else {
            Ok(())
        }
    }

    fn touch(&mut self, stamp: &ChangeStamp) {
        self.record.updated_at = stamp.at;
        self.record.updated_by = stamp.by.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(millis: i64) -> ChangeStamp {
        ChangeStamp {
            at: Timestamp::from_millis(millis),
            by: DeviceId::parse("test").unwrap(),
        }
    }

    fn new_task() -> Task {
        Task::create(
            NewTask {
                id: TaskId::parse("00000000-0000-7000-8000-000000000001").unwrap(),
                parent: None,
                order: OrderKey::first(),
                title: Title::parse("Écrire les tests").unwrap(),
            },
            &stamp(1_000),
        )
    }

    #[test]
    fn a_new_task_is_open_and_stamped() {
        let task = new_task();
        assert_eq!(task.status(), TaskStatus::Open);
        assert_eq!(task.created_at(), Timestamp::from_millis(1_000));
        assert_eq!(task.updated_at(), Timestamp::from_millis(1_000));
    }

    #[test]
    fn close_then_reopen() {
        let mut task = new_task();
        task.close(&stamp(2_000)).unwrap();
        assert_eq!(
            task.status(),
            TaskStatus::Done {
                at: Timestamp::from_millis(2_000)
            }
        );
        assert!(matches!(
            task.close(&stamp(2_500)),
            Err(DomainError::AlreadyClosed(_))
        ));
        task.reopen(&stamp(3_000)).unwrap();
        assert!(!task.is_done());
        assert_eq!(task.updated_at(), Timestamp::from_millis(3_000));
        assert!(matches!(
            task.reopen(&stamp(3_500)),
            Err(DomainError::AlreadyOpen(_))
        ));
    }

    #[test]
    fn a_task_in_the_trash_cannot_be_modified_until_restored() {
        let mut task = new_task();
        task.move_to_trash(&stamp(2_000)).unwrap();
        assert!(matches!(
            task.close(&stamp(3_000)),
            Err(DomainError::TaskInTrash(_))
        ));
        assert!(matches!(
            task.rename(Title::parse("x").unwrap(), &stamp(3_000)),
            Err(DomainError::TaskInTrash(_))
        ));
        task.restore_from_trash(&stamp(4_000)).unwrap();
        assert!(task.close(&stamp(5_000)).is_ok());
    }

    #[test]
    fn tags_are_sorted_and_deduplicated() {
        let mut task = new_task();
        let tags = ["b", "a", "b"].map(|t| Tag::parse(t).unwrap()).to_vec();
        task.set_tags(tags, &stamp(2_000)).unwrap();
        assert_eq!(
            task.tags().iter().map(Tag::as_str).collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    fn sealed_task() -> Task {
        let mut record = new_task().to_record();
        record.confidentiality = Confidentiality::Sealed(crate::SealedContent {
            key_id: crate::KeyId::parse("0123456789abcdef0123456789abcdef").unwrap(),
            sealed: "opaque".to_string(),
        });
        Task::from_record(record)
    }

    #[test]
    fn encrypt_then_decrypt() {
        let mut task = new_task();
        task.encrypt(&stamp(2_000)).unwrap();
        assert_eq!(task.confidentiality(), &Confidentiality::Encrypted);
        assert!(matches!(
            task.encrypt(&stamp(2_500)),
            Err(DomainError::AlreadyEncrypted(_))
        ));
        task.decrypt(&stamp(3_000)).unwrap();
        assert_eq!(task.confidentiality(), &Confidentiality::Plain);
        assert!(matches!(
            task.decrypt(&stamp(3_500)),
            Err(DomainError::NotEncrypted(_))
        ));
    }

    #[test]
    fn a_sealed_task_keeps_everything_editable_but_its_note() {
        let mut task = sealed_task();
        let sealed =
            |result: Result<(), DomainError>| matches!(result, Err(DomainError::TaskSealed(_)));
        assert!(sealed(task.replace_note(NoteBody::empty(), &stamp(2_000))));
        assert!(sealed(task.move_to_base(BaseId::main(), &stamp(2_000))));
        assert!(sealed(task.decrypt(&stamp(2_000))));
        // Titre, étiquettes et vue sont en clair : modifiables coffre verrouillé.
        assert!(
            task.rename(Title::parse("x").unwrap(), &stamp(2_500))
                .is_ok()
        );
        assert!(task.set_tags(Vec::new(), &stamp(2_500)).is_ok());
        assert!(task.inherit_view(&stamp(2_500)).is_ok());
        // Statut, place et corbeille restent modifiables : ils sont écrits en clair.
        assert!(task.close(&stamp(3_000)).is_ok());
        assert!(task.move_to(None, OrderKey::first(), &stamp(4_000)).is_ok());
        assert!(task.move_to_trash(&stamp(5_000)).is_ok());
        assert!(task.is_sealed());
    }

    #[test]
    fn record_round_trip() {
        let mut task = new_task();
        task.replace_note(NoteBody::from_markdown("# Note\n"), &stamp(2_000))
            .unwrap();
        assert_eq!(Task::from_record(task.to_record()), task);
    }
}
