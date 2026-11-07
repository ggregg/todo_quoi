//! Règles de la recherche : quelles fiches correspondent, dans quel ordre, avec quel extrait.
//!
//! [`search_documents`] est l'algorithme complet ; un index (port `SearchIndex`) n'a qu'à
//! conserver les fiches et l'appeler. Parcours linéaire : quelques millisecondes pour
//! 10 000 tâches (archi §6), sans structure d'index à maintenir.

use super::document::SearchDocument;
use super::fold::fold_with_origins;
use super::query::{SearchQuery, StatusFilter};
use crate::TaskId;
use std::collections::HashMap;

/// Longueur visée d'un extrait de note, en caractères.
const SNIPPET_LENGTH: usize = 90;
/// Contexte conservé avant le premier mot trouvé, en caractères.
const SNIPPET_LEAD: usize = 25;

/// Portion surlignée d'un texte : de `start` (inclus) à `end` (exclu), **en caractères**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Highlight {
    /// Premier caractère surligné.
    pub start: usize,
    /// Caractère qui suit le dernier surligné.
    pub end: usize,
}

/// Extrait de note autour du premier mot trouvé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    /// Texte de l'extrait (« … » ajoutés s'il est coupé).
    pub text: String,
    /// Mots trouvés dans l'extrait.
    pub highlights: Vec<Highlight>,
}

/// Un résultat : la tâche, son chemin et de quoi surligner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    /// Tâche trouvée.
    pub id: TaskId,
    /// Titre.
    pub title: String,
    /// Titres des ancêtres, de la racine au parent (fil d'Ariane).
    pub path: Vec<String>,
    /// Statut.
    pub done: bool,
    /// Mots trouvés dans le titre.
    pub title_highlights: Vec<Highlight>,
    /// Extrait de la note, si un mot y a été trouvé (ou début de la note sans mot cherché).
    pub snippet: Option<Snippet>,
}

/// Cherche dans `documents` ; au plus `limit` résultats, titres trouvés d'abord, puis par titre.
pub fn search_documents(
    documents: &HashMap<TaskId, SearchDocument>,
    query: &SearchQuery,
    limit: usize,
) -> Vec<SearchHit> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut found: Vec<(&SearchDocument, Vec<&SearchDocument>)> = documents
        .values()
        .map(|document| (document, ancestors(documents, document)))
        .filter(|(document, ancestors)| matches(query, document, ancestors))
        .collect();
    // Titres qui contiennent tous les mots d'abord (`false` < `true`), puis ordre alphabétique.
    found.sort_by(|(a, _), (b, _)| {
        let rank = |document: &SearchDocument| !title_contains_all(query, document);
        (rank(a), &a.folded_title).cmp(&(rank(b), &b.folded_title))
    });
    found
        .into_iter()
        .take(limit)
        .map(|(document, ancestors)| describe(query, document, &ancestors))
        .collect()
}

/// La fiche respecte-t-elle **tous** les critères de la requête ?
pub fn matches(
    query: &SearchQuery,
    document: &SearchDocument,
    ancestors: &[&SearchDocument],
) -> bool {
    status_matches(query.status, document.done)
        && query
            .tags_included
            .iter()
            .all(|tag| document.folded_tags.contains(tag))
        && !query
            .tags_excluded
            .iter()
            .any(|tag| document.folded_tags.contains(tag))
        && query.media.iter().all(|kind| document.media.contains(kind))
        && within_matches(query.within.as_deref(), ancestors)
        && query.terms.iter().all(|term| contains_term(document, term))
}

fn status_matches(filter: Option<StatusFilter>, done: bool) -> bool {
    match filter {
        None => true,
        Some(StatusFilter::Open) => !done,
        Some(StatusFilter::Done) => done,
    }
}

fn within_matches(within: Option<&str>, ancestors: &[&SearchDocument]) -> bool {
    within.is_none_or(|title| {
        ancestors
            .iter()
            .any(|ancestor| ancestor.folded_title.contains(title))
    })
}

fn contains_term(document: &SearchDocument, term: &str) -> bool {
    document.folded_title.contains(term)
        || document.folded_note.contains(term)
        || document.folded_tags.iter().any(|tag| tag.contains(term))
}

fn title_contains_all(query: &SearchQuery, document: &SearchDocument) -> bool {
    !query.terms.is_empty()
        && query
            .terms
            .iter()
            .all(|term| document.folded_title.contains(term))
}

/// Ancêtres de la racine au parent. S'arrête sur un parent absent (corbeille) ou un cycle.
// [rust] `'d` : les fiches renvoyées sont empruntées à `documents` (et non copiées) ; elles ne
//        peuvent pas lui survivre. Exception documentée au sous-ensemble (guide 03 §9).
fn ancestors<'d>(
    documents: &'d HashMap<TaskId, SearchDocument>,
    document: &SearchDocument,
) -> Vec<&'d SearchDocument> {
    let mut chain = Vec::new();
    let mut next = document.parent.as_ref();
    while let Some(parent) = next.and_then(|id| documents.get(id)) {
        if chain.len() > documents.len() {
            break; // cycle (données corrompues) : on n'insiste pas
        }
        chain.push(parent);
        next = parent.parent.as_ref();
    }
    chain.reverse();
    chain
}

fn describe(
    query: &SearchQuery,
    document: &SearchDocument,
    ancestors: &[&SearchDocument],
) -> SearchHit {
    SearchHit {
        id: document.id.clone(),
        title: document.title.clone(),
        path: ancestors
            .iter()
            .map(|ancestor| ancestor.title.clone())
            .collect(),
        done: document.done,
        title_highlights: highlights(&document.title, &query.terms),
        snippet: snippet(document, &query.terms),
    }
}

/// Positions (en caractères **du texte d'origine**) de chaque mot trouvé dans `original`,
/// triées et fusionnées quand elles se chevauchent. Les mots sont déjà normalisés.
pub fn highlights(original: &str, terms: &[String]) -> Vec<Highlight> {
    let (folded, origins) = fold_with_origins(original);
    let mut found: Vec<Highlight> = terms
        .iter()
        .filter(|term| !term.is_empty())
        .flat_map(|term| folded.match_indices(term.as_str()))
        .filter_map(|(byte, term)| {
            let first = folded[..byte].chars().count();
            let last = first + term.chars().count() - 1;
            Some(Highlight {
                start: *origins.get(first)?,
                end: *origins.get(last)? + 1,
            })
        })
        .collect();
    found.sort_by_key(|highlight| highlight.start);
    merge_overlapping(found)
}

fn merge_overlapping(sorted: Vec<Highlight>) -> Vec<Highlight> {
    let mut merged: Vec<Highlight> = Vec::new();
    for highlight in sorted {
        match merged.last_mut() {
            Some(last) if highlight.start <= last.end => last.end = last.end.max(highlight.end),
            _ => merged.push(highlight),
        }
    }
    merged
}

/// Extrait de la note autour du premier mot trouvé ; début de la note si aucun mot n'est cherché.
fn snippet(document: &SearchDocument, terms: &[String]) -> Option<Snippet> {
    if document.note_text.trim().is_empty() {
        return None;
    }
    let found = highlights(&document.note_text, terms);
    let first = match (found.first(), terms.is_empty()) {
        (Some(first), _) => first.start,
        (None, true) => 0,
        (None, false) => return None,
    };
    Some(cut_snippet(&document.note_text, first, &found))
}

fn cut_snippet(text: &str, first: usize, found: &[Highlight]) -> Snippet {
    let characters: Vec<char> = text.chars().collect();
    let start = first.saturating_sub(SNIPPET_LEAD);
    let end = (start + SNIPPET_LENGTH).min(characters.len());
    let prefix = if start > 0 { "…" } else { "" };
    let suffix = if end < characters.len() { "…" } else { "" };
    let body: String = characters[start..end]
        .iter()
        .map(|&c| if c == '\n' { ' ' } else { c })
        .collect();
    let shift = prefix.chars().count();
    let highlights = found
        .iter()
        .filter(|highlight| highlight.start >= start && highlight.end <= end)
        .map(|highlight| Highlight {
            start: highlight.start - start + shift,
            end: highlight.end - start + shift,
        })
        .collect();
    Snippet {
        text: format!("{prefix}{body}{suffix}"),
        highlights,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Timestamp;
    use crate::search::MediaKind;
    use crate::search::fold_text;

    fn document(id: &str, parent: Option<&str>, title: &str, note: &str) -> SearchDocument {
        SearchDocument {
            id: task_id(id),
            parent: parent.map(task_id),
            title: title.to_string(),
            folded_title: fold_text(title),
            folded_tags: Vec::new(),
            done: false,
            media: Vec::new(),
            note_text: note.to_string(),
            folded_note: fold_text(note),
            version: Timestamp::from_millis(0),
        }
    }

    fn task_id(suffix: &str) -> TaskId {
        TaskId::parse(&format!("0195f3c2-8e4a-7c1d-9b2f-3a6e5d4c7b{suffix:0>2}"))
            .expect("identifiant de test")
    }

    fn index(documents: Vec<SearchDocument>) -> HashMap<TaskId, SearchDocument> {
        documents
            .into_iter()
            .map(|document| (document.id.clone(), document))
            .collect()
    }

    fn titles(hits: &[SearchHit]) -> Vec<&str> {
        hits.iter().map(|hit| hit.title.as_str()).collect()
    }

    #[test]
    fn every_term_must_appear_ignoring_case_and_accents() {
        let documents = index(vec![
            document("01", None, "Préparer la release", ""),
            document("02", None, "Release notes", "rédiger le résumé"),
            document("03", None, "Autre", "rien"),
        ]);
        let hits = search_documents(&documents, &SearchQuery::parse("RELEASE resume"), 10);
        assert_eq!(titles(&hits), ["Release notes"]);
    }

    #[test]
    fn title_matches_come_first() {
        let documents = index(vec![
            document("01", None, "Zèbre", "un budget"),
            document("02", None, "Budget 2027", ""),
        ]);
        let hits = search_documents(&documents, &SearchQuery::parse("budget"), 10);
        assert_eq!(titles(&hits), ["Budget 2027", "Zèbre"]);
    }

    #[test]
    fn filters_on_status_tags_media_and_subtree() {
        let mut done = document("02", Some("01"), "Lot 1", "");
        done.done = true;
        done.folded_tags = vec!["client-x".to_string()];
        done.media = vec![MediaKind::Audio];
        let documents = index(vec![
            document("01", None, "Projet A", ""),
            done,
            document("03", None, "Lot 2", ""),
        ]);
        let search = |text: &str| {
            titles(&search_documents(&documents, &SearchQuery::parse(text), 10))
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(search(r#"lot in:"projet a""#), ["Lot 1"]);
        assert_eq!(search("lot is:open"), ["Lot 2"]);
        assert_eq!(search("tag:client-x has:audio"), ["Lot 1"]);
        assert_eq!(search("lot -tag:client-x"), ["Lot 2"]);
        assert!(search("has:image").is_empty());
    }

    #[test]
    fn hit_carries_path_and_highlights() {
        let documents = index(vec![
            document("01", None, "Projet A", ""),
            document("02", Some("01"), "Été", "Il faut prévoir l'été prochain."),
        ]);
        let hits = search_documents(&documents, &SearchQuery::parse("ete"), 10);
        assert_eq!(hits[0].path, ["Projet A"]);
        assert_eq!(hits[0].title_highlights, [Highlight { start: 0, end: 3 }]);
        let snippet = hits[0].snippet.as_ref().expect("extrait");
        let highlighted: String = snippet
            .text
            .chars()
            .skip(snippet.highlights[0].start)
            .take(3)
            .collect();
        assert_eq!(highlighted, "été");
    }

    #[test]
    fn long_notes_are_cut_around_the_first_term() {
        let note = format!("{} mot-clé {}", "a".repeat(200), "b".repeat(200));
        let documents = index(vec![document("01", None, "Titre", &note)]);
        let snippet = search_documents(&documents, &SearchQuery::parse("mot-cle"), 10)[0]
            .snippet
            .clone()
            .expect("extrait");
        assert!(snippet.text.starts_with('…') && snippet.text.ends_with('…'));
        let highlight = snippet.highlights[0];
        let word: String = snippet
            .text
            .chars()
            .skip(highlight.start)
            .take(highlight.end - highlight.start)
            .collect();
        assert_eq!(word, "mot-clé");
    }

    #[test]
    fn highlights_follow_expanded_ligatures() {
        let terms = vec!["oeufs".to_string()];
        assert_eq!(
            highlights("Des œufs", &terms),
            [Highlight { start: 4, end: 8 }]
        );
    }

    #[test]
    fn overlapping_highlights_are_merged() {
        let terms = vec!["abc".to_string(), "bcd".to_string()];
        assert_eq!(
            highlights("xabcde", &terms),
            [Highlight { start: 1, end: 5 }]
        );
    }

    #[test]
    fn empty_query_finds_nothing_and_limit_is_respected() {
        let documents = index(
            (1..=5)
                .map(|n| document(&n.to_string(), None, "Tâche", ""))
                .collect(),
        );
        assert!(search_documents(&documents, &SearchQuery::parse(""), 10).is_empty());
        assert_eq!(
            search_documents(&documents, &SearchQuery::parse("tache"), 3).len(),
            3
        );
    }

    #[test]
    fn a_parent_cycle_does_not_loop_forever() {
        let documents = index(vec![
            document("01", Some("02"), "A", ""),
            document("02", Some("01"), "B", ""),
        ]);
        assert_eq!(
            search_documents(&documents, &SearchQuery::parse("a"), 10).len(),
            1
        );
    }
}
