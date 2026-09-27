//! Which lines a proposed source changes, for a person to read before accepting.
//!
//! A proposal that shows only its new text asks to be trusted; one that shows
//! what it changed can be checked. Scripts are small, so a plain longest-common-
//! subsequence over lines is exact and fast enough, and needs nothing from
//! outside the standard library.

/// One line of a diff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Line {
    Same(String),
    Removed(String),
    Added(String),
}

impl Line {
    pub const fn is_change(&self) -> bool {
        !matches!(self, Self::Same(_))
    }
}

/// Above this many line pairs the table is not built and the diff says the
/// whole file was replaced, which is true and costs nothing.
const MAX_CELLS: usize = 4_000_000;

/// The lines of `before` and `after`, marked as kept, removed or added.
pub fn lines(before: &str, after: &str) -> Vec<Line> {
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    if old.len().saturating_mul(new.len()) > MAX_CELLS {
        return old
            .iter()
            .map(|line| Line::Removed((*line).to_owned()))
            .chain(new.iter().map(|line| Line::Added((*line).to_owned())))
            .collect();
    }
    // `common[i][j]`: the longest common run of old[i..] and new[j..].
    let width = new.len() + 1;
    let mut common = vec![0u32; (old.len() + 1) * width];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            common[i * width + j] = if old[i] == new[j] {
                common[(i + 1) * width + j + 1] + 1
            } else {
                common[(i + 1) * width + j].max(common[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::with_capacity(old.len().max(new.len()));
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            out.push(Line::Same(old[i].to_owned()));
            i += 1;
            j += 1;
        } else if common[(i + 1) * width + j] >= common[i * width + j + 1] {
            out.push(Line::Removed(old[i].to_owned()));
            i += 1;
        } else {
            out.push(Line::Added(new[j].to_owned()));
            j += 1;
        }
    }
    out.extend(
        old[i..]
            .iter()
            .map(|line| Line::Removed((*line).to_owned())),
    );
    out.extend(new[j..].iter().map(|line| Line::Added((*line).to_owned())));
    out
}

/// The changed lines with `context` unchanged lines around each run, and
/// `None` where unchanged lines were left out.
pub fn around_changes(diff: &[Line], context: usize) -> Vec<Option<&Line>> {
    let near_change = |index: usize| {
        let from = index.saturating_sub(context);
        let to = (index + context + 1).min(diff.len());
        diff[from..to].iter().any(Line::is_change)
    };
    let mut out = Vec::new();
    let mut skipping = false;
    for (index, line) in diff.iter().enumerate() {
        if near_change(index) {
            out.push(Some(line));
            skipping = false;
        } else if !skipping {
            out.push(None);
            skipping = true;
        }
    }
    out
}

/// How many lines were removed and added.
pub fn counts(diff: &[Line]) -> (usize, usize) {
    diff.iter()
        .fold((0, 0), |(removed, added), line| match line {
            Line::Removed(_) => (removed + 1, added),
            Line::Added(_) => (removed, added + 1),
            Line::Same(_) => (removed, added),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_changed_line_is_one_removal_and_one_addition() {
        let diff = lines("a\nb\nc\n", "a\nB\nc\n");
        assert_eq!(
            diff,
            [
                Line::Same("a".into()),
                Line::Removed("b".into()),
                Line::Added("B".into()),
                Line::Same("c".into()),
            ]
        );
        assert_eq!(counts(&diff), (1, 1));
    }

    #[test]
    fn identical_text_has_no_changes() {
        let diff = lines("a\nb\n", "a\nb\n");
        assert!(diff.iter().all(|line| !line.is_change()));
    }

    #[test]
    fn distant_unchanged_lines_are_elided_once() {
        let before = (0..20)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let after = before.replace("10", "ten");
        let diff = lines(&before, &after);
        let shown = around_changes(&diff, 2);
        // A gap, two lines of context, the change, two lines of context, a gap.
        assert_eq!(shown.len(), 1 + 2 + 2 + 2 + 1);
        assert!(shown.first().is_some_and(Option::is_none));
        assert!(shown.last().is_some_and(Option::is_none));
    }
}
