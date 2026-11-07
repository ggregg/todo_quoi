//! L'index en mémoire respecte le contrat commun de `SearchIndex`, et tient la cible de
//! performance de la spécification (§8 : recherche < 100 ms sur 10 000 tâches).

use std::time::{Duration, Instant};
use tq_adapter_memory_index::MemorySearchIndex;
use tq_application::ports::SearchIndex;
use tq_domain::search::{NoteDigest, SearchDocument, SearchQuery};
use tq_testing::TaskBuilder;
use tq_testing::contract::check_search_index_contract;

#[test]
fn memory_index_honours_the_contract() {
    check_search_index_contract(&MemorySearchIndex::new());
}

#[test]
fn ten_thousand_tasks_are_searched_quickly() {
    let index = MemorySearchIndex::new();
    for number in 0..10_000_u32 {
        let task = TaskBuilder::new(u64::from(number), &format!("Tâche numéro {number}")).build();
        let digest = NoteDigest {
            text: format!(
                "Note de la tâche {number} : préparer la livraison du lot {}",
                number % 37
            ),
            media: Vec::new(),
        };
        index.upsert(SearchDocument::new(&task, digest));
    }
    let query = SearchQuery::parse("livraison lot 12");
    let started = Instant::now();
    let hits = index.search(&query, 200);
    let elapsed = started.elapsed();
    assert!(!hits.is_empty());
    // Large marge (build de debug, machine chargée) : la cible réelle est < 10 ms en release.
    assert!(
        elapsed < Duration::from_millis(100),
        "recherche trop lente : {elapsed:?}"
    );
}
