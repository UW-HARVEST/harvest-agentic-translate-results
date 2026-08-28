# VERIFICATION.md — differential verification of the C→Rust translation

## Scope

The C library is a single translation unit with a single public function:

```c
/* c_src/include/lib.h */
const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
```

The Rust crate `translation/` re-implements it as a `cdylib` exporting the same
symbol. Verification is entirely differential: both `.so`s are loaded with
`libloading` and driven only through `dlsym("UTIL_createLinePointers")`. The
Rust crate is **never linked or called directly** — verified by
`grep -rn "use driver\|driver::\|extern crate driver" tests/ src/` → no matches.
So the `#[no_mangle] extern "C"` wrapper is itself under test.

## Result

**The translation is correct.** No divergence from the C was found in
1,084,082 FFI invocations per profile across every configuration. No change to
`src/lib.rs` was required (it is byte-identical to the delivered translation).

Two defects were found and fixed **in the test harness**, not in the
translation — see "Harness defects found and fixed" below. Both were the kind
that make a green suite meaningless, so they are the most important findings
here.

## Artifacts

| file | purpose |
|------|---------|
| `SYMBOLS.md` | Phase A/D — every `nm -D` export of the C `.so` and its Rust counterpart |
| `ERRORS.md` | Phase A/C — error-surface table, one row per distinct rejection in the C |
| `CONFIGS.md` | Phase A/B — configuration-surface table, one row per combination the C distinguishes |
| `tests/common/mod.rs` | harness: `.so` discovery + on-demand rebuild, observation/normalisation, PCG32 PRNG, independent reference model |
| `tests/differential.rs` | Phase B — 26 tests, one per `CONFIGS.md` row |
| `tests/errors.rs` | Phase C — 11 tests covering every `ERRORS.md` row + generic boundaries |
| `tests/robustness.rs` | allocation capacity, failure-path leak parity, guard-page over-read check, stability |
| `tests/alloc_size.rs` | exact `malloc` request-size parity via `malloc` interposition |
| `tests/symbols.rs` | Phase D — `nm -D` parity and import-set sanity |
| `run_verification.sh` | builds the C `.so`, enumerates feature combos, runs everything × {debug, release}, diffs symbols |
| `mutation_check.sh` | proves the suite can actually detect divergence (mutation testing) |
| `.mutation-summary.txt` | recorded output of the last mutation run |

## Build

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && ./run_verification.sh        # everything
```

`libloading = "0.8"` was added to `[dev-dependencies]`. Nothing else in
`Cargo.toml` changed; nothing in `c_src/` was modified (source mtimes unchanged;
only `c_src/build/` was added, as instructed).

## Completion gate

- [x] **`cargo check` clean** — 0 errors, 0 warnings, in every feature combination
      and both profiles.
- [x] **`SYMBOLS.md` / Phase D symbol parity** — the C `.so` exports exactly one
      global symbol, `UTIL_createLinePointers`; the Rust `.so` exports it under the
      identical name. `comm -23 c_syms rust_syms` is **empty**. Nothing was stubbed:
      `grep -rn "unimplemented!\|todo!\|panic!" src/` → no matches. The Rust `.so`
      has no unresolved non-libc imports, and it imports libc `malloc`/`free`
      (asserted in `tests/symbols.rs`) so its result is `free()`-able by C callers
      exactly like the C library's.
- [x] **Phase B — every `CONFIGS.md` row (1–26) passes** across randomized inputs
      (fixed seed `0x5DEECE66D`), including an exhaustive sweep of *every* NUL/non-NUL
      mask for `bufferSize ≤ 12` crossed with `numLines ≤ 14`.
- [x] **Phase C — every `ERRORS.md` row has a passing differential test**, asserting
      the same sentinel (`NULL` vs. non-`NULL` block), not merely "both failed".
      Rows 10 and 11 are the C's two genuinely undefined-behaviour inputs; they are
      documented and deliberately not executed (see `ERRORS.md`).
- [x] **All of the above under every feature combination.** `Cargo.toml` declares no
      `[features]` table, so the default set is the only real configuration;
      `run_verification.sh` nonetheless runs `<default>`, `--no-default-features` and
      `--all-features` × {debug, release} = 6 configurations. All 46 tests pass in all 6.
- [x] **The suite is proven able to detect divergence** (below).

Measured output of `./run_verification.sh`:

```
PASS  profile=debug   features=[<default>]              (46 tests, 0 warnings)
PASS  profile=release features=[<default>]              (46 tests, 0 warnings)
PASS  profile=debug   features=[--no-default-features]  (46 tests, 0 warnings)
PASS  profile=release features=[--no-default-features]  (46 tests, 0 warnings)
PASS  profile=debug   features=[--all-features]         (46 tests, 0 warnings)
PASS  profile=release features=[--all-features]         (46 tests, 0 warnings)
C exports: UTIL_createLinePointers
symbol diff: EMPTY (every C export is present in the Rust .so)
ALL CONFIGURATIONS PASSED
```

Measured FFI invocations per profile (printed by the harness at exit):

| test binary | invocations |
|---|---|
| `alloc_size` | 1,089 |
| `differential` | 1,041,772 |
| `errors` | 33,192 |
| `robustness` | 8,026 |
| `symbols` | 3 |
| **total** | **1,084,082** |

Stability: 5 consecutive full runs in debug and 3 in release, all 6/6 test
binaries green each time — no flakiness.

## Harness defects found and fixed

These are the substantive findings. Both produced a fully green suite that was
verifying nothing useful.

### 1. `cargo test` does not rebuild a `crate-type = ["cdylib"]` artifact

The harness originally located the Rust `.so` by looking in
`target/<profile>/`. But `cargo test` builds the lib only as a *test* binary; it
never produces `libdriver.so`. `target/debug/libdriver.so` did not even exist,
so the harness silently fell back to `target/release/libdriver.so` left over
from an earlier `cargo build --release` — **stale code**.

Proof it was vacuous: replacing the C's reconciliation check
`if (lineIndex != numLines)` with `if false` in `src/lib.rs` — a blatant
behavioural change — left the whole suite green.

Fix (`tests/common/mod.rs::rust_so_path`): the harness now runs a nested
`cargo build --lib` into a *separate* target directory
(`target/so-under-test/<profile>`, so it cannot deadlock against the parent
`cargo test`'s lock on `target/`), propagates the parent's feature selection via
`SO_UNDER_TEST_CARGO_ARGS`, and then asserts the artifact is newer than every
`src/**/*.rs` (`assert_fresh`, message `STALE ARTIFACT`). After the fix the same
`if false` mutation fails 3 test binaries.

### 2. `malloc_usable_size` is not a function of the request size

The first attempt at verifying that both libraries issue the same
`malloc(numLines * sizeof(const char**))` compared
`malloc_usable_size(result)`. That is unsound: glibc may satisfy a request out
of a larger free chunk, so the usable size depends on heap *state*. Observed
directly: for `numLines = 31` (a 248-byte request) the C block reported 248 and
the Rust block 264 — a **false positive** that "caught" two mutants which are in
fact semantically equivalent.

Fix: `tests/robustness.rs` now asserts only the heap-state-independent lower
bound (`usable ≥ numLines * 8`, i.e. the array is big enough to hold what gets
written), and exact request-size parity moved to `tests/alloc_size.rs`, which
**interposes `malloc`** in the test executable. Because the executable is first
in the dynamic symbol lookup scope, the `malloc@plt` call inside *both* dlopened
`.so`s resolves to the recorder; the real allocator is reached via glibc's
`__libc_malloc` alias, so there is no recursion and blocks stay `free()`-able.
The interposition is self-validating: the test first asserts that it actually
observed the C library's `malloc` and that the recorded size is `5 * 8` for
`numLines = 5`.

This gives an exact measurement, and it confirms the C's **wrapping** `size_t`
multiplication is reproduced: for `numLines = 2^61` both libraries request
`0` bytes, for `2^61 + 1` both request `8`, for `SIZE_MAX` both request
`SIZE_MAX - 7`.

## Suite sensitivity (mutation testing)

`./mutation_check.sh` injects each mutation into `src/lib.rs`, verifies it landed,
rebuilds, and requires the suite to fail. Run over **both** profiles:

```
==> profile debug:   suite is SENSITIVE and PRECISE   (22 caught, 0 missed, 0 false positives)
==> profile release: suite is SENSITIVE and PRECISE   (22 caught, 0 missed, 0 false positives)
```

Caught: `wrong-sizeof`, `oversized-sizeof`, `no-free-on-failure`,
`inner-bound-off-by-one`, `outer-numlines-bound`, `outer-bufsize-bound`,
`drop-malloc-null-check`, `drop-count-reconciliation`, `count-reconciliation-lt`,
`split-on-newline`, `pointer-off-by-one`, `skip-two-past-terminator`,
`saturating-mul`, `checked-mul-panic`, `use-rust-allocator`, `len-starts-at-one`,
`skip-guard-off-by-one`, plus five deliberate coverage probes that only trigger
on rare shapes — `COVERAGE-large-buffer` (buffers > 200 B),
`COVERAGE-large-numlines` (`numLines > 500`), `COVERAGE-empty-line`,
`COVERAGE-high-bit` (bytes ≥ 0x80, i.e. negative `char`), `COVERAGE-zero-numlines`.
All five are caught, which is direct evidence the suite's *breadth* is real and
not concentrated on tiny happy-path inputs.

Two mutants are **provably semantically equivalent** and correctly do *not* fail
the suite (verified by exhaustive enumeration: every NUL mask for
`bufferSize ≤ 14` × `numLines ≤ 17` = 589,806 inputs, **0 divergences**):

* `unconditional-terminator-skip` — after `pos += len`, `pos ≤ bufferSize` always
  holds; when `pos == bufferSize` the extra `pos += 1` cannot be observed because
  the outer guard `pos < bufferSize` already fails.
* `inner-bound-off-by-one-other-way` — stopping the scan one byte early only
  shortens `len` (not observable) in exactly the cases where the following
  `if pos < bufferSize { pos += 1 }` adds the byte back.

## What is compared, and why that is byte-exact

`assert_same` hands **one shared allocation** to both libraries, so the returned
arrays must be **bit-identical**, not merely equivalent modulo a base address.
For every call it asserts:

1. identical `NULL`-ness of the return value;
2. bit-identical `numLines` stored pointers (`oc.raw == or.raw`);
3. identical offsets relative to `buffer` (redundant, kept for diagnostics);
4. the caller's buffer is unmodified (the C never writes to it, so neither may
   the Rust);
5. every returned pointer lies inside `[buffer, buffer + bufferSize)`.

`assert_same_and_model` additionally checks the C against an **independent
re-derivation** of the algorithm (`common::model`), so a shared misreading of
the C cannot hide behind "both agree".

Beyond the pointer array, three properties invisible to that comparison are
checked separately:

* **exact `malloc` request size** — `tests/alloc_size.rs` (interposition, above);
  also asserts exactly **one** `malloc` per call, which rejects a translation
  that allocates twice or routes through Rust's global allocator.
* **failure-path `free`** — `c_src/src/lib.c:29` frees before returning `NULL`.
  `failure_path_frees_its_allocation` issues 512 failing calls that each request
  8 MiB and asserts RSS/VmPeak growth stays far below the 4 GiB a missing `free`
  would cost. (This is what caught `no-free-on-failure`.)
* **no read past `bufferSize`** — `never_reads_past_buffer_size` places the
  buffer so its last byte is the last byte of a mapped page, with the next page
  `PROT_NONE`. Any over-read faults. The guard layout was independently proven
  to trap: a standalone C probe reads the last in-window byte successfully and
  dies with SIGSEGV one byte past.

## Notes on the C's behaviour that the Rust reproduces

Recorded because each looks like a bug but is the ground truth:

* The function splits on `'\0'`, **not** on newlines, despite the name
  `UTIL_createLinePointers` (`cfg_17` pins this down).
* `numLines * sizeof(const char**)` is a plain wrapping `size_t` multiply.
  `numLines = 2^61` therefore requests `malloc(0)`, which **succeeds** on glibc,
  so the `NULL` comes from the later `lineIndex != numLines` check rather than
  from the allocation check (`ERRORS.md` rows 3–4).
* `numLines == 0` returns the **non-NULL** `malloc(0)` pointer, which the caller
  must still `free()` (`ERRORS.md` row 9).
* `buffer` is never null-checked, but a null `buffer` with `bufferSize == 0` is
  safe because the loop guard fails first (`ERRORS.md` rows 8–9).
* The last "line" need not be NUL-terminated; a segment that runs to the end of
  the buffer still counts, and the terminator-skip is then suppressed
  (`CONFIGS.md` rows 5, 6, 9).
* On failure the C frees the array and returns `NULL`, discarding the pointers
  it already wrote.
* `char` is signed on x86-64, so bytes ≥ 0x80 compare as negative but are still
  `!= '\0'` (`CONFIGS.md` row 16, `COVERAGE-high-bit`).

## Known undefined-behaviour inputs (not executed)

`ERRORS.md` rows 10 and 11. The C dereferences `buffer[pos+len]` without knowing
the real allocation size, and writes `linePointers[lineIndex]` without knowing
whether the size multiplication wrapped. `bufferSize = SIZE_MAX` with a small
buffer, and a wrapped size with `bufferSize > 0`, are therefore UB. The Rust
mirrors the C's instruction sequence for these (`wrapping_mul` for the size,
`wrapping_add` + `read()` for the scan, plain `add` for the store), so it faults
in the same place, but running them would abort the test process, so they are
documented rather than executed.
