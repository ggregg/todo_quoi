//! L'ensemble des tâches, vu comme un arbre (une tâche peut contenir des sous-tâches).
//!
//! Sur le disque, la hiérarchie n'est portée que par le champ `parent` de chaque tâche
//! (arborescence plate, ADR-0004). [`TaskTree`] reconstitue les liens parent → enfants et
//! répond aux questions « quels sont les enfants / ancêtres / descendants de… ».
//!
//! Robustesse face à la synchronisation : une tâche dont le parent est introuvable (fichier
//! pas encore synchronisé, supprimé ailleurs…) est affichée **au premier niveau**, jamais perdue.

use crate::{BaseId, DomainError, OrderKey, Task, TaskId};
use std::collections::{HashMap, HashSet};

/// Toutes les tâches, indexées par identifiant.
#[derive(Debug, Clone, Default)]
pub struct TaskTree {
    // [rust] `HashMap<K, V>` ≈ `Map<K, V>` en TS.
    tasks: HashMap<TaskId, Task>,
}

impl TaskTree {
    /// Construit l'arbre.
    ///
    /// # Erreurs
    /// [`DomainError::DuplicateTask`] si deux tâches ont le même identifiant.
    pub fn new(tasks: Vec<Task>) -> Result<Self, DomainError> {
        let mut tree = Self::default();
        for task in tasks {
            tree.insert(task)?;
        }
        Ok(tree)
    }

    /// Ajoute une tâche.
    ///
    /// # Erreurs
    /// [`DomainError::DuplicateTask`] si l'identifiant existe déjà.
    pub fn insert(&mut self, task: Task) -> Result<(), DomainError> {
        if self.tasks.contains_key(task.id()) {
            return Err(DomainError::DuplicateTask(task.id().clone()));
        }
        self.tasks.insert(task.id().clone(), task);
        Ok(())
    }

    /// Lit une tâche (y compris dans la corbeille).
    ///
    /// # Erreurs
    /// [`DomainError::TaskNotFound`].
    pub fn get(&self, id: &TaskId) -> Result<&Task, DomainError> {
        self.tasks
            .get(id)
            .ok_or_else(|| DomainError::TaskNotFound(id.clone()))
    }

    /// Accès en modification à une tâche.
    ///
    /// # Erreurs
    /// [`DomainError::TaskNotFound`].
    // [rust] `&mut` : emprunt exclusif — tant qu'on tient cette référence, personne d'autre
    //        ne peut lire ni modifier l'arbre (c'est ce qui rend les accès concurrents sûrs).
    pub fn get_mut(&mut self, id: &TaskId) -> Result<&mut Task, DomainError> {
        self.tasks
            .get_mut(id)
            .ok_or_else(|| DomainError::TaskNotFound(id.clone()))
    }

    /// Toutes les tâches, dans un ordre quelconque (y compris la corbeille).
    pub fn iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    /// Nombre total de tâches (y compris la corbeille).
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Vrai s'il n'y a aucune tâche.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Parent **effectif** : le parent déclaré s'il existe, sinon `None` (premier niveau).
    // [rust] On renvoie une copie (`TaskId`) plutôt qu'une référence (`&TaskId`) : la référence
    //        viendrait de `task` ET serait vérifiée dans `self`, ce qui obligerait à annoter les
    //        durées de vie. Une copie d'identifiant coûte quelques octets : la simplicité gagne.
    pub fn effective_parent(&self, task: &Task) -> Option<TaskId> {
        // Un parent rangé dans une autre base (fichier déplacé à la main) ne compte pas :
        // une tâche et son parent sont toujours dans la même base (ADR-0009).
        task.parent()
            .filter(|parent| {
                self.tasks
                    .get(*parent)
                    .is_some_and(|found| found.base() == task.base())
            })
            .cloned()
    }

    /// Tâches actives de premier niveau d'une base, dans l'ordre d'affichage.
    pub fn roots_in(&self, base: &BaseId) -> Vec<&Task> {
        self.children(None)
            .into_iter()
            .filter(|task| task.base() == base)
            .collect()
    }

    /// Sœurs d'une tâche, elle comprise : les enfants de son parent ou, au premier niveau, les
    /// tâches de premier niveau **de sa base**.
    pub fn siblings(&self, task: &Task) -> Vec<&Task> {
        match self.effective_parent(task) {
            Some(parent) => self.children(Some(&parent)),
            None => self.roots_in(task.base()),
        }
    }

    /// Enfants actifs (hors corbeille) d'un parent (`None` = premier niveau), dans l'ordre
    /// d'affichage. Égalité de clés (créations simultanées sur deux appareils) : départage
    /// par identifiant, pour un ordre stable.
    pub fn children(&self, parent: Option<&TaskId>) -> Vec<&Task> {
        let mut children: Vec<&Task> = self
            .tasks
            .values()
            .filter(|task| !task.is_deleted() && self.effective_parent(task).as_ref() == parent)
            .collect();
        children.sort_by(|a, b| a.order().cmp(b.order()).then_with(|| a.id().cmp(b.id())));
        children
    }

    /// Ancêtres d'une tâche, du parent direct vers la racine.
    ///
    /// # Erreurs
    /// [`DomainError::TaskNotFound`] si la tâche n'existe pas.
    pub fn ancestors(&self, id: &TaskId) -> Result<Vec<&Task>, DomainError> {
        let mut ancestors = Vec::new();
        let mut visited = HashSet::from([id.clone()]);
        let mut current = self.get(id)?;
        // Le garde-fou `visited` protège d'une boucle parent ↔ enfant venue d'un fichier corrompu.
        while let Some(parent_id) = self.effective_parent(current) {
            if !visited.insert(parent_id.clone()) {
                break;
            }
            current = self.get(&parent_id)?;
            ancestors.push(current);
        }
        Ok(ancestors)
    }

    /// Descendants actifs (hors corbeille), en profondeur.
    pub fn descendants(&self, id: &TaskId) -> Vec<&Task> {
        let mut found = Vec::new();
        let mut pending = vec![id];
        let mut visited = HashSet::from([id.clone()]);
        while let Some(current) = pending.pop() {
            for child in self.children(Some(current)) {
                if visited.insert(child.id().clone()) {
                    found.push(child);
                    pending.push(child.id());
                }
            }
        }
        found
    }

    /// Vrai si `candidate` est `root` lui-même ou l'un de ses descendants (corbeille comprise).
    pub fn is_in_subtree(&self, candidate: &TaskId, root: &TaskId) -> bool {
        let mut current = Some(candidate.clone());
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if &id == root {
                return true;
            }
            current = self
                .tasks
                .get(&id)
                .and_then(|task| self.effective_parent(task));
            if !visited.insert(id) {
                return false;
            }
        }
        false
    }

    /// Clé d'ordre pour ajouter une tâche **à la fin** des enfants de `parent`.
    pub fn order_at_end(&self, parent: Option<&TaskId>) -> OrderKey {
        let children = self.children(parent);
        // `between(Some(last), None)` ne peut pas échouer (pas de borne haute).
        OrderKey::between(children.last().map(|task| task.order()), None)
            .unwrap_or_else(|_| OrderKey::first())
    }

    /// Rend toutes les tâches (pour les enregistrer).
    pub fn into_tasks(self) -> Vec<Task> {
        self.tasks.into_values().collect()
    }
}
