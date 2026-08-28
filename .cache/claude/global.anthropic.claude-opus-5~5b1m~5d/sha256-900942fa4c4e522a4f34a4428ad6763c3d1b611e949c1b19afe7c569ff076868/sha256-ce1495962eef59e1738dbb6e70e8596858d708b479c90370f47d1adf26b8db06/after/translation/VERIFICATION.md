# VERIFICATION.md — differential verification of the Rust translation

Ground truth: `c_src/` (never modified). Subject: `translation/src/lib.rs`.

Every assertion below is produced by loading **both** shared objects with
`libloading` and calling their exported `encode_base64` symbol. The Rust
function is never called directly, so the `#[unsafe(no_mangle)] extern "C"`
export wrapper is itself under test.

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && ./check_features.sh     # all combos x both profiles
```

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | `nm -D` surface of both `.so`s; symbol diff |
| `CONFIGS.md` | Phase B configuration table (25 rows) |
| `ERRORS.md` | Phase C error-surface table (6 error rows + 7 boundary rows) |
| `tests/common/mod.rs` | loader, the `int` arithmetic model, the comparator, the seeded PRNG |
| `tests/phase_b_valid.rs` | 26 tests — one per `CONFIGS.md` row + a liveness check |
| `tests/phase_c_errors.rs` | 13 tests — one per `ERRORS.md` row |
| `tests/phase_d_negative_control.rs` | mutation testing: proves the comparator can fail |
| `check_features.sh` | Phase D driver (feature combos × profiles × symbol diff) |

## What the comparator checks

For each input it compares:

1. **NULL-ness** of the returned pointer (the API's only error sentinel), and
2. the **entire `calloc`'d region**, `n = size*4/3+4` bytes — so both the
   emitted base64 bytes *and* the zero padding that the C code relies on for
   NUL-termination are compared byte-for-byte.

`n` is recomputed in the harness with the same wrapping `int` arithmetic the C
uses, which is also how the harness decides whether a given `(size, buffer)`
pair is inside the C code's contract at all (see below).

## Results

```
Phase B  tests/phase_b_valid.rs             26 passed   ~66,000 differential call pairs
Phase C  tests/phase_c_errors.rs            13 passed   ~ 4,100 differential call pairs
Phase D  tests/phase_d_negative_control.rs   1 passed    6/6 injected bugs detected
                                            --------
                                            40 passed, 0 failed
```

Per feature combination × profile (`check_features.sh`):

```
combo=__default__ debug   : build PASS   symbols 0 missing   40 tests PASS
combo=__default__ release : build PASS   symbols 0 missing   40 tests PASS
combo=__none__    debug   : build PASS   symbols 0 missing   40 tests PASS
combo=__none__    release : build PASS   symbols 0 missing   40 tests PASS
combo=__all__     debug   : build PASS   symbols 0 missing   40 tests PASS
combo=__all__     release : build PASS   symbols 0 missing   40 tests PASS
```

**No divergence between the C and Rust implementations was found, so no change
to `translation/src/lib.rs` was required.** The translation already reproduced
every quirk of the C code, including the ones listed below. The only source
changes made were adding `libloading` to `[dev-dependencies]` and adding the
test files and artifacts.

### Why the debug profile is run too

`cargo build` (debug) turns on Rust's arithmetic overflow checks. The C code's
`size * 4 / 3 + 4` relies on `int` wrap-around for extreme `size` values; had
the translation used plain `*`/`+` instead of `wrapping_*`, the debug build
would panic where C silently wraps. Debug passes, so the wrapping semantics are
genuinely reproduced rather than accidentally matching under release codegen.

## C quirks confirmed preserved

| quirk | where | verified by |
|-------|-------|-------------|
| `size == 0` means "measure with `strlen`", not "empty input" | `lib.c:37` | C14–C17, G2, G3 |
| bytes after a NUL are invisible in strlen mode but are data in explicit mode | `lib.c:37` vs `lib.c:51` | C13 vs C16 |
| `char` is signed, so `0x80..0xFF` passes through a negative `char` into `unsigned char b1` | `lib.c:51` | C9, C15, C6 (all 65536 pairs) |
| padding is emitted from the *read* guards, so a short tail yields `=`/`==` | `lib.c:69,75` | C1–C4, C17 |
| `encode()`'s catch-all maps every sextet `>= 63` to `/`, and only `62` to `+` | `lib.c:17-21` | C11, C12 |
| the trailing NUL is never written — it comes from `calloc` zeroing | `lib.c:41` | whole-region comparison in every row |
| `n = size*4/3+4` is `int` arithmetic that wraps, then sign-extends into `size_t` | `lib.c:41` | C21–C24, E5, E6, G4–G6 |
| a negative `size` never enters the loop, so it returns an all-zero buffer instead of failing | `lib.c:48` | C21–C24, G5, G6 |
| `calloc(1, 0)` at `size == -3` still returns non-`NULL` | `lib.c:41` | G5, C21 |

## Inputs deliberately NOT invoked (out-of-contract C undefined behaviour)

For a positive `size` greater than the buffer the caller actually supplied, the
C code reads `src[0..size]` and writes `4*ceil(size/3)` bytes into an `n`-byte
buffer. When `size * 4` overflows `int` to a *small positive* value, `n` becomes
tiny and the C code overruns both buffers and crashes — e.g. `size = INT_MAX`
gives `n = 3`, and `size = 1073741821` gives `n = 0`. That is UB in the ground
truth rather than a behaviour the library defines, and a segfault inside the C
`.so` would kill the test process, so these values cannot be compared
differentially. `tests/common/mod.rs::is_well_defined` is the guard that keeps
the suite out of that region, and `g4_oversized_lengths` records the expected
`n` for each such value from the model instead. The neighbouring values where
`calloc`'s success/failure flips (`-3`/`-4`, `1073741820`/`1073741821`) *are*
compared, which pins the arithmetic on both sides of every boundary.

Everything else in the `int` domain is exercised: `size == 0`, `size` in range,
`size` smaller than the buffer, all negative `size` classes, and the positive
overflow range `[2^29, 1073741820]` where `calloc` fails before the loop.

## Robustness cross-check: optimization level

Because the C relies on signed-overflow wrap-around, the library was also built
at `-O0`, `-O1`, `-O2` and `-O3` and probed on all the overflow-sensitive
`size` values. All four produce **identical** NULL/non-NULL results, so the
Rust translation's `wrapping_*` semantics match the C at every optimization
level, not just at the unoptimized setting CMake defaults to
(`CMAKE_BUILD_TYPE` is empty in `c_src/CMakeLists.txt`).

## Negative control (why "all green" is meaningful)

`tests/phase_d_negative_control.rs` compiles six single-token mutants of the C
source into temporary `.so`s and runs them through the same comparator:

```
baseline (real Rust .so vs real C .so):   0 mismatches over 233 inputs
mutant m_slash  ("return '/'" -> '_')  :  40 mismatches   detected
mutant m_plus   ("return '+'" -> '-')  :  34 mismatches   detected
mutant m_digit  (digit branch off by 1):  103 mismatches  detected
mutant m_shift  ("b2 >> 4" -> ">> 3")  :  90 mismatches   detected
mutant m_pad    ("i+1 < size" -> "<=") :  32 mismatches   detected
mutant m_alloc  ("+ 4" -> "+ 5")       :  1 mismatch      detected
```

The comparator catches all six while reporting zero mismatches for the real
translation, so the passing suite is a real signal and not a vacuous one.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
      The C `.so` exports exactly one symbol (`encode_base64`); the Rust `.so`
      exports it with the identical name. The `static` helper `encode` is
      correctly absent from both dynamic tables. No module was left
      untranslated and nothing was stubbed.
- [x] Phase B: EVERY one of the 25 `CONFIGS.md` rows passes across randomized
      inputs (fixed seed `0x243F6A8885A308D3`).
- [x] Phase C: EVERY one of the 13 `ERRORS.md` rows has a passing error-path
      differential test asserting the same `NULL` sentinel, not merely "both
      failed".
- [x] All of the above hold under EVERY feature combination (the crate declares
      no features, so `--no-default-features` / default / `--all-features` are
      all verified) and under BOTH the debug and release profiles.
