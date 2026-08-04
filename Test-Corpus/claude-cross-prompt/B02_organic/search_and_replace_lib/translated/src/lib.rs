// Translation of c_src/src/lib.c
//
// The C function `searchAndReplace` returns a heap-allocated C string that the
// caller is expected to free. We model the same byte-for-byte behavior here by
// operating on byte slices (since the C code is byte-oriented and has no
// understanding of multi-byte encodings).

/// Search `orig` for occurrences of `search` and replace each with `value`,
/// returning a new owned `Vec<u8>` containing the result (without a trailing
/// NUL byte). This mirrors `searchAndReplace` from `c_src/src/lib.c`.
///
/// Behavior preserves the quirks of the C implementation. In particular, the
/// C version uses `strstr` and only ever advances by `search_len` after a
/// match, so the replacement behavior for overlapping matches matches the
/// original.
pub fn search_and_replace(orig: &[u8], search: &[u8], value: &[u8]) -> Vec<u8> {
    let orig_len = orig.len();
    let search_len = search.len();

    // Find first match. If `search` is empty, `strstr` in C returns the
    // haystack pointer itself (i.e. position 0). `memmem`/our find below has
    // the same behavior when given an empty needle.
    let first = find_subslice(orig, search);
    if first.is_none() {
        return orig.to_vec();
    }

    let mut inx_start = first.unwrap();
    let mut from = inx_start + search_len;
    let mut tmp: Vec<u8> = Vec::new();

    // Copy content before first match, if any
    if inx_start > 0 {
        tmp.extend_from_slice(&orig[..inx_start]);
    }

    let mut p_some = true;
    while p_some {
        // Copy replacement
        tmp.extend_from_slice(value);

        // Search for further occurrences starting after the current match
        let search_from = inx_start + search_len;
        let next = if search_from <= orig_len {
            find_subslice(&orig[search_from..], search).map(|i| i + search_from)
        } else {
            None
        };

        match next {
            Some(inx_start2) => {
                // Copy content between matches, if any
                if inx_start2 > from {
                    let gap = inx_start2 - from;
                    tmp.extend_from_slice(&orig[from..from + gap]);
                }
                inx_start = inx_start2;
                from = inx_start + search_len;
            }
            None => {
                from = inx_start + search_len;
                p_some = false;
            }
        }
    }

    // Copy content after last match, if any
    if from < orig_len && from > 0 {
        tmp.extend_from_slice(&orig[from..orig_len]);
    }

    tmp
}

/// Find the first occurrence of `needle` in `haystack`, returning its byte
/// index. Mirrors C's `strstr` for byte sequences.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    let last = haystack.len() - needle.len();
    let mut i = 0;
    while i <= last {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}
