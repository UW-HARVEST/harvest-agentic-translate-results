//! Translation of c_src/src/lib.c
//!
//! The original C exposes a single function:
//!     const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
//!
//! It scans a buffer of NUL-separated strings and returns an allocated array of
//! pointers, one per line. The Rust translation preserves identical semantics:
//! it returns `Some(Vec<offset>)` of length `num_lines` on success, or `None`
//! when the buffer does not contain enough NUL-terminated strings.

/// Create a vector of byte offsets, one per line, into `buffer`.
///
/// `buffer` is treated as a sequence of NUL-terminated byte strings packed
/// together. The function walks the buffer, recording the start of each
/// string, advancing past its terminating NUL byte. If fewer than `num_lines`
/// strings are found before the buffer is exhausted, `None` is returned to
/// mirror the original C function returning `NULL`.
pub fn util_create_line_pointers(buffer: &[u8], num_lines: usize) -> Option<Vec<usize>> {
    let buffer_size = buffer.len();
    let mut line_offsets: Vec<usize> = Vec::with_capacity(num_lines);
    let mut line_index: usize = 0;
    let mut pos: usize = 0;

    while line_index < num_lines && pos < buffer_size {
        let mut len: usize = 0;
        line_offsets.push(pos);
        line_index += 1;

        // Find the next null terminator, being careful not to go past the buffer.
        while (pos + len < buffer_size) && buffer[pos + len] != 0 {
            len += 1;
        }

        // Move past this string and its null terminator.
        pos += len;
        if pos < buffer_size {
            pos += 1; // Skip the null terminator if we're not at buffer end.
        }
    }

    // Verify we processed the expected number of lines.
    if line_index != num_lines {
        return None;
    }

    Some(line_offsets)
}
