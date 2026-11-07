//! Tests de l'héritage et de la mise en forme des listes (spec §5.3).

use super::*;
use crate::rules::test_support::{close, id, stamp, titles, tree};
use crate::search::MediaKind;
use crate::{Tag, TaskTree, Title};

fn no_media(_: &crate::TaskId) -> Vec<MediaKind> {
    Vec::new()
}

fn sorted_by(field: SortField, direction: SortDirection) -> ListView {
    ListView {
        sort: vec![SortKey { field, direction }],
        ..ListView::default()
    }
}

/// Titres des sous-tâches de `parent` affichées avec `view`.
fn shown(tree: &TaskTree, parent: Option<&str>, view: &ListView) -> Vec<String> {
    arrange_children(tree, parent.map(id).as_ref(), view, &no_media)
        .iter()
        .map(|task| tree.get(task).expect("existe").title().to_string())
        .collect()
}

fn customize(tree: &mut TaskTree, label: &str, view: ListView) {
    tree.get_mut(&id(label))
        .expect("existe")
        .customize_view(view, &stamp(3))
        .expect("active");
}

#[test]
fn a_list_inherits_from_its_closest_customized_ancestor() {
    let mut tree = tree(&[("1", None), ("11", Some("1")), ("111", Some("11"))]);
    let root = ListView::default();
    let custom = sorted_by(SortField::Title, SortDirection::Ascending);
    customize(&mut tree, "1", custom.clone());

    assert_eq!(
        effective_view(&tree, None, &root),
        (root.clone(), ViewSource::Own)
    );
    assert_eq!(
        effective_view(&tree, Some(&id("1")), &root),
        (custom.clone(), ViewSource::Own)
    );
    assert_eq!(
        effective_view(&tree, Some(&id("111")), &root),
        (custom, ViewSource::InheritedFrom(id("1")))
    );
}

#[test]
fn without_customized_ancestor_the_root_view_applies() {
    let tree = tree(&[("1", None), ("11", Some("1"))]);
    let root = sorted_by(SortField::CreatedAt, SortDirection::Descending);
    assert_eq!(
        effective_view(&tree, Some(&id("11")), &root),
        (root, ViewSource::Root)
    );
}

#[test]
fn going_back_to_inheritance_removes_the_own_view() {
    let mut tree = tree(&[("1", None)]);
    customize(
        &mut tree,
        "1",
        sorted_by(SortField::Title, SortDirection::Ascending),
    );
    tree.get_mut(&id("1"))
        .expect("existe")
        .inherit_view(&stamp(4))
        .expect("active");
    assert_eq!(
        effective_view(&tree, Some(&id("1")), &ListView::default()).1,
        ViewSource::Root
    );
}

#[test]
fn manual_order_is_the_default() {
    let tree = tree(&[("3", None), ("1", None), ("2", None)]);
    assert_eq!(
        shown(&tree, None, &ListView::default()),
        titles(&tree, None)
    );
}

#[test]
fn sorts_by_title_both_ways_ignoring_case() {
    let mut tree = tree(&[("1", None), ("2", None), ("3", None)]);
    for (label, title) in [("1", "banane"), ("2", "Abricot"), ("3", "cerise")] {
        let task = tree.get_mut(&id(label)).expect("existe");
        task.rename(Title::parse(title).expect("titre"), &stamp(2))
            .expect("active");
    }
    let ascending = sorted_by(SortField::Title, SortDirection::Ascending);
    assert_eq!(
        shown(&tree, None, &ascending),
        ["Abricot", "banane", "cerise"]
    );
    let descending = sorted_by(SortField::Title, SortDirection::Descending);
    assert_eq!(
        shown(&tree, None, &descending),
        ["cerise", "banane", "Abricot"]
    );
}

#[test]
fn sorts_by_progress_then_by_manual_order() {
    // 1 : 1/2 terminée ; 2 : 0/1 ; 3 : sans sous-tâche (0 %) ; 4 : 2/2.
    let mut tree = tree(&[
        ("1", None),
        ("11", Some("1")),
        ("12", Some("1")),
        ("2", None),
        ("21", Some("2")),
        ("3", None),
        ("4", None),
        ("41", Some("4")),
        ("42", Some("4")),
    ]);
    for label in ["11", "41", "42"] {
        close(&mut tree, label);
    }
    let view = sorted_by(SortField::Progress, SortDirection::Descending);
    assert_eq!(shown(&tree, None, &view), ["4", "1", "2", "3"]);
}

#[test]
fn done_tasks_come_first_when_sorting_by_closing_date() {
    let mut tree = tree(&[("1", None), ("2", None)]);
    close(&mut tree, "2");
    let view = sorted_by(SortField::DoneAt, SortDirection::Ascending);
    assert_eq!(shown(&tree, None, &view), ["2", "1"]);
}

#[test]
fn filters_on_status_tags_and_media() {
    let mut tree = tree(&[("1", None), ("2", None), ("3", None)]);
    close(&mut tree, "1");
    let tag = |text: &str| Tag::parse(text).expect("étiquette");
    tree.get_mut(&id("2"))
        .expect("existe")
        .set_tags(vec![tag("client")], &stamp(2))
        .expect("active");

    let mut view = ListView::default();
    view.filter.status = StatusChoice::Open;
    assert_eq!(shown(&tree, None, &view), ["2", "3"]);
    view.filter.tags_excluded = vec![tag("client")];
    assert_eq!(shown(&tree, None, &view), ["3"]);

    let mut view = ListView::default();
    view.filter.tags_included = vec![tag("client")];
    assert_eq!(shown(&tree, None, &view), ["2"]);

    let mut view = ListView::default();
    view.filter.media = MediaChoice::WithMedia;
    let media_of_three = |task: &crate::TaskId| {
        if *task == id("3") {
            vec![MediaKind::Image]
        } else {
            Vec::new()
        }
    };
    let arranged = arrange_children(&tree, None, &view, &media_of_three);
    assert_eq!(arranged, [id("3")]);
}

#[test]
fn progress_counts_direct_active_children() {
    let mut tree = tree(&[
        ("1", None),
        ("11", Some("1")),
        ("12", Some("1")),
        ("121", Some("12")),
    ]);
    close(&mut tree, "11");
    assert_eq!(
        Progress::of(&tree, &id("1")),
        Progress { done: 1, total: 2 }
    );
    assert_eq!(
        Progress::of(&tree, &id("121")),
        Progress { done: 0, total: 0 }
    );
}
