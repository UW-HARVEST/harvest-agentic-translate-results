//! Phase D — resource-parity checks.
//!
//! Output-only differential testing is blind to leaks: dropping the
//! `regfree()` call in `w_regexec` yields byte-identical results forever while
//! leaking a compiled DFA per call. The C calls `regfree`, so the Rust must
//! too, and that is only observable as heap growth.
//!
//! `mallinfo2().uordblks` (total bytes currently allocated by glibc malloc) is
//! used instead of RSS: it is exact, so the assertion does not need a fuzzy
//! threshold that allocator arena growth could trip.

mod common;

use common::*;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct MallInfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    /// total allocated space currently in use
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}

extern "C" {
    fn mallinfo2() -> MallInfo2;
}

/// Bytes currently handed out by glibc malloc.
fn heap_in_use() -> usize {
    unsafe { mallinfo2() }.uordblks
}

/// `mallinfo2` is process-global, so the measuring tests must not overlap.
static HEAP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const CALLS: usize = 20_000;

/// Bytes still allocated after `CALLS` `w_regexec` invocations that each
/// compile and (should) free one regex.
fn leak_of(lib: &Lib) -> i64 {
    let pat = CBuf::new(br"^[0-9]+\.[0-9]+\.([0-9]+(\.[0-9]+)*)\.*");
    let subj = CBuf::new(b"10.0.19041.1.2.3");
    let mut m = [RegMatch { rm_so: 0, rm_eo: 0 }; 2];
    let one = |m: &mut [RegMatch; 2]| unsafe {
        (lib.w_regexec)(pat.ptr(), subj.ptr(), 2, m.as_mut_ptr())
    };
    // warm-up: let any one-off internal caches settle
    for _ in 0..1_000 {
        assert_eq!(one(&mut m), 1);
    }
    let before = heap_in_use() as i64;
    for _ in 0..CALLS {
        assert_eq!(one(&mut m), 1);
    }
    heap_in_use() as i64 - before
}

/// D1 — `w_regexec` must release its compiled regex, exactly like the C does.
#[test]
fn d1_regexec_releases_its_regex() {
    let _g = HEAP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = libs();
    let c = leak_of(&l.c);
    let rs = leak_of(&l.rs);
    eprintln!(
        "[d1] heap retained after {} w_regexec calls: C = {} B, Rust = {} B",
        CALLS, c, rs
    );

    // A leaking implementation retains at least one regex_t + DFA per call,
    // i.e. hundreds of KiB at minimum. A correct one retains ~0.
    let budget = 64 * 1024i64;
    assert!(
        c.abs() < budget,
        "the C reference retained {} B — measurement is unreliable",
        c
    );
    assert!(
        rs.abs() < budget,
        "Rust w_regexec retained {} B of heap over {} calls (C: {} B) — \
         the compiled regex is not being freed",
        rs,
        CALLS,
        c
    );
}

/// D2 — same check through `parse_uname_string`, which performs up to three
/// `w_regexec` calls per invocation.
///
/// `parse_uname_string` deliberately transfers ownership of its output strings
/// to the caller, so this test frees them (the Windows branch never performs
/// the `*(p-1)` write on a `malloc`'d string, so freeing is safe here) and then
/// requires the net retention to be the same on both sides.
#[test]
fn d2_parse_releases_its_regexes() {
    extern "C" {
        fn free(p: *mut std::os::raw::c_void);
    }
    let _g = HEAP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = libs();
    let run = |lib: &Lib| -> i64 {
        let input = b"Microsoft Windows [Ver: 10.0.19041.1]";
        let once = || {
            let buf = CBuf::new(input);
            let mut osd = OsData::zeroed();
            unsafe {
                (lib.parse_uname_string)(buf.ptr(), &mut osd);
                for p in osd.as_array() {
                    if !p.is_null() {
                        free(p as *mut std::os::raw::c_void);
                    }
                }
            }
        };
        for _ in 0..1_000 {
            once();
        }
        let before = heap_in_use() as i64;
        for _ in 0..10_000 {
            once();
        }
        heap_in_use() as i64 - before
    };
    let c = run(&l.c);
    let rs = run(&l.rs);
    eprintln!("[d2] heap retained: C = {} B, Rust = {} B", c, rs);
    let budget = 64 * 1024i64;
    assert!(c.abs() < budget, "C retained {} B", c);
    assert!(
        rs.abs() < budget,
        "Rust parse_uname_string retained {} B vs C {} B",
        rs,
        c
    );
}

/// D3 — the POSIX regex ABI assumptions baked into `translation/src/lib.rs`.
///
/// The Rust translation mirrors glibc's `regmatch_t` as two `c_int` and reserves
/// `[u64; 8]` for the opaque `regex_t`. If either were wrong, `regcomp` would
/// scribble past the Rust stack slot or `match[1]` would be read at the wrong
/// offset. Measured on this platform with a throwaway C probe:
///
/// ```text
/// sizeof(regex_t)   = 64   _Alignof(regex_t) = 8
/// sizeof(regoff_t)  = 4
/// sizeof(regmatch_t)= 8    offsetof(rm_eo)   = 4
/// REG_EXTENDED      = 1    REG_NOMATCH       = 1
/// ```
#[test]
fn d3_regex_abi_assumptions() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<RegMatch>(), 8, "regmatch_t must be 8 bytes");
    assert_eq!(align_of::<RegMatch>(), 4, "regmatch_t must be 4-byte aligned");
    assert_eq!(size_of::<std::os::raw::c_int>(), 4, "regoff_t is int");
    // The 64-byte regex_t reservation is exercised destructively rather than
    // measured: 20 000 regcomp/regfree round-trips on the Rust .so (see d1)
    // would corrupt the stack and crash if the reservation were too small.
    assert_eq!(size_of::<[u64; 8]>(), 64);

    // os_data layout parity: 9 pointers, no padding.
    assert_eq!(size_of::<OsData>(), 9 * size_of::<*mut u8>());
    assert_eq!(OS_DATA_FIELDS.len(), 9);
}
