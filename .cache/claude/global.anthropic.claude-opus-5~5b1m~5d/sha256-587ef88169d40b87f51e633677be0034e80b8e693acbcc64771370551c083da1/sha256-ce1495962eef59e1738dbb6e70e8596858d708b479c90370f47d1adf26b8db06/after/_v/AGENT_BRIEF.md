# Differential-verification briefing (read fully)

`$W = $HARVEST_WORKDIR` (the working dir).

We are verifying that the Rust crate in `$W/translation` is a byte-exact
translation of the C libsodium in `$W/c_src/libsodium`.

**The C is ground truth and is ALWAYS correct.** Never "fix" the C. Never modify
anything under `$W/c_src/`. Fix only the Rust.

## Already built for you

* C shared library: `$W/c_src/build/libsodium.so` (already built; do not rebuild
  unless it is missing: `cd $W/c_src/build && cmake --build . -j8`).
* Rust cdylib: `cd $W/translation && cargo build --release --offline`
  → `target/release/liblibsodium.so`. `cargo check` is clean already.
* Build config being reproduced: **no `HAVE_*` macros at all** (equivalent to
  `configure --disable-asm`): no TI_MODE, no NATIVE_LITTLE_ENDIAN, no SIMD,
  no pthreads. To see exactly what the C compiler sees, run:
  `HARVEST_WORKDIR=$W bash $W/tools/cpp.sh $W/c_src/libsodium/<path>/<file>.c`
  (the `HARVEST_WORKDIR=` prefix is required). **That output is the ground truth**
  for which branch is compiled and for the final linker names (many internal
  names are `#define`d to `_sodium_`-prefixed symbols by
  `include/sodium/private/quirks.h`).

## Test harness (already written — DO NOT EDIT IT)

`$W/translation/tests/common/mod.rs` loads BOTH `.so`s with `libloading` and
calls `sodium_init()` on each. Never call Rust functions directly; always go
through the loaded `.so`. Example (see `$W/translation/tests/smoke.rs`):

```rust
#[macro_use]
mod common;
use core::ffi::c_int;

#[test]
fn sha256() {
    // both!(symbol_name, fn type) -> (c_fn, rust_fn)
    let (c, r) = both!("crypto_hash_sha256",
                       unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int);
    let mut rng = common::Rng::new(0xC0FFEE);      // deterministic splitmix64
    for n in [0usize, 1, 63, 64, 65, 1000] {
        let msg = rng.bytes(n);
        let (mut co, mut ro) = ([0u8; 32], [0u8; 32]);
        let rc = unsafe { c(co.as_mut_ptr(), msg.as_ptr(), n as u64) };
        let rr = unsafe { r(ro.as_mut_ptr(), msg.as_ptr(), n as u64) };
        assert_eq!(rc, rr, "return code");            // same error/success code
        common::eqb(&format!("sha256 n={n}"), &co, &ro); // byte-identical output
    }
}
```

Harness API: `common::libs()`, macros `both!`, `getsym!`, `both_data!`,
`common::Rng{new,next_u64,u8,below,fill,bytes}`, `common::eqb`, `common::eqi`,
`common::hex`.

Helpful extras you may need (write them in YOUR OWN test file, not in common):
* Opaque state structs: allocate `vec![0u8; statebytes]` where `statebytes`
  comes from the C `crypto_*_statebytes()` export, and check the C and Rust
  `statebytes()` agree. Compare the **whole state buffer** byte-for-byte after
  `init`/`update` where the C state is a plain struct (this catches padding-free
  layout divergence); if a state legitimately contains uninitialised padding,
  compare only the final digest/output and say so in a comment.
* When output length is variable, always also fill the output buffer with a
  distinct canary pattern in both calls and compare the FULL buffer, so
  over-writes / under-writes are caught.

## Rules

1. Run tests as:
   `cd $W/translation && timeout 600 cargo test --release --offline --target-dir $W/_t/<AREA> --test <AREA> 2>&1 | tail -40`
   (use YOUR OWN `--target-dir` so parallel agents don't fight over the lock).
   `--release` is mandatory (the crate needs `overflow-checks = false`).
   Note `panic = "abort"`: a failing assert aborts the test binary, so run
   with `-- --test-threads=1` if you need to see which case failed first.
2. Do **not** modify: `$W/c_src/**`, `translation/Cargo.toml`,
   `translation/tests/common/mod.rs`, `translation/src/lib.rs`, or any Rust
   `src/*.rs` module outside your assigned list. If you find a divergence that
   is rooted in a module you do not own, STILL report it precisely (function,
   input, C vs Rust output) in your final report and mark the row as FAILING
   rather than silently skipping it.
3. Never `panic!`/`unwrap` in library code; never stub a Rust function to make a
   test pass. Fix the real logic to match the C.
4. Randomized, property-style testing with a FIXED seed. Many inputs per
   configuration row (≥20 random cases per row where cheap; ≥3 for expensive
   ones like pwhash). Include boundary sizes: 0, 1, block-1, block, block+1,
   multi-block, and sizes that straddle internal buffer boundaries.
5. Error paths must match the **exact** return value (`-1`, `0`, `NULL`,
   specific errno-like codes), not merely "both failed". Include:
   null pointers where the C tolerates them, zero/oversized lengths, values one
   past a valid range, and **out-of-range enum values** (e.g. an `alg` or `tag`
   int with no valid variant) — C enums accept any `int`.
   If a C path calls `sodium_misuse()`/`abort()`, do NOT test it in-process
   (it would kill the test binary); instead note it in the errors table with
   "abort — not testable in-process" and verify by inspection that the Rust
   does the same.

## Deliverables (all three are mandatory)

1. `$W/translation/tests/<AREA>.rs` — the differential tests, all passing.
2. `$W/_v/configs/<AREA>.md` — CONFIGURATION-SURFACE table fragment: markdown
   rows only (no header), one row per meaningful combination of
   options/modes/flags × input shapes that the C actually branches on, in the
   format (last column `[x]` once its randomized test passes):

   `| <area>-1 | crypto_foo_bar, crypto_foo_bar_final | key=NULL, outlen=16, 3 chunked updates | [x] |`

   Cover the FULL set of public entry points in your area, including the
   low-level/streaming ones, not just one-shot wrappers.
3. `$W/_v/errors/<AREA>.md` — ERROR-SURFACE table fragment: markdown rows only,
   one row per DISTINCT rejection site found by grepping your C files for
   `return -1`, `return NULL`, `goto`-to-error, `sodium_misuse`, `assert`,
   explicit range/size checks, `_MIN`/`_MAX` constants, etc. Format:

   `| <area>-E1 | crypto_foo_bar | outlen > crypto_foo_bytes_max | returns -1 | [x] |`

   Every row needs a test in your `.rs` file (except abort/misuse rows, marked
   `[abort]`).

Derive rows mechanically from the C source. Do not guess, do not skip the
"unimportant looking" ones. Aim for completeness over brevity.

## Final report

Report: (a) counts — config rows, error rows, tests, all passing yes/no;
(b) every Rust divergence you found, with the exact input and the fix;
(c) anything you could NOT verify and why. Be honest — do not claim success for
work you did not run.
