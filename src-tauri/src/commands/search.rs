//! Commande de recherche (spec §5.6).

use super::run;
use crate::composition::LiveState;
use crate::dto::parse_base_id;
use crate::error::UiError;
use serde::Serialize;
use tauri::State;
use tq_domain::search::{Highlight, SearchHit, Snippet};

/// Portion surlignée, en caractères (`text.slice(start, end)` après `Array.from(text)` côté TS).
#[derive(Debug, Clone, Serialize)]
pub struct HighlightDto {
    /// Premier caractère surligné.
    pub start: usize,
    /// Caractère qui suit le dernier surligné.
    pub end: usize,
}

/// Extrait de note.
#[derive(Debug, Clone, Serialize)]
pub struct SnippetDto {
    /// Texte.
    pub text: String,
    /// Mots trouvés.
    pub highlights: Vec<HighlightDto>,
}

/// Un résultat de recherche.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitDto {
    /// Identifiant de la tâche.
    pub id: String,
    /// Sa base.
    pub base: String,
    /// Titre.
    pub title: String,
    /// Titres des ancêtres, de la racine au parent.
    pub path: Vec<String>,
    /// Terminée ?
    pub done: bool,
    /// Mots trouvés dans le titre.
    pub title_highlights: Vec<HighlightDto>,
    /// Extrait de la note.
    pub snippet: Option<SnippetDto>,
}

impl From<Highlight> for HighlightDto {
    fn from(highlight: Highlight) -> Self {
        Self {
            start: highlight.start,
            end: highlight.end,
        }
    }
}

impl From<Snippet> for SnippetDto {
    fn from(snippet: Snippet) -> Self {
        Self {
            text: snippet.text,
            highlights: snippet
                .highlights
                .into_iter()
                .map(HighlightDto::from)
                .collect(),
        }
    }
}

impl From<SearchHit> for SearchHitDto {
    fn from(hit: SearchHit) -> Self {
        Self {
            id: hit.id.to_string(),
            base: hit.base.to_string(),
            title: hit.title,
            path: hit.path,
            done: hit.done,
            title_highlights: hit
                .title_highlights
                .into_iter()
                .map(HighlightDto::from)
                .collect(),
            snippet: hit.snippet.map(SnippetDto::from),
        }
    }
}

/// Recherche dans les bases affichées. `invoke('search_tasks', { query: 'tag:x', bases: ['pro'] })`
/// (`bases` absent ou vide = toutes).
#[tauri::command]
pub async fn search_tasks(
    state: State<'_, LiveState>,
    query: String,
    bases: Option<Vec<String>>,
) -> Result<Vec<SearchHitDto>, UiError> {
    let bases = bases
        .unwrap_or_default()
        .iter()
        .map(|base| parse_base_id(base))
        .collect::<Result<Vec<_>, _>>()?;
    let use_case = state.current().search_tasks.clone();
    run(move || {
        Ok(use_case
            .execute(&query, bases)
            .into_iter()
            .map(SearchHitDto::from)
            .collect())
    })
    .await
}
