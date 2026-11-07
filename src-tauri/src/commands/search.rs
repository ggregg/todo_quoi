//! Commande de recherche (spec §5.6).

use super::run;
use crate::composition::AppState;
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

/// Recherche. `invoke('search_tasks', { query: 'tag:client-x release' })`.
#[tauri::command]
pub async fn search_tasks(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<SearchHitDto>, UiError> {
    let use_case = state.search_tasks.clone();
    run(move || {
        Ok(use_case
            .execute(&query)
            .into_iter()
            .map(SearchHitDto::from)
            .collect())
    })
    .await
}
