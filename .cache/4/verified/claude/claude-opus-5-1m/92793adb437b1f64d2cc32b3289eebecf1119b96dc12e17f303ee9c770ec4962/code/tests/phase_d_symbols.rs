//! Phase D — symbol parity and library-isolation sanity checks.

mod common;

use common::*;
use std::process::Command;

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under
/// the exact same name (checked via `dlsym`, i.e. the real dynamic linker).
fn d01_every_c_symbol_is_dlsym_able_in_rust() {
    let _g = lock();
    // `Api::load` already panics with a precise message if any of the ten
    // symbols is missing from either library.
    let [c, r] = both();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    assert_eq!(EXPECTED_SYMBOLS.len(), 10);
}

/// Mechanical `nm -D` diff: the set of exported symbols must be identical.
fn d02_nm_dynamic_symbol_sets_match() {
    let _g = lock();

    fn exported(p: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {}", p.display());
        let noise = [
            "_init",
            "_fini",
            "_edata",
            "_end",
            "__bss_start",
            "__gmon_start__",
        ];
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
            .filter(|s| !noise.contains(&s.as_str()))
            .filter(|s| !s.starts_with("_ITM_") && !s.starts_with("__cxa_"))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c = exported(&c_so_path());
    let r = exported(&rust_so_path());

    assert_eq!(c.len(), 10, "unexpected C export count: {c:?}");

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
    );

    let extra: Vec<&String> = r.iter().filter(|s| !c.contains(s)).collect();
    assert!(extra.is_empty(), "Rust .so exports extra symbols: {extra:?}");

    assert_eq!(c, r, "symbol sets differ");
}

/// The Rust `.so` must not import anything that is not libc / the GCC unwinder.
fn d03_rust_so_has_no_non_libc_undefined_symbols() {
    let _g = lock();
    let out = Command::new("nm")
        .args([
            "-D",
            "--undefined-only",
            rust_so_path().to_str().unwrap(),
        ])
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1).map(str::to_string))
        .filter(|s| {
            !(s.contains("@GLIBC")
                || s.contains("@GCC")
                || s.starts_with("_ITM_")
                || s.starts_with("__cxa_")
                || s.starts_with("_Unwind_")
                // weak ELF housekeeping symbols the C .so imports too
                || s == "__gmon_start__"
                || s == "gettid"
                || s == "statx")
        })
        .collect();
    assert!(bad.is_empty(), "non-libc undefined symbols in Rust .so: {bad:?}");
}

/// Both libraries are loaded `RTLD_LOCAL`, so the C `.so`'s *internal* calls to
/// `log_info` / `log_error` / `log_warning` must bind to its own definitions and
/// must not be interposed by the Rust `.so` (and vice versa). If interposition
/// happened, the two libraries would share one `log_file` static.
fn d04_libraries_have_independent_log_file_statics() {
    let _g = lock();
    let [c, r] = both();

    let c_log = unique_path("iso_c.log");
    let r_log = unique_path("iso_r.log");
    let _ = std::fs::remove_file(&c_log);
    let _ = std::fs::remove_file(&r_log);

    unset_env("MAX_TASKS");

    // Open only the C logger.
    set_env("LOG_FILE", c_log.to_str().unwrap());
    let (rc, _) = capture(|| unsafe { (c.initialize_logger)() });
    assert_eq!(rc, 0);

    // The Rust logger is still closed, so this must write nowhere at all.
    let msg = cstr(b"written-by-rust");
    capture(|| unsafe { (r.log_info)(msg.as_ptr() as *const _) });
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let c_bytes = std::fs::read(&c_log).unwrap_or_default();
    assert!(
        !c_bytes.windows(15).any(|w| w == b"written-by-rust"),
        "the Rust .so's log_info wrote into the C .so's log file — the two \
         libraries are sharing a `log_file` static (symbol interposition)"
    );
    assert!(
        !r_log.exists(),
        "the Rust logger was never initialised yet its log file exists"
    );

    // Now the mirror image.
    set_env("LOG_FILE", r_log.to_str().unwrap());
    let (rc, _) = capture(|| unsafe { (r.initialize_logger)() });
    assert_eq!(rc, 0);
    let msg = cstr(b"written-by-c");
    capture(|| unsafe { (c.log_info)(msg.as_ptr() as *const _) });
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let r_bytes = std::fs::read(&r_log).unwrap_or_default();
    assert!(
        !r_bytes.windows(12).any(|w| w == b"written-by-c"),
        "the C .so's log_info wrote into the Rust .so's log file"
    );
    // The C library's own log file must have received it.
    let c_bytes = std::fs::read(&c_log).unwrap_or_default();
    assert!(
        c_bytes.windows(12).any(|w| w == b"written-by-c"),
        "C log_info did not write to the C log file: {:?}",
        String::from_utf8_lossy(&c_bytes)
    );

    capture(|| unsafe {
        (c.finalize_logger)();
        (r.finalize_logger)();
    });
}

/// The struct layout the tests assume is the layout the C compiler produced.
fn d05_struct_layout_matches_c() {
    assert_eq!(size_of::<Task>(), 260);
    assert_eq!(align_of::<Task>(), 4);
    assert_eq!(size_of::<TaskManager>(), 16);
    assert_eq!(align_of::<TaskManager>(), 8);
    assert_eq!(std::mem::offset_of!(Task, description), 0);
    assert_eq!(std::mem::offset_of!(Task, priority), 256);
    assert_eq!(std::mem::offset_of!(TaskManager, tasks), 0);
    assert_eq!(std::mem::offset_of!(TaskManager, max_tasks), 8);
    assert_eq!(std::mem::offset_of!(TaskManager, task_count), 12);
}

// ---------------------------------------------------------------------------
// Single serialized entry point.
//
// The libtest harness writes its own "test NAME ... ok" progress lines to fd 1
// from the main thread while other test threads are still running. Because this
// harness temporarily redirects fd 1/fd 2 to capture what the *libraries* print,
// concurrently-running tests would pollute the capture. Exposing exactly one
// #[test] removes that race entirely; each scenario still reports itself through
// the label carried in the assertion message.
// ---------------------------------------------------------------------------
#[test]
fn phase_d_all() {
    d01_every_c_symbol_is_dlsym_able_in_rust();
    d02_nm_dynamic_symbol_sets_match();
    d03_rust_so_has_no_non_libc_undefined_symbols();
    d04_libraries_have_independent_log_file_statics();
    d05_struct_layout_matches_c();
}
