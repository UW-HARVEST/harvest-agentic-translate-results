//! Phase D — symbol parity between the C `.so` and the Rust `.so`.
//!
//! Mechanical, not hand-maintained: the expected set is whatever `nm -D` says
//! the C shared object exports.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

fn defined_dynamic_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("cannot run nm on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(str::to_string))
        .collect()
}

#[test]
fn symbols_c_so_is_a_subset_of_rust_so() {
    let c_so = c_so_path();
    let rust_so = rust_so_path();
    assert!(c_so.exists(), "build the C library first: {}", c_so.display());
    assert!(rust_so.exists(), "missing {}", rust_so.display());

    let cs = defined_dynamic_symbols(&c_so);
    let rs = defined_dynamic_symbols(&rust_so);

    let missing: Vec<&String> = cs.difference(&rs).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so does not export {} of the C .so's {} symbols: {:?}",
        missing.len(),
        cs.len(),
        missing
    );
    assert_eq!(cs.len(), 16, "the C .so is expected to export 16 symbols, got {}", cs.len());

    // and every one of them must actually be resolvable through dlsym
    let _ = apis();
}

#[test]
fn rust_so_exports_no_extra_stbds_symbols() {
    // The Rust .so must not widen the public surface either: nothing beyond the
    // C symbol set may be exported under a C-ish (non-mangled) name.
    let cs = defined_dynamic_symbols(&c_so_path());
    let rs = defined_dynamic_symbols(&rust_so_path());
    let extra: Vec<&String> = rs
        .difference(&cs)
        .filter(|s| s.starts_with("stbds_") || s.as_str() == "arr_del" || s.as_str() == "strkey")
        .collect();
    assert!(extra.is_empty(), "Rust .so exports unexpected public symbols: {extra:?}");
}

#[test]
fn rust_so_has_no_unresolved_non_runtime_symbols() {
    let out = Command::new("nm")
        .arg("-D")
        .arg("--undefined-only")
        .arg(rust_so_path())
        .output()
        .expect("nm");
    let text = String::from_utf8_lossy(&out.stdout);
    let unresolved: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap())
        // libc + the platform C runtime + the unwinder are expected imports
        .filter(|s| {
            !s.starts_with("_Unwind_")
                && !s.starts_with("__")
                && !s.starts_with("_ITM_")
                && !s.starts_with("pthread_")
                && !matches!(
                    *s,
                    "malloc"
                        | "free"
                        | "calloc"
                        | "realloc"
                        | "posix_memalign"
                        | "memcmp"
                        | "bcmp"
                        | "memcpy"
                        | "memmove"
                        | "memset"
                        | "strcmp"
                        | "strlen"
                        | "abort"
                        | "getenv"
                        | "getcwd"
                        | "realpath"
                        | "readlink"
                        | "open64"
                        | "close"
                        | "read"
                        | "write"
                        | "writev"
                        | "lseek64"
                        | "fstat64"
                        | "stat64"
                        | "statx"
                        | "mmap64"
                        | "munmap"
                        | "syscall"
                        | "gettid"
                        | "dl_iterate_phdr"
                        | "sprintf"
                )
        })
        .collect();
    assert!(
        unresolved.is_empty(),
        "unexpected unresolved symbols in the Rust .so: {unresolved:?}"
    );
}

// ---------------------------------------------------------------------------
// Struct-layout parity. `stbds_make_hash_index` computes
//   t->storage = STBDS_ALIGN_FWD((size_t)(t+1), 64)
// so the distance from the hash index to its bucket array encodes
// `sizeof(stbds_hash_index)`. Comparing that distance per malloc alignment
// class is address-independent and catches any layout drift in the Rust
// mirror of the struct (which would otherwise silently shift every field).
// ---------------------------------------------------------------------------

fn storage_offsets_by_alignment(api: &Api) -> std::collections::BTreeMap<usize, usize> {
    use std::ffi::c_void;
    let mut m = std::collections::BTreeMap::new();
    let es = 16usize;
    let mut keep = Vec::new();
    // many allocations so that several malloc alignment classes are hit
    for _ in 0..64 {
        let t = unsafe { (api.shmode_func)(es, 0) } as *mut u8;
        unsafe {
            let ht = (*t.sub(es).sub(HEADER_SIZE).cast::<ArrayHeader>()).hash_table
                as *const HashIndex;
            let off = (*ht).storage as usize - ht as usize;
            m.insert(ht as usize & 63, off);
        }
        keep.push(t);
    }
    for t in keep {
        unsafe { (api.hmfree_func)(t.sub(es) as *mut c_void, es) };
    }
    m
}

#[test]
fn layout_hash_index_bucket_offset_matches() {
    let (_g, c, r) = scenario(0x3141_5926);
    let cm = storage_offsets_by_alignment(c);
    let rm = storage_offsets_by_alignment(r);
    assert!(!cm.is_empty(), "no samples collected");
    for (align, coff) in &cm {
        if let Some(roff) = rm.get(align) {
            assert_eq!(
                coff, roff,
                "sizeof(stbds_hash_index) disagrees: for a hash index at &63=={align} the C \
                 bucket array sits at +{coff} but the Rust one at +{roff}"
            );
        }
    }
    // 104 == sizeof(char* + 8*size_t + stbds_string_arena + stbds_hash_bucket*)
    for (align, off) in &cm {
        let want = ((align + 104) + 63) & !63usize;
        assert_eq!(*off, want - align, "unexpected C bucket offset for &63=={align}");
    }
    assert!(
        cm.keys().count() >= 2,
        "only one malloc alignment class was observed ({:?}) -- the check is weak",
        cm.keys()
    );
}
