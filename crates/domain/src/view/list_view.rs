//! La présentation d'une liste : colonnes, tri, filtres (spec §5.3).

use crate::Tag;

/// Colonne affichable en plus du titre (toujours présent, en premier).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// Progression `n/m` des sous-tâches.
    Progress,
    /// Étiquettes.
    Tags,
    /// Icônes : note, image, audio, pièce jointe.
    Icons,
    /// Date de création.
    CreatedAt,
    /// Date de dernière modification.
    UpdatedAt,
    /// Date de clôture.
    DoneAt,
}

impl Column {
    /// Toutes les colonnes, dans l'ordre proposé par défaut dans l'éditeur de vue.
    pub const ALL: [Column; 6] = [
        Column::Progress,
        Column::Tags,
        Column::Icons,
        Column::CreatedAt,
        Column::UpdatedAt,
        Column::DoneAt,
    ];
}

/// Critère de tri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortField {
    /// Ordre choisi à la main (glisser, `Alt+↑/↓`).
    Manual,
    /// Titre, ordre alphabétique sans tenir compte de la casse.
    Title,
    /// Date de création.
    CreatedAt,
    /// Date de dernière modification.
    UpdatedAt,
    /// Date de clôture (les tâches ouvertes après les terminées).
    DoneAt,
    /// Part des sous-tâches terminées.
    Progress,
}

/// Sens du tri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SortDirection {
    /// Croissant (A → Z, ancien → récent).
    Ascending,
    /// Décroissant.
    Descending,
}

/// Un critère de tri et son sens. Une vue en enchaîne plusieurs (tri multi-critères).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SortKey {
    /// Critère.
    pub field: SortField,
    /// Sens.
    pub direction: SortDirection,
}

/// Quelles tâches montrer selon leur statut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusChoice {
    /// Ouvertes et terminées.
    All,
    /// Ouvertes seulement.
    Open,
    /// Terminées seulement.
    Done,
}

/// Filtre sur la présence de médias dans la note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaChoice {
    /// Peu importe.
    Any,
    /// Seulement les tâches dont la note contient une image, un audio ou une pièce jointe.
    WithMedia,
}

/// Filtres d'une liste. Une tâche écartée l'est avec ses sous-tâches.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewFilter {
    /// Statut.
    pub status: StatusChoice,
    /// Étiquettes obligatoires (toutes).
    pub tags_included: Vec<Tag>,
    /// Étiquettes interdites (aucune).
    pub tags_excluded: Vec<Tag>,
    /// Médias.
    pub media: MediaChoice,
}

impl Default for ViewFilter {
    fn default() -> Self {
        Self {
            status: StatusChoice::All,
            tags_included: Vec::new(),
            tags_excluded: Vec::new(),
            media: MediaChoice::Any,
        }
    }
}

/// Présentation d'une liste de sous-tâches (ou de la liste racine).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListView {
    /// Colonnes affichées après le titre, dans cet ordre (sans doublon).
    pub columns: Vec<Column>,
    /// Critères de tri, du plus important au moins important. Vide = ordre manuel.
    pub sort: Vec<SortKey>,
    /// Filtres.
    pub filter: ViewFilter,
}

impl Default for ListView {
    /// Vue d'origine : progression, étiquettes, icônes ; ordre manuel ; aucun filtre.
    fn default() -> Self {
        Self {
            columns: vec![Column::Progress, Column::Tags, Column::Icons],
            sort: vec![SortKey {
                field: SortField::Manual,
                direction: SortDirection::Ascending,
            }],
            filter: ViewFilter::default(),
        }
    }
}

impl ListView {
    /// Retire les doublons (colonnes, critères de tri, étiquettes) en gardant la première
    /// occurrence : une vue relue d'un fichier modifié à la main reste cohérente.
    pub fn normalized(mut self) -> Self {
        dedup_keep_first(&mut self.columns);
        let mut seen_fields = Vec::new();
        self.sort.retain(|key| {
            let first = !seen_fields.contains(&key.field);
            seen_fields.push(key.field);
            first
        });
        dedup_keep_first(&mut self.filter.tags_included);
        dedup_keep_first(&mut self.filter.tags_excluded);
        self
    }
}

fn dedup_keep_first<T: PartialEq + Clone>(items: &mut Vec<T>) {
    let mut kept: Vec<T> = Vec::with_capacity(items.len());
    for item in items.drain(..) {
        if !kept.contains(&item) {
            kept.push(item);
        }
    }
    *items = kept;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_removes_duplicates_keeping_the_first() {
        let key = |field, direction| SortKey { field, direction };
        let view = ListView {
            columns: vec![Column::Tags, Column::Progress, Column::Tags],
            sort: vec![
                key(SortField::Title, SortDirection::Descending),
                key(SortField::Title, SortDirection::Ascending),
            ],
            filter: ViewFilter::default(),
        }
        .normalized();
        assert_eq!(view.columns, [Column::Tags, Column::Progress]);
        assert_eq!(
            view.sort,
            [key(SortField::Title, SortDirection::Descending)]
        );
    }
}
