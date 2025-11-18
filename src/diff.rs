use std::{
    fmt::{Debug, Display},
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use crate::{
    index::{FileMode, Index, IndexEntry},
    object_resolver::ObjectResolver,
    objects::{Blob, GitObject, Tree, TreeEntry},
    output::{Color, OutputWriter},
    status,
    workspace::Repository,
};

const MAX_DIFF_CONTEXT_LINES: usize = 3;

#[derive(Default, Builder, Debug)]
pub struct Options {
    pub cached: bool,
}

pub fn diff_repository(
    repository: &Repository,
    options: &Options,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    if options.cached {
        diff_repository_cached(repository, writer)
    } else {
        diff_repository_default(repository, writer)
    }
}

fn diff_repository_cached(
    repository: &Repository,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    let mut index = repository.load_index()?;
    let path_to_committed_id = status::resolve_committed_paths_and_ids(repository)?;
    let files_with_staged_changes = status::resolve_files_with_staged_changes(
        &path_to_committed_id,
        repository,
        index.as_mut(),
    )?;

    let mut object_cache = ObjectResolver::from_head_commit(repository)?;

    for file in files_with_staged_changes {
        let relative_path = repository.worktree().relativize_path(file);
        let staged_blob_id = &index.as_mut().get(&relative_path).unwrap().object_id;
        let staged_blob = repository.database.load_blob(staged_blob_id)?;
        let committed_blob = object_cache.find_blob_by_path(&relative_path).ok();
        diff_blobs(
            committed_blob.as_ref(),
            Some(&staged_blob),
            &relative_path,
            writer,
        )?;
    }

    Ok(())
}

fn diff_repository_default(
    repository: &Repository,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    let mut index = repository.load_index()?;
    let path_to_committed_id = status::resolve_committed_paths_and_ids(repository)?;

    let tracked_paths =
        status::resolve_tracked_paths(&path_to_committed_id, repository.worktree(), index.as_mut());
    let mut unstaged_changes =
        status::resolve_unstaged_changes(&tracked_paths, repository, index.as_mut());
    unstaged_changes.sort_by(|a, b| a.path.cmp(&b.path));

    for change in unstaged_changes {
        diff_unstaged_change(index.as_mut(), &change, repository, writer)?;
    }

    Ok(())
}

fn diff_unstaged_change(
    index: &mut Index,
    change: &status::Change,
    repository: &Repository,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    let a_index_entry = index.get(&change.path).unwrap();
    let (a_lines, a_oid) = read_blob_from_index_entry(a_index_entry, repository)?;
    let a_lines_ref = a_lines.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

    let (b_lines, b_oid) = read_blob_from_worktree(change, repository)?;
    let b_lines_ref = b_lines.iter().map(|s| s.as_str()).collect::<Vec<&str>>();

    diff_content(
        &change.path,
        &a_lines_ref,
        a_oid,
        &b_lines_ref,
        b_oid,
        writer,
    )?;

    Ok(())
}

fn read_blob_from_index_entry(
    index_entry: &IndexEntry,
    repository: &Repository,
) -> crate::Result<(Vec<String>, Option<String>)> {
    let blob = repository.database.load_blob(&index_entry.object_id)?;
    let content = String::from_utf8(blob.content().to_vec()).ok().unwrap();
    let lines: Vec<String> = content.split('\n').map(|s| s.to_owned()).collect();
    let object_id = Some(index_entry.object_id.to_short_string());
    Ok((lines, object_id))
}

fn read_blob_from_worktree(
    change: &status::Change,
    repository: &Repository,
) -> crate::Result<(Vec<String>, Option<String>)> {
    let (b_lines, b_oid) = match change.change_type {
        status::ChangeType::Deleted => (vec![], None),
        _ => {
            let b_raw = fs::read(repository.worktree().root().join(&change.path))?;
            let b = String::from_utf8(b_raw.clone()).unwrap();
            let b_blob = Blob::new(b_raw);
            let b_lines = b.split('\n').map(|s| s.to_owned()).collect::<Vec<String>>();
            let b_oid = Some(b_blob.short_id_as_string());
            (b_lines, b_oid)
        }
    };
    Ok((b_lines, b_oid))
}

fn diff_blobs(
    committed_blob: Option<&Blob>,
    staged_blob: Option<&Blob>,
    relative_path: &Path,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    let empty_string = || "".to_string();
    let committed_content = committed_blob
        .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
        .unwrap_or_else(empty_string);
    let staged_content = staged_blob
        .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
        .unwrap_or_else(empty_string);

    let committed_lines = committed_content.lines().collect::<Vec<_>>();
    let staged_lines = staged_content.lines().collect::<Vec<_>>();

    let edit_script = edit_script(&committed_lines, &staged_lines);
    let chunks = chunk_edit_script(&edit_script, MAX_DIFF_CONTEXT_LINES);

    write_header(
        relative_path,
        committed_blob.map(|blob| blob.short_id_as_string()),
        staged_blob.map(|blob| blob.short_id_as_string()),
        writer,
    )?;

    write_chunks(&chunks, writer)?;

    Ok(())
}

fn diff_content<T: AsRef<str> + Eq>(
    relative_path: &Path,
    a_lines: &[T],
    a_oid: Option<String>,
    b_lines: &[T],
    b_oid: Option<String>,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    // TODO can this be optimized away somehow? Perhaps pass in iterators instead of materialized
    // vectors/slices?
    let a_lines: Vec<&str> = a_lines.iter().map(|line| line.as_ref()).collect();
    let b_lines: Vec<&str> = b_lines.iter().map(|line| line.as_ref()).collect();

    let edit_script = edit_script(&a_lines, &b_lines);
    let chunks = chunk_edit_script(&edit_script, MAX_DIFF_CONTEXT_LINES);

    write_header(relative_path, a_oid, b_oid, writer)?;
    write_chunks(&chunks, writer)?;

    Ok(())
}

fn write_chunks<T: AsRef<str> + Eq>(
    chunks: &Vec<Chunk<T>>,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    for chunk in chunks {
        write_chunk_header(chunk, writer)?;
        for edit in &chunk.edits {
            match edit.kind {
                EditKind::Equal => {
                    writer.writeln(format!(" {}", edit.content.as_ref()))?;
                }
                EditKind::Deletion => {
                    writer.set_color(Color::Red)?;
                    writer.writeln(format!("-{}", edit.content.as_ref()))?;
                    writer.reset_formatting()?;
                }
                EditKind::Addition => {
                    writer.set_color(Color::Green)?;
                    writer.writeln(format!("+{}", edit.content.as_ref()))?;
                    writer.reset_formatting()?;
                }
            }
        }
    }

    Ok(())
}

fn write_chunk_header<'a, S: Eq>(
    chunk: &Chunk<S>,
    writer: &'a mut dyn OutputWriter,
) -> io::Result<&'a mut dyn OutputWriter> {
    let a_size = chunk.a_end - chunk.a_start;
    let b_size = chunk.b_end - chunk.b_start;

    writer
        .set_color(Color::Cyan)?
        .write(String::from("@@"))?
        .write(format!(" -{}", chunk.a_start))?
        .write(if a_size != 1 {
            format!(",{} ", a_size)
        } else {
            String::from(" ")
        })?
        .write(format!("+{}", chunk.b_start))?
        .write(if b_size != 1 {
            format!(",{} ", b_size)
        } else {
            String::from(" ")
        })?
        .write(String::from("@@"))?
        .reset_formatting()?;

    writer.linefeed()?;

    Ok(writer)
}

fn write_header<'a>(
    path: &Path,
    a_oid: Option<String>,
    b_oid: Option<String>,
    writer: &'a mut dyn OutputWriter,
) -> io::Result<&'a mut dyn OutputWriter> {
    let a_path = a_oid
        .as_ref()
        .map(|_| format!("a/{}", path.display()))
        .unwrap_or_else(|| "/dev/null".to_string());
    let b_path = b_oid
        .as_ref()
        .map(|_| format!("b/{}", path.display()))
        .unwrap_or_else(|| "/dev/null".to_string());

    writer
        .writeln(format!(
            "diff --git a/{} b/{}",
            path.display(),
            path.display()
        ))?
        .writeln(format!(
            "index {}..{}",
            // FIXME don't hard-code short-form length here
            &a_oid.unwrap_or_else(|| "0000000".to_string())[..=6],
            &b_oid.unwrap_or_else(|| "0000000".to_string())[..=6]
        ))?
        .writeln(format!("--- {}", a_path))?
        .writeln(format!("+++ {}", b_path))
}

#[derive(Debug, PartialEq, Eq)]
struct Chunk<'a, S: Eq> {
    edits: Vec<&'a Edit<S>>,
    a_start: usize,
    a_end: usize,
    b_start: usize,
    b_end: usize,
}

impl<'a, S: Eq> Chunk<'a, S> {
    fn new(edits: Vec<&'a Edit<S>>) -> Self {
        let mut a_start = None;
        let mut a_end = None;
        let mut b_start = None;
        let mut b_end = None;

        for edit in edits.iter() {
            match edit.kind {
                EditKind::Equal => {
                    if a_start.is_none() {
                        a_start = edit.a_position;
                    }
                    if b_start.is_none() {
                        b_start = edit.b_position;
                    }
                    a_end = edit.a_position;
                    b_end = edit.b_position;
                }
                EditKind::Deletion => {
                    if a_start.is_none() {
                        a_start = edit.a_position;
                    }
                    a_end = edit.a_position;
                }
                EditKind::Addition => {
                    if b_start.is_none() {
                        b_start = edit.b_position;
                    }
                    b_end = edit.b_position;
                }
            }
        }

        Chunk {
            edits,
            // Note: Add 1 to make 1-indexed
            a_start: a_start.map(|x| x + 1).unwrap_or(0),
            // Note: Add 1 to make 1-indexed and another 1 to make range exclusive in end
            a_end: a_end.map(|x| x + 2).unwrap_or(0),
            b_start: b_start.map(|x| x + 1).unwrap_or(0),
            b_end: b_end.map(|x| x + 2).unwrap_or(0),
        }
    }
}

fn chunk_edit_script<'a, T: AsRef<str> + Eq>(
    edit_script: &'a [Edit<T>],
    context_size: usize,
) -> Vec<Chunk<'a, T>> {
    let mut chunks: Vec<Chunk<T>> = vec![];
    let mut chunk_content: Vec<&Edit<T>> = vec![];
    let mut context: Vec<&Edit<T>> = vec![];

    let mut last_mutating_edit_idx = 0;

    for (i, edit) in edit_script.iter().enumerate() {
        match edit.kind {
            EditKind::Equal => {
                if i - last_mutating_edit_idx > context_size && !chunk_content.is_empty() {
                    chunk_content.append(&mut context);
                    chunk_content.append(&mut context);
                    chunks.push(Chunk::new(chunk_content));
                    chunk_content = vec![];
                }

                if should_show(edit, i, edit_script.len()) {
                    context.push(edit);
                }
            }
            EditKind::Deletion => {
                last_mutating_edit_idx = i;
                drain_context_into_chunk(&mut context, &mut chunk_content, context_size);

                if should_show(edit, i, edit_script.len()) {
                    chunk_content.push(edit);
                }
            }
            EditKind::Addition => {
                last_mutating_edit_idx = i;
                drain_context_into_chunk(&mut context, &mut chunk_content, context_size);
                chunk_content.push(edit);
            }
        }
    }

    if !chunk_content.is_empty() {
        drain_context_into_chunk(&mut context, &mut chunk_content, context_size);
        chunks.push(Chunk::new(chunk_content));
    }

    chunks
}

fn should_show<T: AsRef<str> + Eq>(
    edit: &Edit<T>,
    position: usize,
    edit_script_size: usize,
) -> bool {
    if position < edit_script_size - 1 {
        true
    } else {
        !edit.content.as_ref().is_empty()
    }
}

fn drain_context_into_chunk<'b, 'a: 'b, S: Eq>(
    context: &mut Vec<&'a Edit<S>>,
    chunk_content: &mut Vec<&'b Edit<S>>,
    context_size: usize,
) {
    let context_to_skip = if context.len() > context_size {
        context.len() - context_size
    } else {
        0
    };
    chunk_content.extend(context.drain(..).skip(context_to_skip));
}

/// Computes a diff between two arbitrary sequences. The typical thing to use would be two lists of
/// strings, where each element represents a line.
///
/// ```
/// use rut::diff;
///
/// let a = "First line\nSecond line\nThird line".split('\n').collect::<Vec<&str>>();
/// let b = "Second line\nThird line\nFourth line".split('\n').collect::<Vec<&str>>();
///
/// let diff = diff::diff(&a, &b);
///
/// assert_eq!(diff, "-First line\n Second line\n Third line\n+Fourth line\n");
/// ```
pub fn diff<S: Eq + Copy + Display>(a: &[S], b: &[S]) -> String {
    let edit_script = edit_script(a, b);
    let mut result = String::new();

    for edit in edit_script {
        match edit.kind {
            EditKind::Equal => {
                result.push_str(&format!(" {}", edit.content));
            }
            EditKind::Deletion => {
                result.push_str(&format!("-{}", edit.content));
            }
            EditKind::Addition => {
                result.push_str(&format!("+{}", edit.content));
            }
        }
        result.push('\n');
    }
    result
}

/// Computes an edit script between two arbitrary sequences.
///
/// Example:
/// ```
/// use rut::diff;
/// use rut::diff::{Edit, EditKind};
///
/// let a = "ABC".chars().collect::<Vec<char>>();
/// let b = "BBD".chars().collect::<Vec<char>>();
///
/// let expected_edits = vec![
///     Edit::deletion('A', 0),
///     Edit::equal('B', 1, 0),
///     Edit::deletion('C', 2),
///     Edit::addition('B', 1),
///     Edit::addition('D', 2),
/// ];
///
/// let edit_script = diff::edit_script(&a, &b);
///
/// assert_eq!(edit_script, expected_edits);
/// ```
pub fn edit_script<S: Eq + Copy>(a: &[S], b: &[S]) -> Vec<Edit<S>> {
    let (final_k_value, edit_path_graph) = compute_edit_path_graph(a, b);
    let reversed_edit_trace = trace_edit_points(final_k_value, edit_path_graph);
    compute_edit_script(a, b, &reversed_edit_trace)
}

fn compute_edit_path_graph<S: Eq>(a: &[S], b: &[S]) -> (i32, Vec<Vec<usize>>) {
    let max_depth = a.len() + b.len();

    let mut v = vec![0; 2 * max_depth + 1];
    let mut trace = vec![];

    for d in 0..(v.len() as i32) {
        for k in (-d..d + 1).step_by(2) {
            let mut x = if k == -d || (k != d && get(&v, k - 1) < get(&v, k + 1)) {
                *get(&v, k + 1)
            } else {
                *get(&v, k - 1) + 1
            };

            let mut y = (x as i32 - k) as usize;

            while x < a.len() && y < b.len() && a[x] == b[y] {
                x += 1;
                y += 1;
            }

            set(&mut v, k, x);

            if x >= a.len() && y >= b.len() {
                trace.push(v.clone());
                return (k, trace);
            }
        }

        trace.push(v.clone());
    }

    panic!("could not find the shortest path")
}

#[derive(PartialEq, Eq)]
pub struct Edit<S: Eq> {
    content: S,
    a_position: Option<usize>,
    b_position: Option<usize>,
    kind: EditKind,
}

impl<S: Eq + Debug> Debug for Edit<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let to_position_string = |position: Option<usize>| match position {
            Some(position) => position.to_string(),
            None => "_".to_string(),
        };

        write!(
            f,
            "{:?}:{:?}:{:?}:{:?}",
            self.kind,
            self.content,
            to_position_string(self.a_position),
            to_position_string(self.b_position)
        )
    }
}

impl<S: Eq> Edit<S> {
    pub fn addition(content: S, b_position: usize) -> Edit<S> {
        Edit {
            content,
            a_position: None,
            b_position: Some(b_position),
            kind: EditKind::Addition,
        }
    }

    pub fn deletion(content: S, a_position: usize) -> Edit<S> {
        Edit {
            content,
            a_position: Some(a_position),
            b_position: None,
            kind: EditKind::Deletion,
        }
    }

    pub fn equal(content: S, a_position: usize, b_position: usize) -> Edit<S> {
        Edit {
            content,
            a_position: Some(a_position),
            b_position: Some(b_position),
            kind: EditKind::Equal,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum EditKind {
    Addition,
    Deletion,
    Equal,
}

fn trace_edit_points(final_k: i32, trace: Vec<Vec<usize>>) -> Vec<(i32, i32)> {
    let final_v = &trace[trace.len() - 1];
    let mut k = final_k;
    let final_x = *get(final_v, k) as i32;
    let final_y = final_x - final_k;

    let mut edit_points = Vec::with_capacity(trace.len());
    edit_points.push((final_x, final_y));

    for d in (0..trace.len() - 1).rev() {
        let v = &trace[d];
        k = compute_previous_k(k, d as i32, v);
        let x = *get(v, k) as i32;
        let y = x - k;
        edit_points.push((x, y));
    }

    edit_points
}

/// Compute the previous k-value in the edit path graph. This function is optimized for
/// understandability rather than performance, it can easily be compressed into a single condition.
fn compute_previous_k(k: i32, d: i32, v: &[usize]) -> i32 {
    if k == -d {
        // the previous move must have been from a larger k as abs(k) <= d
        k + 1
    } else if k == d {
        // the previous move must have been from a smaller k as abs(k) <= d
        k - 1
    } else if *get(v, k - 1) < *get(v, k + 1) {
        // abs(k) != d and we have a larger x-value at k+1, we choose k for the larger x-value
        k + 1
    } else {
        // abs(k) != d and we have a larger or equal x-value at k-1
        k - 1
    }
}

fn compute_edit_script<S: Eq + Copy>(
    a: &[S],
    b: &[S],
    reversed_edit_points: &[(i32, i32)],
) -> Vec<Edit<S>> {
    let (mut x, mut y) = reversed_edit_points[0];

    let mut edits = vec![];
    for (prev_x, prev_y) in reversed_edit_points.iter().skip(1) {
        while x > *prev_x && y > *prev_y {
            x -= 1;
            y -= 1;
            edits.push(Edit::equal(a[x as usize], x as usize, y as usize));
        }

        if x > *prev_x {
            x -= 1;
            edits.push(Edit::deletion(a[x as usize], x as usize));
        } else {
            y -= 1;
            edits.push(Edit::addition(b[y as usize], y as usize));
        }
    }

    while x > 0 && y > 0 {
        x -= 1;
        y -= 1;
        edits.push(Edit::equal(a[x as usize], x as usize, y as usize));
    }

    edits.reverse();
    edits
}

/// Get a value from the vector with support for negative indexing.
fn get<S>(iterable: &[S], index: i32) -> &S {
    let adjusted_index = adjust_index(iterable, index);
    iterable.get(adjusted_index).unwrap()
}

/// Set a value in the vector with support for negative indexing.
fn set<S>(iterable: &mut [S], index: i32, value: S) {
    let adjusted_index = adjust_index(iterable, index);
    iterable[adjusted_index] = value
}

fn adjust_index<S>(iterable: &[S], index: i32) -> usize {
    (if index < 0 {
        iterable.len() as i32 + index
    } else {
        index
    }) as usize
}

pub fn diff_refs<S: AsRef<str>>(
    repository: &Repository,
    lhs: S,
    rhs: S,
    writer: &mut dyn OutputWriter,
) -> crate::Result<()> {
    let mut lhs_object_resolver = ObjectResolver::from_reference(lhs.as_ref(), repository)?;
    let mut rhs_object_resolver = ObjectResolver::from_reference(rhs.as_ref(), repository)?;

    let lhs_tree = lhs_object_resolver.find_tree_by_path(&PathBuf::new()).ok();
    let rhs_tree = rhs_object_resolver.find_tree_by_path(&PathBuf::new()).ok();

    let changes = compare_trees(
        lhs_tree,
        rhs_tree,
        PathBuf::new(),
        &mut lhs_object_resolver,
        &mut rhs_object_resolver,
    )?;

    let bytes_to_lines = |bytes: &[u8]| {
        bytes
            .lines()
            .map(|s| s.map(|s| s.to_owned()))
            .collect::<Result<Vec<String>, std::io::Error>>()
    };

    for change in changes {
        match change.change_type {
            status::ChangeType::Created => {
                let rhs_blob = rhs_object_resolver.find_blob_by_path(&change.path)?;
                let rhs_blob_lines: Vec<String> = bytes_to_lines(rhs_blob.content())?;
                diff_content(
                    &change.path,
                    &[],
                    None,
                    &rhs_blob_lines,
                    Some(rhs_blob.id_as_string()),
                    writer,
                )?;
            }
            status::ChangeType::Deleted => {
                let lhs_blob = lhs_object_resolver.find_blob_by_path(&change.path)?;
                let lhs_blob_lines: Vec<String> = bytes_to_lines(lhs_blob.content())?;
                diff_content(
                    &change.path,
                    &lhs_blob_lines,
                    Some(lhs_blob.id_as_string()),
                    &[],
                    None,
                    writer,
                )?;
            }
            status::ChangeType::Modified => {
                let lhs_blob = lhs_object_resolver.find_blob_by_path(&change.path)?;
                let lhs_blob_lines = bytes_to_lines(lhs_blob.content())?;
                let rhs_blob = rhs_object_resolver.find_blob_by_path(&change.path)?;
                let rhs_blob_lines = bytes_to_lines(rhs_blob.content())?;
                diff_content(
                    &change.path,
                    &lhs_blob_lines,
                    Some(lhs_blob.id_as_string()),
                    &rhs_blob_lines,
                    Some(rhs_blob.id_as_string()),
                    writer,
                )?;
            }
        }
    }

    Ok(())
}

pub fn compare_trees(
    lhs: Option<Tree>,
    rhs: Option<Tree>,
    prefix: PathBuf,
    lhs_resolver: &mut ObjectResolver,
    rhs_resolver: &mut ObjectResolver,
) -> io::Result<Vec<status::Change>> {
    if lhs == rhs {
        return Ok(Vec::new());
    }

    let mut changes: Vec<status::Change> = Vec::new();

    let lhs_entries: Vec<&TreeEntry> = lhs
        .as_ref()
        .map_or(Vec::new(), |tree| tree.entries().iter().collect());
    let rhs_entries: Vec<&TreeEntry> = rhs
        .as_ref()
        .map_or(Vec::new(), |tree| tree.entries().iter().collect());

    let get_subtree = |entry: &TreeEntry, resolver: &mut ObjectResolver| {
        if entry.mode == FileMode::Directory {
            resolver.find_tree_by_path(&prefix.join(&entry.name)).ok()
        } else {
            None
        }
    };

    // TODO fix performance, this is O(len(lhs_entries)*len(rhs_entries))
    'outer: for lhs_entry in lhs_entries.iter() {
        for rhs_entry in rhs_entries.iter() {
            if lhs_entry.name == rhs_entry.name {
                if lhs_entry.object_id != rhs_entry.object_id {
                    if lhs_entry.mode == FileMode::Regular && rhs_entry.mode == FileMode::Regular {
                        changes.push(status::Change {
                            path: prefix.join(&lhs_entry.name),
                            change_type: status::ChangeType::Modified,
                            changed_in: status::ChangePlace::Worktree, // FIXME this is wrong
                        });
                        continue 'outer;
                    }

                    let lhs_entry_tree = get_subtree(lhs_entry, lhs_resolver);
                    let rhs_entry_tree = get_subtree(rhs_entry, rhs_resolver);

                    let sub_changes = compare_trees(
                        rhs_entry_tree,
                        lhs_entry_tree,
                        prefix.join(&lhs_entry.name),
                        lhs_resolver,
                        rhs_resolver,
                    )?;
                    changes.extend(sub_changes);
                }

                continue 'outer;
            }
        }

        changes.push(status::Change {
            path: prefix.join(&lhs_entry.name),
            change_type: status::ChangeType::Deleted,
            changed_in: status::ChangePlace::Worktree, // FIXME this is wrong
        });
    }

    'outer: for rhs_entry in rhs_entries.iter() {
        for lhs_entry in lhs_entries.iter() {
            if lhs_entry.name == rhs_entry.name {
                continue 'outer;
            }
        }

        changes.push(status::Change {
            path: prefix.join(&rhs_entry.name),
            change_type: status::ChangeType::Created,
            changed_in: status::ChangePlace::Worktree, // FIXME this is wrong
        });
    }

    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shortest_edit_path() {
        let a = "ABCABBA".chars().collect::<Vec<char>>();
        let b = "CBABAC".chars().collect::<Vec<char>>();

        let (k, shortest_paths) = compute_edit_path_graph(&a, &b);

        let x = *get(&shortest_paths[shortest_paths.len() - 1], k) as i32;
        let y = x - k;

        assert_eq!(x, 7);
        assert_eq!(y, 6);
        assert_eq!(shortest_paths.len(), 6);
    }

    #[test]
    fn test_trace_edit_points() {
        let a = "ABCABBA".chars().collect::<Vec<char>>();
        let b = "CBABAC".chars().collect::<Vec<char>>();

        let (k, trace) = compute_edit_path_graph(&a, &b);

        let edit_point_trace = trace_edit_points(k, trace);

        let expected_edit_point_trace = vec![(7, 6), (7, 5), (5, 4), (3, 1), (1, 0), (0, 0)];
        assert_eq!(edit_point_trace.len(), 6);
        assert_eq!(edit_point_trace, expected_edit_point_trace);
    }

    #[test]
    fn test_produce_edit_script() {
        let a = "ABCABBA".chars().collect::<Vec<char>>();
        let b = "CBABAC".chars().collect::<Vec<char>>();

        let expected_edits = vec![
            Edit::deletion('A', 0),
            Edit::deletion('B', 1),
            Edit::equal('C', 2, 0),
            Edit::addition('B', 1),
            Edit::equal('A', 3, 2),
            Edit::equal('B', 4, 3),
            Edit::deletion('B', 5),
            Edit::equal('A', 6, 4),
            Edit::addition('C', 5),
        ];

        let edit_point_trace = vec![(7, 6), (7, 5), (5, 4), (3, 1), (1, 0), (0, 0)];

        let edit_script = compute_edit_script(&a, &b, &edit_point_trace);

        assert_eq!(edit_script, expected_edits);
    }

    #[test]
    fn test_edit_script() {
        let a = "ABCABBA".chars().collect::<Vec<char>>();
        let b = "CBABAC".chars().collect::<Vec<char>>();

        let expected_edits = vec![
            Edit::deletion('A', 0),
            Edit::deletion('B', 1),
            Edit::equal('C', 2, 0),
            Edit::addition('B', 1),
            Edit::equal('A', 3, 2),
            Edit::equal('B', 4, 3),
            Edit::deletion('B', 5),
            Edit::equal('A', 6, 4),
            Edit::addition('C', 5),
        ];

        let edit_script = edit_script(&a, &b);

        assert_eq!(edit_script, expected_edits);
    }
}
