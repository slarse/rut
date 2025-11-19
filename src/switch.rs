use std::path::PathBuf;

use crate::diff;
use crate::object_resolver::ObjectResolver;
use crate::status::ChangeType;
use crate::workspace::Repository;

pub enum WorktreeEdit {
    Add(PathBuf),
    Delete(PathBuf),
    Update(PathBuf),
}

pub fn plan_switch(
    repository: &Repository,
    from_ref: &str,
    to_ref: &str,
) -> crate::Result<Vec<WorktreeEdit>> {
    let mut lhs_object_resolver = ObjectResolver::from_reference(from_ref, repository)?;
    let mut rhs_object_resolver = ObjectResolver::from_reference(to_ref, repository)?;

    let lhs_tree = lhs_object_resolver.find_tree_by_path(&PathBuf::new()).ok();
    let rhs_tree = rhs_object_resolver.find_tree_by_path(&PathBuf::new()).ok();

    let changes = diff::compare_trees(
        lhs_tree,
        rhs_tree,
        PathBuf::new(),
        &mut lhs_object_resolver,
        &mut rhs_object_resolver,
    )?;

    Ok(changes
        .iter()
        .map(|change| match change.change_type {
            ChangeType::Created => WorktreeEdit::Add(change.path.clone()),
            ChangeType::Modified => WorktreeEdit::Update(change.path.clone()),
            ChangeType::Deleted => WorktreeEdit::Delete(change.path.clone()),
        })
        .collect())
}
