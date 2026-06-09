// Translation of c_src/src/lib.c to Rust.
//
// The original C source defines a single library function `tool_basename`
// (declared in c_src/include/lib.h) and is built as a SHARED library by the
// CMakeLists.txt — it has no `main` entry point. This crate exposes the same
// function with byte-identical behavior and provides an empty `main` so the
// crate can be produced as an executable as required.

/// Equivalent of the C `tool_basename` function.
///
/// Mirrors the behavior of:
/// ```c
/// char *tool_basename(char *path)
/// {
///   char *s1 = strrchr(path, '/');
///   char *s2 = strrchr(path, '\\');
///
///   if (s1 && s2) {
///     path = (s1 > s2) ? s1 + 1 : s2 + 1;
///   } else if (s1) {
///     path = s1 + 1;
///   } else if (s2) {
///     path = s2 + 1;
///   }
///   return path;
/// }
/// ```
///
/// Returns a sub-slice of `path` starting just after the last `/` or `\\`.
/// If neither is present, returns `path` unchanged.
pub fn tool_basename(path: &str) -> &str {
    let bytes = path.as_bytes();
    let s1 = bytes.iter().rposition(|&b| b == b'/');
    let s2 = bytes.iter().rposition(|&b| b == b'\\');

    let start = match (s1, s2) {
        (Some(i1), Some(i2)) => {
            // In C, s1 > s2 means the '/' was at a higher address (later index)
            // than the '\\'. Skip past whichever separator came last.
            if i1 > i2 {
                i1 + 1
            } else {
                i2 + 1
            }
        }
        (Some(i1), None) => i1 + 1,
        (None, Some(i2)) => i2 + 1,
        (None, None) => return path,
    };

    // Safety: `start` is at most `bytes.len()` (strrchr returns a pointer
    // within the string, so +1 lands at most at the trailing NUL position,
    // which corresponds to the end of the Rust string slice).
    &path[start..]
}

fn main() {
    // The original C source is a library with no `main`, so this program
    // intentionally produces no output for any input.
}
