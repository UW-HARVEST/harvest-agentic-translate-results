//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Enforced mechanically with `nm -D` so it can never drift from `SYMBOLS.md`.

mod harness;
use harness::*;

fn nm_defined(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .args(["-D", "--defined-only", "--format=posix"])
        .arg(path)
        .output()
        .expect("running `nm` (binutils) failed");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let name = it.next()?;
            let kind = it.next()?;
            // Global/weak text or data symbols only.
            if matches!(kind, "T" | "W" | "D" | "B" | "R" | "V" | "G" | "S") {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name.  The diff must be EMPTY.
#[test]
fn symbols_c_subset_of_rust() {
    let c = nm_defined(&c_so_path());
    let r = nm_defined(&rust_so_path());

    assert!(
        !c.is_empty(),
        "nm found no exported symbols in the C .so ({}) — is it built?",
        c_so_path().display()
    );

    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} symbol(s) exported by the C .so: {:?}\n\
         C  ({} syms): {:?}\nRust ({} syms): {:?}",
        missing.len(),
        missing,
        c.len(),
        c,
        r.len(),
        r
    );

    // The three library symbols must be there explicitly.
    for want in ["tflac_pack_u64le", "tflac_md5_addsample", "update_md5"] {
        assert!(c.contains(&want.to_string()), "C .so must export {want}");
        assert!(r.contains(&want.to_string()), "Rust .so must export {want}");
    }
}

/// The C `.so` must export exactly the three documented library symbols — a
/// guard so that if `c_src` ever grows a function, `SYMBOLS.md` and the tests
/// are forced to be updated instead of silently passing.
#[test]
fn c_exports_exactly_the_documented_set() {
    let mut c = nm_defined(&c_so_path());
    c.retain(|s| !s.starts_with('_'));
    assert_eq!(
        c,
        vec![
            "tflac_md5_addsample".to_string(),
            "tflac_pack_u64le".to_string(),
            "update_md5".to_string(),
        ],
        "the C .so's exported symbol set changed; update SYMBOLS.md/CONFIGS.md/ERRORS.md"
    );
}

/// All three symbols must be resolvable via `dlsym` in BOTH objects (this is
/// what `harness::libs()` does) and callable across the FFI boundary.
#[test]
fn all_symbols_are_dlsym_able_and_callable() {
    let l = libs();
    let mut tpl = vec![0u8; ARENA];
    put_tflac(&mut tpl, 0, 0, 0, &[0u8; BUF_LEN], 64, 1);
    let stpl = vec![0u8; ARENA];

    diff_pack(&tpl, 0, 0x0123_4567_89AB_CDEF, "phaseD pack");
    diff_add(&tpl, 0, 64, 0x0123_4567_89AB_CDEF, "phaseD addsample");
    let r = diff_upd(&tpl, 0, &stpl, 0, "phaseD update_md5");
    assert_eq!(r, 24, "64*1 - 40 == 24");
    let _ = l;
}

/// The Rust `.so` must have no unresolved non-libc imports.  `dlopen` with
/// `RTLD_NOW` (what `libloading::Library::new` uses) already proves this, but
/// assert it explicitly too.
#[test]
fn rust_so_has_no_unresolved_symbols() {
    let out = std::process::Command::new("nm")
        .args(["-D", "-u", "--format=posix"])
        .arg(rust_so_path())
        .output()
        .expect("nm");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let allowed_prefixes = [
        "_Unwind_", "__", "_ITM_", "gettid", "statx", "abort", "bcmp", "calloc", "close",
        "dl_iterate_phdr", "free", "fstat", "getcwd", "getenv", "lseek", "malloc", "memcpy",
        "memmove", "memset", "mmap", "munmap", "open", "posix_memalign", "pthread_", "read",
        "readlink", "realloc", "realpath", "stat", "strlen", "syscall", "write", "writev",
        "sysconf", "getauxval", "sigaltstack", "sigaction", "mprotect", "pipe", "poll", "dlsym",
        "environ", "qsort", "strerror", "memcmp", "memrchr", "abs", "exit", "raise",
    ];
    let bad: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .filter(|n| !allowed_prefixes.iter().any(|p| n.starts_with(p)))
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so has unexpected undefined symbols (not libc/unwinder): {bad:?}"
    );
}
