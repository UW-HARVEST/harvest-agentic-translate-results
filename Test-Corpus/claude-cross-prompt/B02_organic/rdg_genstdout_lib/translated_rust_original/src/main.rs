// Translation of c_src/src/lib.c to Rust.
//
// The original C source defines two library functions: `extractFilename` and
// `FIO_createFilename_fromOutDir`. There is no `main` in the C code, so this
// executable's `main` performs no I/O. The translated functions are provided
// here so they can be exercised by callers that link to this crate.

use std::process::exit;

/// Equivalent of C `extractFilename`: returns the slice of `path` after the
/// last occurrence of `separator`. If the separator is not found, returns
/// `path` unchanged.
fn extract_filename(path: &[u8], separator: u8) -> &[u8] {
    match path.iter().rposition(|&b| b == separator) {
        Some(idx) => &path[idx + 1..],
        None => path,
    }
}

/// Equivalent of C `FIO_createFilename_fromOutDir`.
///
/// Combines `out_dir_name` and the trailing filename component of `path`,
/// allocating extra room for `suffix_len` bytes. Mirrors the C behaviour of
/// using '/' as the separator on non-Windows platforms.
#[allow(non_snake_case)]
fn FIO_createFilename_fromOutDir(path: &[u8], out_dir_name: &[u8], suffix_len: usize) -> Vec<u8> {
    // Match the C preprocessor selection. We follow the non-Windows branch on
    // any non-Windows target to mirror the build that produced the original
    // shared library.
    #[cfg(windows)]
    let separator: u8 = b'\\';
    #[cfg(not(windows))]
    let separator: u8 = b'/';

    let filename_start = extract_filename(path, separator);
    #[cfg(windows)]
    let filename_start = extract_filename(filename_start, b'/');

    // calloc(1, total_len) — match the C-side size computation. We don't need
    // to actually allocate that exact size in Rust, but we'll size the Vec to
    // match what the C function produces.
    let total_len = out_dir_name.len() + 1 + filename_start.len() + suffix_len + 1;
    let mut result: Vec<u8> = vec![0u8; total_len];

    // memcpy(result, outDirName, strlen(outDirName))
    result[..out_dir_name.len()].copy_from_slice(out_dir_name);

    if !out_dir_name.is_empty() && out_dir_name[out_dir_name.len() - 1] == separator {
        // memcpy(result + strlen(outDirName), filenameStart, strlen(filenameStart))
        let start = out_dir_name.len();
        result[start..start + filename_start.len()].copy_from_slice(filename_start);
    } else {
        // memcpy(result + strlen(outDirName), &separator, 1)
        result[out_dir_name.len()] = separator;
        // memcpy(result + strlen(outDirName) + 1, filenameStart, strlen(filenameStart))
        let start = out_dir_name.len() + 1;
        result[start..start + filename_start.len()].copy_from_slice(filename_start);
    }

    result
}

// Suppress unused-warning for code paths that the executable's `main` does not
// exercise. The functions exist to faithfully mirror the C library API.
#[allow(dead_code)]
fn _silence_unused() {
    let _ = FIO_createFilename_fromOutDir(b"", b"", 0);
}

fn main() {
    // The original translation unit (lib.c) does not define `main`; it is a
    // shared library. To match the absence of any program output, this `main`
    // performs no I/O and exits successfully.
    let _ = exit;
}
