//! Liste des tâches mise en forme, et commandes des vues par liste (spec §5.3).
//!
//! Les DTO de vue sont des miroirs des types de `tq_domain::view`, en camelCase pour
//! TypeScript (`ui/src/domain/view.ts`). Valeurs : `'progress'`, `'createdAt'`, `'asc'`…

use super::run;
use crate::composition::AppState;
use crate::dto::{TaskDto, parse_task_id};
use crate::error::UiError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::State;
use tq_application::AppError;
use tq_application::use_cases::list_tasks::{ListLayout, ListTarget, TaskListing};
use tq_domain::Tag;
use tq_domain::search::MediaKind;
use tq_domain::view::{
    Column, ListView, MediaChoice, SortDirection, SortField, SortKey, StatusChoice, ViewFilter,
    ViewSource,
};

// [rust] `#[derive(Serialize, Deserialize)]` sur une énumération sans données + `rename_all` :
//        `Column::CreatedAt` s'écrit `"createdAt"` en JSON (≈ une union de chaînes TS).
/// Colonne (miroir de [`Column`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColumnDto {
    /// Progression.
    Progress,
    /// Étiquettes.
    Tags,
    /// Icônes.
    Icons,
    /// Création.
    CreatedAt,
    /// Modification.
    UpdatedAt,
    /// Clôture.
    DoneAt,
}

/// Critère de tri (miroir de [`SortField`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortFieldDto {
    /// Ordre manuel.
    Manual,
    /// Titre.
    Title,
    /// Création.
    CreatedAt,
    /// Modification.
    UpdatedAt,
    /// Clôture.
    DoneAt,
    /// Progression.
    Progress,
}

/// Sens du tri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DirectionDto {
    /// Croissant.
    Asc,
    /// Décroissant.
    Desc,
}

/// Un critère de tri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortKeyDto {
    /// Critère.
    pub field: SortFieldDto,
    /// Sens.
    pub direction: DirectionDto,
}

/// Statut affiché.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusDto {
    /// Toutes.
    All,
    /// Ouvertes.
    Open,
    /// Terminées.
    Done,
}

/// Filtre sur les médias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaChoiceDto {
    /// Peu importe.
    Any,
    /// Avec médias.
    WithMedia,
}

/// Filtres.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterDto {
    /// Statut.
    pub status: StatusDto,
    /// Étiquettes obligatoires.
    pub tags_in: Vec<String>,
    /// Étiquettes interdites.
    pub tags_out: Vec<String>,
    /// Médias.
    pub media: MediaChoiceDto,
}

/// Vue d'une liste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListViewDto {
    /// Colonnes après le titre.
    pub columns: Vec<ColumnDto>,
    /// Critères de tri.
    pub sort: Vec<SortKeyDto>,
    /// Filtres.
    pub filter: FilterDto,
}

/// Origine de la vue : `{ kind: 'own' }`, `{ kind: 'inherited', fromId }`, `{ kind: 'root' }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ViewSourceDto {
    /// Vue propre.
    Own,
    /// Héritée d'un ancêtre.
    #[serde(rename_all = "camelCase")]
    Inherited {
        /// L'ancêtre.
        from_id: String,
    },
    /// Héritée de la vue racine.
    Root,
}

/// Mise en forme d'une liste.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLayoutDto {
    /// Tâche dont c'est la liste (`null` = racine).
    pub task_id: Option<String>,
    /// Vue appliquée.
    pub view: ListViewDto,
    /// Origine de la vue.
    pub source: ViewSourceDto,
    /// Sous-tâches affichées, dans l'ordre.
    pub children: Vec<String>,
}

/// Tâches et mise en forme de toutes les listes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListingDto {
    /// Tâches actives.
    pub tasks: Vec<TaskDto>,
    /// Une mise en forme par liste (racine d'abord).
    pub lists: Vec<ListLayoutDto>,
    /// Médias de chaque tâche qui en a : `'image' | 'audio' | 'file'`.
    pub media: BTreeMap<String, Vec<&'static str>>,
}

/// Tâches actives et mise en forme de chaque liste. `invoke('list_tasks')`.
#[tauri::command]
pub async fn list_tasks(state: State<'_, AppState>) -> Result<ListingDto, UiError> {
    let use_case = state.list_tasks.clone();
    run(move || Ok(ListingDto::from(use_case.execute()?))).await
}

/// Personnalise une liste (`taskId` absent ou `null` = racine).
/// `invoke('customize_list_view', { taskId, view })`.
#[tauri::command]
pub async fn customize_list_view(
    state: State<'_, AppState>,
    task_id: Option<String>,
    view: ListViewDto,
) -> Result<(), UiError> {
    let list = match task_id {
        Some(id) => ListTarget::Task(parse_task_id(&id)?),
        None => ListTarget::Root,
    };
    let view = view.into_view()?;
    let use_case = state.customize_list_view.clone();
    run(move || use_case.execute(&list, view)).await
}

/// Revenir à la vue héritée. `invoke('inherit_list_view', { taskId })`.
#[tauri::command]
pub async fn inherit_list_view(state: State<'_, AppState>, task_id: String) -> Result<(), UiError> {
    let id = parse_task_id(&task_id)?;
    let use_case = state.inherit_list_view.clone();
    run(move || use_case.execute(&id)).await
}

// ----- Conversions --------------------------------------------------------------------------

impl From<TaskListing> for ListingDto {
    fn from(listing: TaskListing) -> Self {
        Self {
            tasks: listing.tasks.iter().map(TaskDto::from).collect(),
            lists: listing.lists.into_iter().map(ListLayoutDto::from).collect(),
            media: listing
                .media
                .into_iter()
                .map(|(id, kinds)| (id.to_string(), kinds.into_iter().map(media_name).collect()))
                .collect(),
        }
    }
}

impl From<ListLayout> for ListLayoutDto {
    fn from(layout: ListLayout) -> Self {
        Self {
            task_id: layout.list.task().map(ToString::to_string),
            view: ListViewDto::from(&layout.view),
            source: match layout.source {
                ViewSource::Own => ViewSourceDto::Own,
                ViewSource::InheritedFrom(id) => ViewSourceDto::Inherited {
                    from_id: id.to_string(),
                },
                ViewSource::Root => ViewSourceDto::Root,
            },
            children: layout.children.iter().map(ToString::to_string).collect(),
        }
    }
}

fn invalid_tag(error: tq_domain::DomainError) -> UiError {
    UiError::from(AppError::from(error))
}

fn media_name(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Image => "image",
        MediaKind::Audio => "audio",
        MediaKind::File => "file",
    }
}

impl From<&ListView> for ListViewDto {
    fn from(view: &ListView) -> Self {
        Self {
            columns: view
                .columns
                .iter()
                .map(|column| column_dto(*column))
                .collect(),
            sort: view.sort.iter().map(|key| sort_key_dto(*key)).collect(),
            filter: FilterDto {
                status: match view.filter.status {
                    StatusChoice::All => StatusDto::All,
                    StatusChoice::Open => StatusDto::Open,
                    StatusChoice::Done => StatusDto::Done,
                },
                tags_in: view
                    .filter
                    .tags_included
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                tags_out: view
                    .filter
                    .tags_excluded
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                media: match view.filter.media {
                    MediaChoice::Any => MediaChoiceDto::Any,
                    MediaChoice::WithMedia => MediaChoiceDto::WithMedia,
                },
            },
        }
    }
}

impl ListViewDto {
    /// Vue du domaine.
    ///
    /// # Erreurs
    /// Étiquette invalide dans les filtres.
    pub fn into_view(self) -> Result<ListView, UiError> {
        let tags = |names: Vec<String>| {
            names
                .iter()
                .map(|name| Tag::parse(name))
                .collect::<Result<Vec<_>, _>>()
        };
        let filter = ViewFilter {
            status: match self.filter.status {
                StatusDto::All => StatusChoice::All,
                StatusDto::Open => StatusChoice::Open,
                StatusDto::Done => StatusChoice::Done,
            },
            tags_included: tags(self.filter.tags_in).map_err(invalid_tag)?,
            tags_excluded: tags(self.filter.tags_out).map_err(invalid_tag)?,
            media: match self.filter.media {
                MediaChoiceDto::Any => MediaChoice::Any,
                MediaChoiceDto::WithMedia => MediaChoice::WithMedia,
            },
        };
        Ok(ListView {
            columns: self.columns.into_iter().map(column_from_dto).collect(),
            sort: self.sort.into_iter().map(sort_key_from_dto).collect(),
            filter,
        })
    }
}

/// Correspondance colonne ↔ DTO (une table, lue dans les deux sens).
const COLUMNS: [(Column, ColumnDto); 6] = [
    (Column::Progress, ColumnDto::Progress),
    (Column::Tags, ColumnDto::Tags),
    (Column::Icons, ColumnDto::Icons),
    (Column::CreatedAt, ColumnDto::CreatedAt),
    (Column::UpdatedAt, ColumnDto::UpdatedAt),
    (Column::DoneAt, ColumnDto::DoneAt),
];

/// Correspondance critère de tri ↔ DTO.
const SORT_FIELDS: [(SortField, SortFieldDto); 6] = [
    (SortField::Manual, SortFieldDto::Manual),
    (SortField::Title, SortFieldDto::Title),
    (SortField::CreatedAt, SortFieldDto::CreatedAt),
    (SortField::UpdatedAt, SortFieldDto::UpdatedAt),
    (SortField::DoneAt, SortFieldDto::DoneAt),
    (SortField::Progress, SortFieldDto::Progress),
];

fn column_dto(column: Column) -> ColumnDto {
    COLUMNS
        .iter()
        .find(|(known, _)| *known == column)
        .map_or(ColumnDto::Progress, |(_, dto)| *dto)
}

fn column_from_dto(dto: ColumnDto) -> Column {
    COLUMNS
        .iter()
        .find(|(_, known)| *known == dto)
        .map_or(Column::Progress, |(column, _)| *column)
}

fn sort_key_dto(key: SortKey) -> SortKeyDto {
    SortKeyDto {
        field: SORT_FIELDS
            .iter()
            .find(|(known, _)| *known == key.field)
            .map_or(SortFieldDto::Manual, |(_, dto)| *dto),
        direction: match key.direction {
            SortDirection::Ascending => DirectionDto::Asc,
            SortDirection::Descending => DirectionDto::Desc,
        },
    }
}

fn sort_key_from_dto(dto: SortKeyDto) -> SortKey {
    SortKey {
        field: SORT_FIELDS
            .iter()
            .find(|(_, known)| *known == dto.field)
            .map_or(SortField::Manual, |(field, _)| *field),
        direction: match dto.direction {
            DirectionDto::Asc => SortDirection::Ascending,
            DirectionDto::Desc => SortDirection::Descending,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_goes_to_json_and_back_unchanged() {
        let mut view = ListView {
            columns: vec![Column::DoneAt],
            sort: vec![SortKey {
                field: SortField::CreatedAt,
                direction: SortDirection::Descending,
            }],
            ..ListView::default()
        };
        view.filter.status = StatusChoice::Done;
        view.filter.tags_excluded = vec![Tag::parse("perso").expect("valide")];
        let json = serde_json::to_value(ListViewDto::from(&view)).expect("json");
        assert_eq!(json["columns"][0], "doneAt");
        assert_eq!(json["sort"][0]["direction"], "desc");
        assert_eq!(json["filter"]["tagsOut"][0], "perso");
        let back: ListViewDto = serde_json::from_value(json).expect("relu");
        assert_eq!(back.into_view().expect("valide"), view);
    }

    #[test]
    fn the_view_source_is_a_tagged_union() {
        let json = serde_json::to_value(ViewSourceDto::Inherited {
            from_id: "x".to_string(),
        })
        .expect("json");
        assert_eq!(
            json,
            serde_json::json!({ "kind": "inherited", "fromId": "x" })
        );
    }
}
