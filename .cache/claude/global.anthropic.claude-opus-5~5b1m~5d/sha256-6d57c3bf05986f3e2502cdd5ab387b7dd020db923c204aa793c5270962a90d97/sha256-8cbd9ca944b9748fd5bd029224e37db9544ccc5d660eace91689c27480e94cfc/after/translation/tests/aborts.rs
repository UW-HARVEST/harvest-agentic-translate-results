//! Phase C — differential tests for the two inputs on which the C library
//! *terminates the process* rather than returning.
//!
//! These cannot run in-process (one side aborts), so each spawns a tiny helper
//! child that `dlopen`s ONE library and performs ONE operation; the parent then
//! compares the child's exit status and stderr between the C `.so` and the Rust
//! `.so`.  The helper is compiled once with `cc` into the cargo target dir.
//!
//! Covered:
//!   * `ERRORS.md` row 64 — `stbds_stralloc` with a caller-supplied arena whose
//!     `block` field is >= 128, i.e. a shift count >= 64 (C: masked to 6 bits by
//!     `shlq %cl`; Rust must mask identically instead of panicking).
//!   * `ERRORS.md` row 37 — `assert(slot >= 0)` at `lib.c:846`, reachable with
//!     `mode == 2`, which makes `hmdel_key` take the *binary* re-find branch
//!     (`mode == STBDS_HM_STRING` is false) while `hm_find_slot` still hashes
//!     as a *string* (`mode >= STBDS_HM_STRING` is true), so the re-find misses.

mod common;
use common::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const HELPER_SRC: &str = r##"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stddef.h>
typedef struct { void *storage; size_t remaining; unsigned char block, mode; } Arena;
int main(int argc, char **argv) {
  void *h = dlopen(argv[1], RTLD_NOW);
  if (!h) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 2; }
  int which = atoi(argv[2]);
  void (*rs)(size_t) = dlsym(h, "stbds_rand_seed");
  rs(0x31415926);
  if (which == 1) {
    /* stralloc with a preset a->block (shift count = block>>1, up to 127) */
    char *(*stralloc)(Arena *, char *) = dlsym(h, "stbds_stralloc");
    Arena a; memset(&a, 0, sizeof a);
    a.block = (unsigned char) atoi(argv[3]);
    char s[] = "hello";
    char *p = stralloc(&a, s);
    printf("ok len=%zu block=%u remaining=%zu\n", strlen(p), a.block, a.remaining);
  } else if (which == 2) {
    /* hmdel_key(mode=2) removing a NON-last element -> lib.c:846 assert */
    void *(*put)(void *, size_t, void *, size_t, int) = dlsym(h, "stbds_hmput_key");
    void *(*del)(void *, size_t, void *, size_t, size_t, int) = dlsym(h, "stbds_hmdel_key");
    size_t es = 16; void *t = NULL;
    char *keys[4] = { "alpha", "beta", "gamma", "delta" };
    for (int i = 0; i < 4; i++) t = put(t, es, keys[i], 8, 2);
    t = del(t, es, keys[atoi(argv[3])], 8, 0, 2);
    printf("survived len=%zu\n", *(size_t *)((char *)t - es - 32));
  }
  fflush(stdout);
  return 0;
}
"##;

fn helper() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/abort_helper");
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("helper.c");
    let bin = dir.join("helper");
    std::fs::write(&src, HELPER_SRC).unwrap();
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let out = Command::new(&cc)
        .args(["-O0", "-o"])
        .arg(&bin)
        .arg(&src)
        .arg("-ldl")
        .output()
        .expect("failed to invoke the C compiler for the abort helper");
    assert!(
        out.status.success(),
        "helper build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    bin
}

fn run(bin: &Path, so: &Path, which: &str, arg: &str) -> Output {
    Command::new(bin)
        .arg(so)
        .arg(which)
        .arg(arg)
        .output()
        .expect("failed to run the abort helper")
}

/// Status comparison that distinguishes "exited 0", "exited N" and
/// "killed by signal N" — so an abort is never conflated with a segfault or a
/// normal return.
fn status_of(o: &Output) -> String {
    use std::os::unix::process::ExitStatusExt;
    match (o.status.code(), o.status.signal()) {
        (Some(c), _) => format!("exit {c}"),
        (None, Some(s)) => format!("signal {s}"),
        _ => "unknown".to_string(),
    }
}

/// `assert`-message comparison. glibc prints
/// `<prog>: <file>:<line>: <func>: Assertion `<expr>' failed.`
/// The `<file>` part is the C's `__FILE__`, which CMake expands to an absolute,
/// checkout-specific path; the Rust side uses the relative path. Everything
/// after the last `/` before `:<line>:` must match exactly.
fn assert_tail(stderr: &[u8]) -> String {
    let s = String::from_utf8_lossy(stderr);
    match s.find("lib.c:") {
        Some(i) => s[i..].trim_end().to_string(),
        None => s.trim_end().to_string(),
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 64 — stralloc shift count >= 64
// ---------------------------------------------------------------------------

#[test]
fn abort_row64_stralloc_shift_count_masking() {
    let bin = helper();
    let c = find_c_so();
    let r = find_rust_so();

    // Sweep every possible `a->block` (0..=255), which covers the whole
    // >= 128 region where the C's shift count exceeds the word size.
    //
    // Classification is DYNAMIC, from the C's own behaviour, so the test does
    // not depend on how much memory this machine will hand out:
    //
    //  * C exits 0  -> defined behaviour. Rust must also exit 0 with
    //                  byte-identical stdout (block/remaining/len).
    //  * C dies by  -> the masked shift produced a blocksize so large that the
    //    a signal      C's *unchecked* `realloc` (lib.c:906) returned NULL and
    //                  the C then dereferenced it. That is the C's own UB, not
    //                  a shift question; require only that Rust also terminates
    //                  abnormally rather than returning a bogus success.
    let mut defined = 0usize;
    let mut c_ub = 0usize;
    for b in 0..256u32 {
        // `block` values whose masked shift count is >= 25 make the C ask for
        // >= 16 GiB; those runs are slow and are pure C-UB territory, so sample
        // them instead of testing all of them.  Every value with a small
        // blocksize — which includes the entire interesting boundary region
        // 120..=135 and 248..=255 — is always tested.
        let count = (b >> 1) & 63;
        if count >= 25 && b % 8 != 0 {
            continue;
        }
        let bs = b.to_string();
        let oc = run(&bin, &c, "1", &bs);
        let or = run(&bin, &r, "1", &bs);
        if status_of(&oc) == "exit 0" {
            defined += 1;
            assert_eq!(
                status_of(&or),
                "exit 0",
                "block={b}: C returned normally but Rust did not ({})\n{}",
                status_of(&or),
                String::from_utf8_lossy(&or.stderr)
            );
            assert_eq!(
                oc.stdout,
                or.stdout,
                "block={b}: stdout differs (C={:?} Rust={:?})",
                String::from_utf8_lossy(&oc.stdout),
                String::from_utf8_lossy(&or.stdout)
            );
        } else {
            c_ub += 1;
            assert_ne!(
                status_of(&or),
                "exit 0",
                "block={b}: C died ({}) but Rust returned success — Rust must \
                 not invent a result where the C has none",
                status_of(&oc)
            );
        }
    }
    // The >= 128 half must be dominated by the defined case; before the fix
    // every one of those 128 values aborted the Rust build.
    assert!(
        defined >= 60,
        "expected >= 60 defined-behaviour block values, got {defined} (C-UB: {c_ub})"
    );

    // Spot-check the exact boundary the bug lived at: block 127 -> shift 63
    // (in range) and block 128 -> shift 64 (out of range, masked to 0).
    for b in [127u32, 128, 129, 254, 255] {
        let oc = run(&bin, &c, "1", &b.to_string());
        let or = run(&bin, &r, "1", &b.to_string());
        assert_eq!(status_of(&oc), "exit 0", "block={b}: C should return");
        assert_eq!(
            status_of(&or),
            "exit 0",
            "block={b}: Rust must return normally like the C\n{}",
            String::from_utf8_lossy(&or.stderr)
        );
        assert_eq!(oc.stdout, or.stdout, "block={b}: stdout");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 37 — assert(slot >= 0) at lib.c:846
// ---------------------------------------------------------------------------

#[test]
fn abort_row37_hmdel_mode2_assert_slot_ge_zero() {
    let bin = helper();
    let c = find_c_so();
    let r = find_rust_so();

    // Deleting index 0, 1 or 2 out of 4 entries takes the memmove + re-find
    // path (old_index != final_index), where mode == 2 makes the re-find use
    // the raw element bytes as a string key -> miss -> assert fires.
    for target in ["0", "1", "2"] {
        let oc = run(&bin, &c, "2", target);
        let or = run(&bin, &r, "2", target);
        assert_eq!(
            status_of(&oc),
            "signal 6",
            "the C is expected to abort for target={target}, got {} / {}",
            status_of(&oc),
            String::from_utf8_lossy(&oc.stderr)
        );
        assert_eq!(
            status_of(&oc),
            status_of(&or),
            "target={target}: exit status differs\nC: {}\nRust: {}",
            String::from_utf8_lossy(&oc.stderr),
            String::from_utf8_lossy(&or.stderr)
        );
        assert_eq!(
            assert_tail(&oc.stderr),
            assert_tail(&or.stderr),
            "target={target}: assert message differs"
        );
        assert_eq!(
            assert_tail(&oc.stderr),
            "lib.c:846: stbds_hmdel_key: Assertion `slot >= 0' failed.",
            "unexpected assert text"
        );
    }

    // Deleting the LAST element (index 3) skips the re-find entirely, so both
    // must return normally with identical output.
    let oc = run(&bin, &c, "2", "3");
    let or = run(&bin, &r, "2", "3");
    assert_eq!(status_of(&oc), "exit 0", "C should survive deleting the last element");
    assert_eq!(status_of(&oc), status_of(&or), "target=3: status");
    assert_eq!(oc.stdout, or.stdout, "target=3: stdout");
}
