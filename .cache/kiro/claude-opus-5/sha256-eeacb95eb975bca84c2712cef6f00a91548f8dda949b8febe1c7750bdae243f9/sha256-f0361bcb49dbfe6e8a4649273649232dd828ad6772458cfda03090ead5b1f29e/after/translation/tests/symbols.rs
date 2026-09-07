//! Phase D — symbol parity enforced as a test.
//!
//! Every symbol the C `.so` exports must also be exported by the Rust `.so`
//! under the exact same name, and the Rust `.so` must have no undefined
//! non-libc symbols.

mod harness;

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn c_so() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    let mut v: Vec<PathBuf> = std::fs::read_dir(&build)
        .expect("c_src/build missing — build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .collect();
    v.sort();
    v.pop().expect("no .so in c_src/build")
}

fn rust_so() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap();
    let p = profile.join("libflac_validate_lib.so");
    if p.is_file() {
        return p;
    }
    for d in ["release", "debug"] {
        let q = workspace_root().join("translation/target").join(d).join("libflac_validate_lib.so");
        if q.is_file() {
            return q;
        }
    }
    panic!("libflac_validate_lib.so not found");
}

/// `nm -D --defined-only <so>` → sorted symbol names.
fn defined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect();
    v.sort();
    v.dedup();
    v
}

/// `nm -D --undefined-only <so>` → sorted symbol names.
fn undefined_symbols(so: &Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success(), "nm failed on {}", so.display());
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .filter(|s| *s != "U" && *s != "w")
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    let c = defined_symbols(&c_so());
    let r = defined_symbols(&rust_so());

    // Toolchain/runtime symbols present in every ELF object — not part of the
    // library ABI under test.
    let ignorable = |s: &str| matches!(s, "_init" | "_fini" | "__bss_start" | "_edata" | "_end");

    let missing: Vec<&String> = c
        .iter()
        .filter(|s| !ignorable(s) && !r.contains(s))
        .collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
    );

    // Sanity: the two documented API symbols really are there in both.
    for want in ["flac_validate", "tflac_size_memory"] {
        assert!(c.contains(&want.to_string()), "C .so lost {want}");
        assert!(r.contains(&want.to_string()), "Rust .so lost {want}");
    }
}

#[test]
fn rust_so_has_no_unexpected_undefined_symbols() {
    let undef = undefined_symbols(&rust_so());
    // Anything resolved from libc / libgcc / ld.so is fine.
    let allowed_prefix = |s: &str| {
        s.starts_with("__")
            || s.starts_with("_Unwind")
            || s.starts_with("_ITM_")
            || matches!(
                s,
                "abort"
                    | "memcpy"
                    | "memmove"
                    | "memset"
                    | "memcmp"
                    | "bcmp"
                    | "free"
                    | "malloc"
                    | "calloc"
                    | "realloc"
                    | "posix_memalign"
                    | "getenv"
                    | "write"
                    | "writev"
                    | "dl_iterate_phdr"
                    | "sysconf"
                    | "pthread_self"
            )
            || s.contains('@')
    };
    let unexpected: Vec<&String> = undef.iter().filter(|s| !allowed_prefix(s)).collect();
    assert!(
        unexpected.is_empty(),
        "Rust .so has undefined non-libc symbols: {unexpected:?}"
    );
}

/// The `struct tflac` ABI the harness assumes must match what both `.so`s use.
/// Verified behaviourally: writing a distinct byte pattern into each field slot
/// and checking both implementations read/write the same offsets.
#[test]
fn struct_layout_agrees_across_ffi() {
    use harness::{Tflac, load_pair};
    let p = load_pair();
    assert_eq!(std::mem::size_of::<Tflac>(), 28);
    assert_eq!(std::mem::align_of::<Tflac>(), 4);

    // cur_blocksize lives at offset 24: after a successful validate it must
    // equal blocksize in both.
    let mut t = Tflac::valid();
    t.set_blocksize(4100).set_cur_blocksize(0xDEAD_BEEF);
    let (rc_c, oc) = p.c.validate(&t);
    let (rc_r, or) = p.rs.validate(&t);
    assert_eq!((rc_c, rc_r), (0, 0));
    assert_eq!(oc.cur_blocksize(), 4100);
    assert_eq!(or.cur_blocksize(), 4100);
    assert_eq!(oc.0, or.0);

    // Tail padding (bytes 21..=23) must be left alone by both.
    let mut t = Tflac::valid();
    t.0[21] = 0xA5;
    t.0[22] = 0x5A;
    t.0[23] = 0x3C;
    let (_, oc) = p.c.validate(&t);
    let (_, or) = p.rs.validate(&t);
    assert_eq!(&oc.0[21..24], &[0xA5, 0x5A, 0x3C]);
    assert_eq!(&or.0[21..24], &[0xA5, 0x5A, 0x3C]);
}
