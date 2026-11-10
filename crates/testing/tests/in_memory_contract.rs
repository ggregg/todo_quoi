//! Les faux en mémoire respectent eux aussi les contrats.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use tq_testing::contract::{
    check_media_store_contract, check_search_index_contract, check_task_repository_contract,
};
use tq_testing::fakes::{InMemoryMediaStore, InMemorySearchIndex, InMemoryTaskRepository};

#[test]
fn in_memory_repository_honours_the_contract() {
    check_task_repository_contract(&InMemoryTaskRepository::default());
}

#[test]
fn in_memory_search_index_honours_the_contract() {
    check_search_index_contract(&InMemorySearchIndex::default());
}

#[test]
fn in_memory_media_store_honours_the_contract() {
    check_media_store_contract(&InMemoryMediaStore::default());
}
