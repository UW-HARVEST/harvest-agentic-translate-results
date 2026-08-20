# VERIFICATION.md — completion gate

C-to-Rust differential verification of `c_src/src/{lib,main}.c` against
`src/{lib,ffi,main}.rs`.  The C code is the ground truth; nothing under `c_src/`
was modified.

## What is under test

| C artefact | Rust artefact | how it is compared |
|------------|---------------|--------------------|
| `libcdriver.so` (`cc -shared -fPIC c_src/src/lib.c`) | `target/<profile>/libdriver.so` (`crate-type = ["lib", "cdylib"]`) | both `dlopen`ed with `libloading`, `process_buffer` resolved with `dlsym`, called through the C ABI.  **No Rust function is ever called directly**, so the `#[no_mangle] extern "C"` wrapper in `src/ffi.rs` is part of the system under test. |
| `c_driver` (`cc c_src/src/main.c c_src/src/lib.c`) | `target/<profile>/driver` | both spawned as child processes with identical stdin; stdout, stderr and exit status compared byte for byte. |

`build.rs` produces both C artefacts into `OUT_DIR` on every build, so the
reference is always rebuilt from the pristine sources.

## Phase A — surface map

* `SYMBOLS.md` — `nm -D` on both objects.  The C `.so` exports exactly **one**
  symbol, `process_buffer` (all five helpers are `static`).  The Rust `.so`
  exports it under the identical name.  Nothing was stubbed; no C source file
  was left untranslated (`lib.c` → `src/lib.rs`, `main.c` → `src/main.rs`).
* `ERRORS.md` — 33 rejection rows + 4 documented-UB rows, derived by grepping
  every `return`, guard, default substitution, clamp and `fprintf(stderr, …)`
  in the C.
* `CONFIGS.md` — **90** configuration rows: 32 flag-cross-product rows (all
  combinations of the five meaningful `flags` bits) + 58 targeted rows
  (`rotate_buffer` 12, `compact_runs` 12, `remove_duplicates` 6,
  `interleave_halves` 6, `reverse_segments` 12, pipeline interactions 10),
  derived from every `if`/`?:` the C branches on, over the runtime options
  (`flags` bits, `param1` in its three different roles, `param2`) crossed with
  8 data shapes and the length classes the code special-cases.

## Phase B — valid-path differential tests

`tests/valid_paths.rs`, 89 tests, one per `CONFIGS.md` row, each driving many
randomised inputs from a fixed seed.  Both the return value **and every byte** of
the scratch buffer are compared — including the bytes past the returned length
and a 96-byte guard area, so stale data and over-writes are caught too.  The
padding byte is derived from the input (not a constant) so that a read past the
live region in only one implementation cannot accidentally match.

The harness also asserts, for the **C** object, the invariant the FFI wrapper
depends on: `process_buffer` never returns more than
`window(length, flags) = if flags & 0x02 { 2*length } else { length }`.

## Phase C — error-path differential tests

`tests/error_paths.rs` (24 tests) covers `ERRORS.md` rows 1-24 plus the generic
C-ABI boundaries: NULL pointers, zero and oversized lengths (up to
`usize::MAX`), one step past every documented range, and out-of-range
"enum" values across the FFI boundary (all 32 defined `flag` encodings plus
`0x20`, `0x8000_0000`, `0xFFFF_FFE0`, `0xFFFF_FFFF` and uniformly random
`u32`s, together with `INT_MIN`/`INT_MAX` for both mode parameters).

`tests/driver_cli.rs` (14 tests) covers rows 25-33: every `scanf` matching
failure, the `length > 256` rejection, missing data bytes, and the
`strtoul`/`strtol` saturation-and-truncation behaviour of `%u`, `%d` and `%zu`
(including `2^32`, `2^64`, `LONG_MIN-1` and negative inputs), plus non-UTF-8 and
embedded-NUL stdin.

## Phase D — symbol parity, feature combos

`tests/symbol_parity.rs` re-derives both symbol lists with `nm -D` at test time
and fails if any C symbol is missing from the Rust `.so`.  **Symbol diff: empty.**

`Cargo.toml` has **no `[features]` table** and `c_src/CMakeLists.txt` has no
build options / `#ifdef` switches (the C sources contain no `#if`/`#ifdef` at
all), so there is exactly one valid feature configuration.  `check_features.sh`
derives that set from `cargo metadata` (so the loop stays correct if features are
added later) and, for every combination, runs
`cargo check --no-default-features --features <combo> --all-targets`,
`cargo build` and the full test-suite.  It then repeats the suite for these
build configurations:

| configuration | result |
|---------------|--------|
| `--no-default-features` (the only combination) | 130/130 pass |
| `--all-features` | 130/130 pass |
| default feature selection | 130/130 pass |
| `--release` artefacts (`panic = "abort"`, overflow checks off, `opt-level=3`) driven through `DIFF_RUST_SO` / `DIFF_RUST_DRIVER` | 130/130 pass |
| C reference at `-O0` | 130/130 pass |
| C reference at `-O1` | 130/130 pass |
| C reference at `-O2` | 130/130 pass |
| C reference at `-O3` | 130/130 pass |
| C reference at `-Os` | 130/130 pass |

Sweeping the C reference across optimisation levels is a check on the *tests*:
if the suite depended on undefined behaviour, `-O0` and `-O3` would disagree.
They do not.

`cargo check --all-targets` from a clean target directory produces **0 warnings
and 0 errors**.

(`cargo test --release` itself cannot be used: cargo cannot link a
`panic = "abort"` cdylib into an unwinding test binary,
rust-lang/cargo#6313.  Hence the `DIFF_RUST_SO` / `DIFF_RUST_DRIVER`
indirection — the artefact under test is the release one either way.)

## Completion gate

- [x] **`SYMBOLS.md`**: `nm -D` shows **0** symbols exported by the C `.so` and
      missing from the Rust `.so`; **0** unresolved non-libc imports in the Rust
      `.so` (verified by `nm -D -u` *and* by `dlopen` succeeding).
- [x] **Phase B**: every one of the **90** `CONFIGS.md` rows passes across its
      randomised inputs (89 tests in `tests/valid_paths.rs`).
- [x] **Phase C**: every `ERRORS.md` row 1-33 has a passing differential test
      asserting the *same* sentinel / message / exit status (24 + 14 tests).
- [x] **All feature combinations**: the single valid combination
      (`--no-default-features` ≡ default ≡ `--all-features`) passes `cargo check
      --all-targets`, `cargo build` and the full suite, in both the `dev` and the
      `release` profile.

**Total: 130 tests, 0 failures, in every one of the 9 build configurations
listed above.**

## Divergences found and fixed

None in the translated algorithms — the pre-existing `src/lib.rs` matched the C
byte-for-byte on every case tried, including the C's quirks (`compact_runs`
writing the run *count* before shifting the tail, so the shifted data is the
already-overwritten byte; `rotate_buffer` rotating left for small offsets and
right for large ones; the length-growing behaviour at `threshold == 1`).

What *was* missing and had to be added for the library to be usable — and
testable — as a drop-in replacement for the C `.so`:

* `src/ffi.rs` — the crate exported **no** C ABI symbol at all, so the `.so`
  built from it exported none of the C `.so`'s symbols.  Added
  `#[no_mangle] pub unsafe extern "C" fn process_buffer` with the exact C
  signature, plus the `view_len` write-window rule the C code implicitly
  requires.
* `Cargo.toml` — added `crate-type = ["lib", "cdylib"]` (there was no `.so` to
  compare against) and the `libloading` dev-dependency.
* `src/lib.rs` — `#![forbid(unsafe_code)]` → `#![deny(unsafe_code)]` so that the
  single FFI boundary module can opt in; the translated algorithms remain
  completely `unsafe`-free.

## Suite sensitivity

16 non-equivalent single-token mutations injected into the Rust sources were all
detected; the 3 undetected mutations were proven semantically equivalent.  See
the table at the end of `CONFIGS.md`.

## Known-unverifiable inputs (C is undefined there)

`ERRORS.md` rows U1-U4.  In short: `param1 % (int)length` faults for
`length ≡ 0 (mod 2^32)` and for `param1 == INT_MIN, (int)length == -1`;
`rotate_buffer`'s large-offset branch smashes its own `uint8_t temp[256]` once
`length > 512`; and `compact_runs` with `threshold == 1` writes past a caller
buffer sized exactly `length` (which is what `main.c` itself does with its
`uint8_t buffer[256]`).  The first three need ≥ 2 GiB buffers or crash both
implementations identically; the fourth is *reproduced* by giving the Rust the
same `2 * length` window, and is excluded only from **CLI**-level comparison
(where the C prints unrelated stack bytes) — the library-level comparison covers
it exhaustively.
