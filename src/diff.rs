use std::{
    fmt::{Debug, Display},
    cmp,
    fs, io,
    path::Path,
};

use crate::{
    index::Index,
    output::{Color, OutputWriter},
    status,
    workspace::Repository,
};

const MAX_DIFF_CONTEXT_LINES: usize = 3;

pub fn diff_repository(repository: &Repository, writer: &mut dyn OutputWriter) -> io::Result<()> {
    let mut index = repository.load_index()?;
    let mut files_with_unstaged_changes =
        status::resolve_files_with_unstaged_changes(&repository, &mut index.as_mut())?;
    files_with_unstaged_changes.sort();

    for file in files_with_unstaged_changes {
        diff_file(&file, &index.as_mut(), repository, writer)?;
    }

    Ok(())
}

fn diff_file(
    file: &Path,
    index: &Index,
    repository: &Repository,
    writer: &mut dyn OutputWriter,
) -> io::Result<()> {
    let relative_path = repository.worktree().relativize_path(&file);
    let a_index_entry = index.get(&relative_path).unwrap();
    let a_raw = Vec::from(
        repository
            .database
            .load_blob(&a_index_entry.object_id)
            .unwrap()
            .content(),
    );

    let a = String::from_utf8(a_raw).unwrap();
    let b = fs::read_to_string(&file).unwrap();

    let a_lines = a.split("\n").collect::<Vec<&str>>();
    let b_lines = b.split("\n").collect::<Vec<&str>>();

    writer.write(format!("--- a/{}", relative_path.display()))?;
    writer.write(format!("+++ b/{}", relative_path.display()))?;

    let edit_script = edit_script(&a_lines, &b_lines);
    let mut equals_head_size = 0;
    let mut equals_tail_left_to_write = 0;

    let write_head_context =
        |i, equals_head_size, writer: &mut dyn OutputWriter| -> io::Result<()> {
            let context_size = cmp::min(MAX_DIFF_CONTEXT_LINES, equals_head_size);
            let equal_edits = &edit_script[i - context_size..i];

            for edit in equal_edits {
                writer.write(format!(" {}", edit.s))?;
            }

            Ok(())
        };

    for (i, edit) in edit_script.iter().enumerate() {
        match edit.kind {
            EditKind::Equal => {
                if equals_tail_left_to_write > 0 {
                    writer.write(format!(" {}", edit.s))?;
                    equals_tail_left_to_write -= 1;
                } else {
                    equals_head_size += 1;
                }
            }
            EditKind::Deletion => {
                write_head_context(i, equals_head_size, writer)?;

                writer.set_color(Color::Red)?;
                writer.write(format!("-{}", edit.s))?;
                writer.reset_formatting()?;

                equals_head_size = 0;
                equals_tail_left_to_write = MAX_DIFF_CONTEXT_LINES;
            }
            EditKind::Addition => {
                write_head_context(i, equals_head_size, writer)?;

                writer.set_color(Color::Green)?;
                writer.write(format!("+{}", edit.s))?;
                writer.reset_formatting()?;

                equals_head_size = 0;
                equals_tail_left_to_write = MAX_DIFF_CONTEXT_LINES;
            }
        }
    }

    Ok(())
}

/**
 * Computes a diff between two arbitrary sequences. The typical thing to use would be two lists of
 * strings, where each element represents a line.
 *
 * ```
 * use rut::diff;
 *
 * let a = "First line\nSecond line\nThird line".split('\n').collect::<Vec<&str>>();
 * let b = "Second line\nThird line\nFourth line".split('\n').collect::<Vec<&str>>();
 *
 * let diff = diff::diff(&a, &b);
 *
 * assert_eq!(diff, "-First line\n Second line\n Third line\n+Fourth line\n");
 * ```
 */
pub fn diff<S: Eq + Copy + Display>(a: &[S], b: &[S]) -> String {
    let edit_script = edit_script(a, b);
    let mut result = String::new();

    for edit in edit_script {
        match edit.kind {
            EditKind::Equal => {
                result.push_str(&format!(" {}", edit.s));
            }
            EditKind::Deletion => {
                result.push_str(&format!("-{}", edit.s));
            }
            EditKind::Addition => {
                result.push_str(&format!("+{}", edit.s));
            }
        }
        result.push_str("\n");
    }
    result
}

/**
 * Computes an edit script between two arbitrary sequences.
 *
 * Example:
 * ```
 * use rut::diff;
 * use rut::diff::{Edit, EditKind};
 *
 * let a = "ABC".chars().collect::<Vec<char>>();
 * let b = "BBD".chars().collect::<Vec<char>>();
 *
 * let expected_edits = vec![
 *     Edit::deletion('A', 0),
 *     Edit::equal('B', 1, 0),
 *     Edit::deletion('C', 2),
 *     Edit::addition('B', 1),
 *     Edit::addition('D', 2),
 * ];
 *
 * let edit_script = diff::edit_script(&a, &b);
 *
 * assert_eq!(edit_script, expected_edits);
 * ```
 */
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
            } as usize;

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
    s: S,
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
            self.s,
            to_position_string(self.a_position),
            to_position_string(self.b_position)
        )
    }
}

impl<S: Eq> Edit<S> {
    pub fn addition(s: S, b_position: usize) -> Edit<S> {
        Edit {
            s,
            a_position: None,
            b_position: Some(b_position),
            kind: EditKind::Addition,
        }
    }

    pub fn deletion(s: S, a_position: usize) -> Edit<S> {
        Edit {
            s,
            a_position: Some(a_position),
            b_position: None,
            kind: EditKind::Deletion,
        }
    }

    pub fn equal(s: S, a_position: usize, b_position: usize) -> Edit<S> {
        Edit {
            s,
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
        k = compute_previous_k(k, d as i32, &v);
        let x = *get(&v, k) as i32;
        let y = x - k;
        edit_points.push((x, y));
    }

    edit_points
}

/**
 * Compute the previous k-value in the edit path graph. This function is optimized for
 * understandability rather than performance, it can easily be compressed into a single condition.
 */
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

/**
 * Get a value from the vector with support for negative indexing.
 */
fn get<S>(iterable: &[S], index: i32) -> &S {
    let adjusted_index = adjust_index(iterable, index);
    iterable.get(adjusted_index).unwrap()
}

/**
 * Set a value in the vector with support for negative indexing.
 */
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
