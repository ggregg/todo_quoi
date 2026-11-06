//! Le faux dépôt en mémoire respecte lui aussi le contrat.

// Fichier de tests : un `unwrap` qui échoue fait échouer le test, c'est voulu.
#![allow(clippy::unwrap_used)]

use tq_testing::contract::check_task_repository_contract;
use tq_testing::fakes::InMemoryTaskRepository;

#[test]
fn in_memory_repository_honours_the_contract() {
    check_task_repository_contract(&InMemoryTaskRepository::default());
}
