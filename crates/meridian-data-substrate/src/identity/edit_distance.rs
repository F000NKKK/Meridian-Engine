//! The typo suggestion: the known spelling closest to a mistyped one.

/// The largest edit distance a typo is suggested for, per three letters.
const TYPO_LETTERS: usize = 3;

/// The one of `known` whose spelling is closest to `text`, when close
/// enough to be a typo of it.
pub(crate) fn closest_spelling<'k, T>(
    text: &str,
    known: impl Iterator<Item = &'k T>,
    spelling: impl Fn(&'k T) -> &'k str,
) -> Option<&'k T> {
    let limit = text.len().div_ceil(TYPO_LETTERS).max(1);
    known
        .map(|k| (edit_distance(text, spelling(k)), k))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k)
}

/// Levenshtein distance of two spellings.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let swap = prev[j] + usize::from(ca != *cb);
            cur[j + 1] = swap.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}
