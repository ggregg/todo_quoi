//! Forme **fichier** d'une vue de liste ([`ListView`]), commune au `[view]` d'une tâche (TOML)
//! et à la vue racine de `manifest.json` (JSON) — serde sait écrire les deux.
//!
//! ```toml
//! [view]
//! columns = ["progress", "tags", "updated_at"]
//! sort = [{ field = "manual" }, { field = "title", direction = "desc" }]
//! filter = { status = "open", tags_in = [], tags_out = ["perso"], media = "any" }
//! ```
//!
//! Tous les champs sont facultatifs (valeurs par défaut de [`ListView::default`]). `"title"` est
//! accepté dans `columns` et ignoré (le titre est toujours affiché). Une valeur inconnue rend la
//! vue illisible : l'appelant la conserve alors telle quelle dans le fichier.

use serde::{Deserialize, Serialize};
use tq_domain::Tag;
use tq_domain::view::{
    Column, ListView, MediaChoice, SortDirection, SortField, SortKey, StatusChoice, ViewFilter,
};

/// Vue telle qu'écrite dans un fichier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ViewFile {
    #[serde(default = "default_columns")]
    columns: Vec<String>,
    #[serde(default = "default_sort")]
    sort: Vec<SortKeyFile>,
    #[serde(default)]
    filter: FilterFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SortKeyFile {
    field: String,
    #[serde(default = "ascending", skip_serializing_if = "is_ascending")]
    direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilterFile {
    #[serde(default = "all")]
    status: String,
    #[serde(default)]
    tags_in: Vec<String>,
    #[serde(default)]
    tags_out: Vec<String>,
    #[serde(default = "any")]
    media: String,
}

impl ViewFile {
    /// Vue du domaine ; `None` si une valeur est inconnue ou une étiquette invalide.
    pub(crate) fn to_view(&self) -> Option<ListView> {
        let columns = self
            .columns
            .iter()
            .filter(|name| name.as_str() != "title")
            .map(|name| column_from(name))
            .collect::<Option<Vec<_>>>()?;
        let sort = self
            .sort
            .iter()
            .map(SortKeyFile::to_key)
            .collect::<Option<Vec<_>>>()?;
        Some(
            ListView {
                columns,
                sort,
                filter: self.filter.to_filter()?,
            }
            .normalized(),
        )
    }

    /// Forme fichier d'une vue du domaine.
    pub(crate) fn from_view(view: &ListView) -> Self {
        Self {
            columns: view
                .columns
                .iter()
                .map(|column| column_name(*column).to_string())
                .collect(),
            sort: view.sort.iter().map(SortKeyFile::from_key).collect(),
            filter: FilterFile::from_filter(&view.filter),
        }
    }
}

impl SortKeyFile {
    fn to_key(&self) -> Option<SortKey> {
        let direction = match self.direction.as_str() {
            "asc" => SortDirection::Ascending,
            "desc" => SortDirection::Descending,
            _ => return None,
        };
        Some(SortKey {
            field: sort_field_from(&self.field)?,
            direction,
        })
    }

    fn from_key(key: &SortKey) -> Self {
        let direction = match key.direction {
            SortDirection::Ascending => "asc",
            SortDirection::Descending => "desc",
        };
        Self {
            field: sort_field_name(key.field).to_string(),
            direction: direction.to_string(),
        }
    }
}

impl Default for FilterFile {
    fn default() -> Self {
        Self::from_filter(&ViewFilter::default())
    }
}

impl FilterFile {
    fn to_filter(&self) -> Option<ViewFilter> {
        let status = match self.status.as_str() {
            "all" => StatusChoice::All,
            "open" => StatusChoice::Open,
            "done" => StatusChoice::Done,
            _ => return None,
        };
        let media = match self.media.as_str() {
            "any" => MediaChoice::Any,
            "with_media" => MediaChoice::WithMedia,
            _ => return None,
        };
        let tags = |names: &[String]| {
            names
                .iter()
                .map(|name| Tag::parse(name).ok())
                .collect::<Option<Vec<_>>>()
        };
        Some(ViewFilter {
            status,
            tags_included: tags(&self.tags_in)?,
            tags_excluded: tags(&self.tags_out)?,
            media,
        })
    }

    fn from_filter(filter: &ViewFilter) -> Self {
        let status = match filter.status {
            StatusChoice::All => "all",
            StatusChoice::Open => "open",
            StatusChoice::Done => "done",
        };
        let media = match filter.media {
            MediaChoice::Any => "any",
            MediaChoice::WithMedia => "with_media",
        };
        let names = |tags: &[Tag]| tags.iter().map(Tag::to_string).collect();
        Self {
            status: status.to_string(),
            tags_in: names(&filter.tags_included),
            tags_out: names(&filter.tags_excluded),
            media: media.to_string(),
        }
    }
}

/// Correspondance colonne ↔ nom dans le fichier (une seule table, lue dans les deux sens).
const COLUMN_NAMES: [(Column, &str); 6] = [
    (Column::Progress, "progress"),
    (Column::Tags, "tags"),
    (Column::Icons, "icons"),
    (Column::CreatedAt, "created_at"),
    (Column::UpdatedAt, "updated_at"),
    (Column::DoneAt, "done_at"),
];

/// Correspondance critère de tri ↔ nom dans le fichier.
const SORT_FIELD_NAMES: [(SortField, &str); 6] = [
    (SortField::Manual, "manual"),
    (SortField::Title, "title"),
    (SortField::CreatedAt, "created_at"),
    (SortField::UpdatedAt, "updated_at"),
    (SortField::DoneAt, "done_at"),
    (SortField::Progress, "progress"),
];

fn column_from(name: &str) -> Option<Column> {
    COLUMN_NAMES
        .iter()
        .find(|(_, known)| *known == name)
        .map(|(column, _)| *column)
}

fn column_name(column: Column) -> &'static str {
    COLUMN_NAMES
        .iter()
        .find(|(known, _)| *known == column)
        .map_or("progress", |(_, name)| name)
}

fn sort_field_from(name: &str) -> Option<SortField> {
    SORT_FIELD_NAMES
        .iter()
        .find(|(_, known)| *known == name)
        .map(|(field, _)| *field)
}

fn sort_field_name(field: SortField) -> &'static str {
    SORT_FIELD_NAMES
        .iter()
        .find(|(known, _)| *known == field)
        .map_or("manual", |(_, name)| name)
}

fn default_columns() -> Vec<String> {
    ViewFile::from_view(&ListView::default()).columns
}

fn default_sort() -> Vec<SortKeyFile> {
    ViewFile::from_view(&ListView::default()).sort
}

fn ascending() -> String {
    "asc".to_string()
}

fn is_ascending(direction: &str) -> bool {
    direction == "asc"
}

fn all() -> String {
    "all".to_string()
}

fn any() -> String {
    "any".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(toml_text: &str) -> Option<ListView> {
        toml::from_str::<ViewFile>(toml_text).ok()?.to_view()
    }

    #[test]
    fn the_documented_example_is_read() {
        let view = read(
            r#"
columns = ["title", "progress", "tags", "updated_at"]
sort = [{ field = "manual" }, { field = "title", direction = "desc" }]
filter = { status = "open", tags_in = [], tags_out = ["perso"] }
"#,
        )
        .expect("vue lisible");
        assert_eq!(
            view.columns,
            [Column::Progress, Column::Tags, Column::UpdatedAt]
        );
        assert_eq!(
            view.sort[1],
            SortKey {
                field: SortField::Title,
                direction: SortDirection::Descending
            }
        );
        assert_eq!(view.filter.status, StatusChoice::Open);
        assert_eq!(
            view.filter.tags_excluded,
            [Tag::parse("perso").expect("valide")]
        );
    }

    #[test]
    fn missing_fields_take_default_values() {
        assert_eq!(read(""), Some(ListView::default()));
    }

    #[test]
    fn unknown_values_make_the_view_unreadable() {
        assert_eq!(read(r#"columns = ["colour"]"#), None);
        assert_eq!(read(r#"sort = [{ field = "priority" }]"#), None);
        assert_eq!(read(r#"filter = { status = "later" }"#), None);
    }

    #[test]
    fn writing_then_reading_gives_the_same_view() {
        let mut view = ListView {
            columns: vec![Column::DoneAt, Column::Icons],
            sort: vec![SortKey {
                field: SortField::Progress,
                direction: SortDirection::Descending,
            }],
            ..ListView::default()
        };
        view.filter.media = MediaChoice::WithMedia;
        view.filter.tags_included = vec![Tag::parse("client-x").expect("valide")];
        let text = toml::to_string(&ViewFile::from_view(&view)).expect("écriture");
        assert_eq!(read(&text), Some(view));
    }
}
