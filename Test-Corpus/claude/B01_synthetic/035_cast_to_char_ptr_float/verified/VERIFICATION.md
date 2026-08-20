# Verification report

Differential verification of the Rust translation in `src/` against the C
ground truth in `c_src/`. The C implementation is authoritative; every
divergence found was fixed on the Rust side (or, in one case, in a test
expectation that had guessed the C's behaviour wrongly).

## What the program is

`c_src/CMakeLists.txt` builds one **executable** from one source file:

```cmake
add_executable(driver src/main.c)
```

`main.c` is 16 lines of code: `print_hex` (static), `driver`, and a `main`
that does `float x = 0.f; scanf("%f", &x); driver(x); return 0;`.

Almost all of the observable behaviour therefore comes from glibc's
`scanf("%f")` and the `strtof` conversion it performs, which the Rust
translation reimplements from scratch in `src/lib.rs`
(`scan_float` → `c_strtof` → `assemble_f32`). That is where verification was
concentrated.

## Phase A — surface mapping

| artifact | content |
|---|---|
| `SYMBOLS.md` | `nm -D` of the C executable (zero defined exports, seven libc/toolchain imports), the C translation unit's global symbols (`driver`, `main`) and static one (`print_hex`), and the shared-object symbol diff |
| `ERRORS.md`  | 25 rows — every distinct way the program rejects or errors on input, derived by grepping the C source and enumerating the `scanf`/`strtof` rejection branches |
| `CONFIGS.md` | 62 rows — the cross-product of runtime option/shape axes the C actually branches on, plus the build matrix |

## Phase B — valid-path differential tests

`tests/exe_diff.rs` (41 tests, CONFIGS.md rows 1–47, 57–58) drives both
programs end to end over stdin/stdout.
`tests/ffi_diff.rs` (CONFIGS.md rows 48–56) loads **both** implementations as
shared objects with `libloading` and calls the exported `driver` symbol, so
the `#[no_mangle] extern "C"` wrapper is exercised exactly as an external
caller would; the Rust side is never called directly. Because `driver` writes
to file descriptor 1, the harness (`tests/common/mod.rs::capture_fd1`)
redirects fd 1 and flushes both the Rust `io::stdout()` buffer and every C
`FILE*` buffer around each batch.

Rows 37–44 supply randomized inputs (2 000 per seed × 3 seeds each) from a
seeded SplitMix64 generator in `tests/common/corpus.rs`, so results are
reproducible.

## Phase C — error-path differential tests

`tests/error_paths.rs` (27 tests) has one test per `ERRORS.md` row. Each
constructs the exact invalid input, runs both programs, and asserts they agree
**and** that the shared result is the specific documented sentinel.

The sentinel needed care: because `main` discards `scanf`'s return value,
*every* rejection surfaces as the untouched initialiser `+0.0f`, i.e.
`00000000\n` and exit 0. Asserting only "both rejected" would be vacuous, so
rows 14–21 assert their own distinct non-zero sentinel (`0000807f` for
`+inf`, `00000080` for `-0`, `0000c07f` for the quiet NaN, `0000803f` for the
`"1e"` case, …). That is what makes a rejection distinguishable from a
successful conversion at all — e.g. `"0x1p"` must yield `1.0f`
(`0000803f`), not the rejection sentinel, which pins down that glibc backs the
dangling `p` out of the subject sequence instead of failing.

Generic boundaries also covered: null/closed stdin, zero and oversized
lengths (up to 65 KiB literals), one step past every documented range
(`FLT_MAX` vs next, `0x1p-149` vs `0x1p-150`, exact half-ulp ties), every
single byte value 0x00–0xFF alone and adjacent to a digit, and invalid UTF-8.
The API takes no enum — `driver`'s only parameter is `float` — so the
equivalent "value with no valid variant" check is a sweep over `f32` bit
patterns including all NaN payload classes and signalling NaNs.

## Phase D — symbol parity and the build matrix

`tests/symbol_parity.rs` (6 tests) asserts:

* the C executable exports **no** defined dynamic symbols and imports only
  libc/toolchain names, and the Rust executable matches;
* both executables define exactly `{driver, main}` as global text symbols;
* `print_hex` is local on both sides, as its C `static` requires;
* every symbol the C `.so` exports is exported by the Rust `cdylib`;
* the Rust `.so` has no unresolved symbols (`ldd -r`) and does not link
  against the C object.

To make the `.so` symbol diff reach **exactly empty**, `src/lib.rs` gained a
real C-ABI `main` export (a genuine translation of `int main()`, not a stub)
behind a new `c_main` feature — required because a Rust `[[bin]]` emits its
own `main`. With the feature on, `src/main.rs` is `#![no_main]` and the
exported `main` is the entry point, so the executable behaves identically.
`default = []`, so the default build is unchanged.

`./check_features.sh` parses the `[features]` table out of `Cargo.toml` and
`cargo check --all-targets` over its full power set × both profiles.
`./run_all.sh` re-runs the entire test suite over the same matrix:

```
ok    features=<none> profile=dev      (75 tests passed)
ok    features=<none> profile=release  (75 tests passed)
ok    features=c_main profile=dev      (75 tests passed)
ok    features=c_main profile=release  (75 tests passed)
RESULT: all feature combinations and profiles pass
```

Both profiles are swept because `[profile.release] panic = "abort"` makes them
genuinely different builds.

## Additional fuzzing beyond the test suite

`tools/` holds the standalone differential fuzzers used to search for
divergences the curated corpus might miss. All reported zero divergences.

| script | what it does | volume |
|---|---|---|
| `runall.py` + `gen_cases.py` | randomized literals, junk soup, round-trips, extreme exponents | ~21 000 inputs per seed × 13 seeds ≈ **270 000** |
| `brute.py` | **exhaustive** over all strings up to a given length from a chosen alphabet | length ≤ 3 over 17 chars (5 219), length ≤ 4 over 11 chars (16 104), length ≤ 5 over 7 chars (19 607) |
| `ties.py` | exact dyadic midpoints between consecutive `f32` values rendered exactly in decimal and hex, plus one decimal ulp either side — the strongest test of correct ties-to-even rounding | ~10 000 per seed × 6 seeds |
| `fuzz2.py` | targeted sweep of the boundary constants in the Rust conversion: the decimal `dp > 45`/`dp < -50` cutoffs, the 60-bit hex accumulator and its sticky bit, and `e_val > 127`/`e_val < -200`/`shift >= 64` in `assemble_f32` | ~64 000 per seed × 5 seeds |

## Divergences found

**None in the translation.** The one test failure encountered
(`row24_nul_and_non_ascii_bytes`) was a wrong expectation in the *test*: it
asserted `"1.5\0"` → `1.0f` when the C correctly yields `1.5f`. The C and
Rust outputs agreed; the expectation was corrected to match the C.

Three harness bugs were also fixed, none of them translation issues:

1. `cargo test` does not emit the `cdylib`, so `rust_so()` now builds it on
   demand with the running test binary's own profile and feature set (and
   rebuilds it when a previous run left one with different features).
2. `capture_fd1` redirects the process-wide fd 1, so it is serialized behind a
   mutex, and `ffi_diff.rs` drives all its rows from a single `#[test]` —
   otherwise libtest's own parallel progress lines interleave into the capture.
3. `#[cfg(all(feature = "c_main", not(test)))]` on the exported `main`, since
   libtest generates an entry point of its own.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust;
      the C→Rust `.so` symbol diff is empty under `c_main`.
- [x] Phase B: every one of the 62 `CONFIGS.md` rows passes across randomized
      inputs.
- [x] Phase C: every one of the 25 `ERRORS.md` rows has a passing error-path
      differential test asserting the same specific sentinel.
- [x] All of the above hold under every feature combination (2) and both cargo
      profiles (2) — 4 configurations, 75 tests each.

## Reproducing

```sh
# C reference build (exactly as CMakeLists.txt specifies)
cd c_src && cmake -S . -B build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build build

# every feature combination compiles
./check_features.sh

# the whole differential suite over the full build matrix
./run_all.sh

# extra fuzzing (needs the two binaries built)
cargo build --release
cd tools && python3 runall.py 20000 1 && python3 brute.py '01.xXeEpP+-inf a9' 3 \
          && python3 ties.py 7 1500 && python3 fuzz2.py 1
```
