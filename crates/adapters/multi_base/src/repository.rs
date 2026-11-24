//! Dépôt de tâches de plusieurs bases.

use crate::find;
use crate::media::MultiBaseMediaStore;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tq_application::ports::{RepositoryError, TaskRepository};
use tq_domain::rules::conflict::is_newer;
use tq_domain::{BaseId, Task, TaskId, media_refs_in};

/// `TaskRepository` de chaque base, présentés comme un seul.
///
/// Il retient dans quelle base chaque tâche a été lue : si une tâche enregistrée a changé de
/// base, c'est un **déplacement** — écrire dans la nouvelle base (après y avoir copié les médias
/// cités par la note), puis effacer de l'ancienne. Dans cet ordre, une coupure entre les deux
/// laisse un doublon, jamais une perte ; au chargement, la version la plus récente l'emporte.
pub struct MultiBaseTaskRepository {
    bases: Vec<(BaseId, Arc<dyn TaskRepository>)>,
    media: Arc<MultiBaseMediaStore>,
    located: Mutex<HashMap<TaskId, BaseId>>,
}

impl MultiBaseTaskRepository {
    /// Assemble les dépôts des bases ouvertes ; `media` sert à emporter les médias.
    pub fn new(
        bases: Vec<(BaseId, Arc<dyn TaskRepository>)>,
        media: Arc<MultiBaseMediaStore>,
    ) -> Self {
        Self {
            bases,
            media,
            located: Mutex::new(HashMap::new()),
        }
    }

    fn located(&self) -> MutexGuard<'_, HashMap<TaskId, BaseId>> {
        self.located
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Tâches de toutes les bases lues par `load`, chacune marquée de sa base. Doublon entre deux
    /// bases (déplacement interrompu) : la plus récente gagne.
    // [rust] `load` est une fonction reçue en paramètre (`impl Fn`) : la même boucle sert pour
    //        `load_all` et `load_archived` (≈ `merged(load: (repo: TaskRepository) => Task[])`).
    fn merged(
        &self,
        load: impl Fn(&dyn TaskRepository) -> Result<Vec<Task>, RepositoryError>,
    ) -> Result<HashMap<TaskId, Task>, RepositoryError> {
        let mut by_id: HashMap<TaskId, Task> = HashMap::new();
        for (base, repository) in &self.bases {
            for task in load(repository.as_ref())? {
                let task = task.in_base(base.clone());
                let keep = by_id
                    .get(task.id())
                    .is_none_or(|known| is_newer(&task, known));
                if keep {
                    by_id.insert(task.id().clone(), task);
                }
            }
        }
        Ok(by_id)
    }

    /// La tâche quitte `from` pour sa nouvelle base : médias copiés, fichier écrit, ancien effacé.
    fn move_between_bases(&self, task: &Task, from: &BaseId) -> Result<(), RepositoryError> {
        for media in media_refs_in(task.note()) {
            self.media.copy(&media, from, task.base())?;
        }
        find(&self.bases, task.base())?.save(task)?;
        find(&self.bases, from)?.remove(task.id())
    }
}

impl TaskRepository for MultiBaseTaskRepository {
    fn load_all(&self) -> Result<Vec<Task>, RepositoryError> {
        let by_id = self.merged(|repository| repository.load_all())?;
        *self.located() = by_id
            .values()
            .map(|task| (task.id().clone(), task.base().clone()))
            .collect();
        Ok(by_id.into_values().collect())
    }

    /// Archives de toutes les bases. Leur base est retenue aussi : restaurée, une archive est
    /// réécrite dans la sienne (une archive ne change jamais de base, R14).
    fn load_archived(&self) -> Result<Vec<Task>, RepositoryError> {
        let by_id = self.merged(|repository| repository.load_archived())?;
        self.located().extend(
            by_id
                .values()
                .map(|task| (task.id().clone(), task.base().clone())),
        );
        Ok(by_id.into_values().collect())
    }

    fn save(&self, task: &Task) -> Result<(), RepositoryError> {
        let previous = self.located().get(task.id()).cloned();
        match previous {
            Some(from) if &from != task.base() => self.move_between_bases(task, &from)?,
            _ => find(&self.bases, task.base())?.save(task)?,
        }
        self.located()
            .insert(task.id().clone(), task.base().clone());
        Ok(())
    }

    fn remove(&self, id: &TaskId) -> Result<(), RepositoryError> {
        let Some(base) = self.located().remove(id) else {
            return Ok(()); // inconnue : rien à effacer
        };
        find(&self.bases, &base)?.remove(id)
    }
}
