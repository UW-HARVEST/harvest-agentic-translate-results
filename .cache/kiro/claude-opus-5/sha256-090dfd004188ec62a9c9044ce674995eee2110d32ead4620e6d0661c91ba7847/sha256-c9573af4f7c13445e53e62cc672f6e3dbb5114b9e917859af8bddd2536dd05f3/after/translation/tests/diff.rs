//! Differential tests: the C `libpng.so` and the Rust `liblibpng.so` are both
//! loaded with `libloading` and driven through their exported C ABI only.
//!
//! Each scenario runs in a child process (a re-exec of this binary that runs
//! only the `worker` test) so that error paths — which end in `png_error` —
//! can be compared exactly, including the message text and the exit status.

mod support;

use support::configs;
use support::errors_tbl;
use support::*;

/// The child-process entry point.  A no-op unless `PNGDIFF_SCEN` is set.
#[test]
fn worker() {
    support::worker_main();
}

/* ================================================================== */
/* Phase A / D — symbol parity                                        */
/* ================================================================== */

#[test]
fn a00_both_libraries_load() {
    support::ensure_rust_so_built();
    for w in [Which::C, Which::Rust] {
        let p = w.so_path();
        assert!(p.exists(), "{:?} missing (build it first)", p);
    }
}

#[test]
fn a01_symbol_parity() {
    support::ensure_rust_so_built();
    let c = nm_defined(&Which::C.so_path());
    let r = nm_defined(&Which::Rust.so_path());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(*s)).collect();
    assert!(
        missing.is_empty(),
        "{} symbols exported by the C .so are missing from the Rust .so: {:?}",
        missing.len(),
        missing
    );
    assert_eq!(c.len(), 384, "unexpected C symbol count");
    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(*s)).collect();
    assert!(extra.is_empty(), "Rust .so exports symbols the C .so does not: {extra:?}");
}

#[test]
fn a02_no_undefined_symbols_beyond_libc() {
    support::ensure_rust_so_built();
    let out = std::process::Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(Which::Rust.so_path())
        .output()
        .expect("nm");
    let allowed_prefixes = [
        "_ITM_", "_Unwind_", "__cxa_", "__errno_", "__gmon_", "__tls_get_addr", "_setjmp",
    ];
    let allowed = [
        "abort", "bcmp", "calloc", "close", "crc32", "deflate", "deflateEnd", "deflateInit2_",
        "deflateReset", "dl_iterate_phdr", "fclose", "ferror", "fflush", "fopen", "fprintf",
        "fputc", "fread", "free", "frexp", "fstat64", "fwrite", "getcwd", "getenv", "gettid",
        "gmtime", "inflate", "inflateEnd", "inflateInit2_", "inflateReset", "inflateReset2",
        "longjmp", "lseek64", "malloc", "memcmp", "memcpy", "memmove", "memset", "mmap64", "modf",
        "munmap", "open64", "posix_memalign", "pow", "pthread_key_create", "pthread_key_delete",
        "pthread_setspecific", "read", "readlink", "realloc", "realpath", "remove", "stat64",
        "statx", "stderr", "strerror", "strlen", "strtod", "syscall", "write", "writev",
        "ceil", "floor", "atof", "sysconf", "getauxval", "pthread_getspecific", "abs",
        "pthread_mutex_lock", "pthread_mutex_unlock", "pthread_mutex_trylock", "pthread_self",
        "sigaltstack", "sigaction", "sigemptyset", "sigaddset", "mprotect", "pthread_attr_init",
        "pthread_attr_destroy", "pthread_attr_getstack", "pthread_getattr_np", "__libc_start_main",
    ];
    let mut bad = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Some(sym) = line.split_whitespace().nth(1) else { continue };
        let base = sym.split('@').next().unwrap_or(sym);
        if allowed.contains(&base) || allowed_prefixes.iter().any(|p| base.starts_with(p)) {
            continue;
        }
        bad.push(base.to_string());
    }
    assert!(
        bad.is_empty(),
        "Rust .so has undefined non-libc/non-zlib symbols: {bad:?}"
    );
}

fn nm_defined(p: &std::path::Path) -> std::collections::BTreeSet<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(p)
        .output()
        .expect("nm");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

/* ================================================================== */
/* Phase B — valid-path differential tests, one test per CONFIGS group */
/* ================================================================== */

macro_rules! phase_b {
    ($name:ident, $group:literal, $rows:path) => {
        #[test]
        fn $name() {
            let rows = $rows();
            assert!(!rows.is_empty());
            check_rows($group, &rows_of(&rows));
        }
    };
}

phase_b!(b01_write_shapes, "B1", configs::rows_write_shapes);
phase_b!(b02_write_zlib_options, "B2", configs::rows_write_zlib);
phase_b!(b03_write_filters, "B3", configs::rows_write_filters);
phase_b!(b04_write_transforms, "B4", configs::rows_write_transforms);
phase_b!(b05_write_chunk_sets, "B5", configs::rows_write_chunks);
phase_b!(b06_write_io_plumbing, "B6", configs::rows_write_io);
phase_b!(b07_read_shapes, "B7", configs::rows_read_shapes);
phase_b!(b08_read_transforms, "B8", configs::rows_read_transforms);
phase_b!(b09_read_png_masks, "B9", configs::rows_read_png_masks);
phase_b!(b10_read_chunk_sets, "B10", configs::rows_read_chunks);
phase_b!(b11_unknown_chunks, "B11", configs::rows_unknown);
phase_b!(b12_progressive_read, "B12", configs::rows_progressive);
phase_b!(b13_simplified_read, "B13", configs::rows_simple_read);
phase_b!(b14_simplified_write, "B14", configs::rows_simple_write);
phase_b!(b15_setters_getters, "B15", configs::rows_setget);
phase_b!(b16_read_fuzz, "B16", configs::rows_read_fuzz);
phase_b!(b17_write_fuzz, "B17", configs::rows_write_fuzz);
phase_b!(b18_large_images, "B18", configs::rows_large);
phase_b!(b19_user_transforms, "B19", configs::rows_user_transforms);
phase_b!(b20_mng_features, "B20", configs::rows_mng);
phase_b!(b21_crc_actions, "B21", configs::rows_crc_actions);
phase_b!(b22_fp_getters, "B22", configs::rows_fp_getters);
phase_b!(b23_stdio_entry_points, "B23", configs::rows_stdio);
phase_b!(b24_free_data, "B24", configs::rows_freedata);
phase_b!(b25_filter_heuristics, "B25", configs::rows_heuristics);
phase_b!(b26_simplified_api_fuzz, "B26", configs::rows_simple_fuzz);

#[test]
fn c06_mutation_fuzz() {
    let rows = configs::rows_mutation_fuzz();
    assert!(!rows.is_empty());
    check_rows("C5", &rows_of(&rows));
}

/* ================================================================== */
/* Phase C — error-path differential tests, one per ERRORS.md row      */
/* ================================================================== */

fn err_rows(range: std::ops::Range<usize>) -> Vec<(String, String)> {
    errors_tbl::ROWS[range]
        .iter()
        .map(|(id, func, trig, exp)| {
            (
                format!("{func} — {trig} — {exp}"),
                format!("err|id={id}"),
            )
        })
        .collect()
}

#[test]
fn c01_error_surface_part1() {
    let n = errors_tbl::ROWS.len();
    check_rows("C1", &err_rows(0..n / 4));
}

#[test]
fn c02_error_surface_part2() {
    let n = errors_tbl::ROWS.len();
    check_rows("C2", &err_rows(n / 4..n / 2));
}

#[test]
fn c03_error_surface_part3() {
    let n = errors_tbl::ROWS.len();
    check_rows("C3", &err_rows(n / 2..3 * n / 4));
}

#[test]
fn c04_error_surface_part4() {
    let n = errors_tbl::ROWS.len();
    check_rows("C4", &err_rows(3 * n / 4..n));
}

#[test]
fn c05_every_error_row_has_a_unique_id() {
    let mut ids: Vec<&str> = errors_tbl::ROWS.iter().map(|r| r.0).collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), total, "duplicate ids in errors_tbl::ROWS");
    assert!(total >= 200, "error surface table unexpectedly small: {total}");
}
