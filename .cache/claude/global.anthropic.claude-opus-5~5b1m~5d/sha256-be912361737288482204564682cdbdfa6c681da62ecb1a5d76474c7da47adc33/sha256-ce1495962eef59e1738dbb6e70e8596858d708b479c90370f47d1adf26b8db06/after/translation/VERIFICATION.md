# Verification report

Differential verification of the Rust translation of `c_src/src/driver.c`
against the C shared library. The C is the ground truth; every fix was applied
to the Rust side and `c_src/` was never modified.

## What the library is

A 4-function CWE-457 demonstration. `printIntPtrLine` prints `*p` with
`printf("%d\n", ...)`; `good()` prints `5`; `bad()` reads an **uninitialised**
`int *` and dereferences it; `driver(useGood)` dispatches to one or the other.

## Method

`tests/differential.rs` loads BOTH `.so` files with `libloading` and calls only
their exported C symbols — the Rust functions are never called directly, so the
`#[no_mangle] extern "C"` wrappers are themselves under test. Every call happens
in a child process (`examples/runner.rs`) so that a `SIGSEGV` is a comparable
observation instead of a lost test run; C and Rust are compared on
**(stdout bytes, exit code, terminating signal)**.

Both Rust build profiles (`debug` and `release`) are tested on every row, which
is what exposed the one real bug.

## Divergence found and fixed

**The debug build died with `SIGABRT` where the C reads the pointer.**

`printIntPtrLine` originally used a plain Rust deref, `*intNumber`. With
`debug-assertions` on, `rustc` emits UB-checks for null and misaligned
dereferences; those checks `panic!`, and a panic inside an `extern "C"` function
aborts the process. The result:

| input | C | Rust (debug, before fix) |
|---|---|---|
| misaligned pointer into a valid buffer | prints the value, exit 0 | **SIGABRT** |
| `NULL` | **SIGSEGV** | **SIGABRT** |
| unmapped address | **SIGSEGV** | **SIGABRT** |

Four tests failed: `cfg_16_print_misaligned_pointer`,
`err_01_print_int_ptr_line_null`, `err_02_print_int_ptr_line_unmapped_low`,
`err_04_print_int_ptr_line_misaligned`. The release build happened to pass,
so testing only `--release` would have missed this entirely.

**Fix:** the load is now issued as inline assembly (`load_c_int` in
`src/lib.rs`), which is invisible to rustc's UB-checks and therefore behaves
identically in *every* build profile — valid address yields the 4 bytes at that
address (unaligned permitted), invalid address lets the hardware raise `SIGSEGV`,
exactly as the C does. `read_volatile` and `read_unaligned` carry the same
preconditions and would not have helped. `[profile.dev] debug-assertions = false`
was added as defence in depth for the portable fallback path.

`mutation_check.sh` proves the fix is what provides profile-independence: with
`RUSTFLAGS="-C debug-assertions=yes"` forced on, the current code passes and the
plain-deref version fails.

## Note on the CWE-457 rows

`bad()` reads an indeterminate `int *`. There is no defined behaviour to match
byte-for-byte, and the C is not even self-consistent with itself — reaching
`bad()` directly and via `driver(0)` prints *different* garbage, because the
leftover stack contents differ. Byte-equality is therefore not the right
assertion for those rows. Instead the tests assert that **the defect is
preserved**:

* the outcome is either a fatal fault or a single garbage integer line — the two
  things an uninitialised pointer dereference can do;
* it is never `5` (which would mean `bad()` had been quietly replaced by
  `good()`);
* **anti-sanitization gate:** across both call paths, more than one distinct
  outcome must be observed. A translation that "fixed" the defect to any
  deterministic substitute — `data = null` (always `SIGSEGV`), `data = &0`
  (always prints `0`) — collapses this to a single outcome and is rejected.

Mutants 8, 9 and 10 in `mutation_check.sh` are exactly those three "fixes", and
all three are caught.

## Harness self-validation

A test suite that cannot detect an injected bug proves nothing, so
`mutation_check.sh` injects 12 deliberate bugs and confirms the suite fails on
each. All 12 are caught, plus the 2 UB-check robustness checks — **14/14, 0
survivors**:

byte/half-word truncation of `useGood`, inverted branch, wrong constant in
`good`, 8-byte instead of 4-byte load, byte-swapped value, off-by-one element
read, an added null guard, the three CWE-457 "fixes" above, a removed
`#[no_mangle]`, and the plain-deref UB-check divergence.

The harness also distinguishes a mutant that **failed to compile** from one that
survived — an early version silently scored a non-compiling mutant as
"survived", which would have overstated coverage.

## Staleness hazard

`cargo test` builds the test targets but does **not** re-emit the `cdylib`, so
the whole suite can silently pass against an outdated `.so`. That is how the
divergence above briefly appeared to be fixed when it was not. The suite now has
a staleness guard that fails loudly, and `verify_all.sh` sequences the builds
correctly.

## Results

| gate | result |
|---|---|
| `cargo check` | clean, no errors or warnings |
| `nm -D` symbol parity (debug + release) | **0 missing**, 0 undefined non-libc |
| Phase B — all 34 `CONFIGS.md` rows | pass, randomized, fixed seed |
| Phase C — all 11 `ERRORS.md` rows | pass |
| Feature combinations | no `[features]` declared; default and `--no-default-features` both pass |
| Flakiness | 12 consecutive suite runs, 0 failures |
| Mutation self-check | 14/14 caught, 0 survivors |

31 tests, all passing under both build profiles.

```sh
bash verify_all.sh     # everything above, end to end
```
