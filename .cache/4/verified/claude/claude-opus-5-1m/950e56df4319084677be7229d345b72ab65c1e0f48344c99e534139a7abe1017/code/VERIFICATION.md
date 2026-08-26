# VERIFICATION.md — differential verification report

Verifies that `src/lib.rs` (Rust) is byte-for-byte equivalent to
`c_src/src/driver.c` (C, the ground truth) across the FFI boundary.

Run everything with:

```
./run_verification.sh            # C build + every feature combo + all tests
DRIVER_TEST_CASES=4096 cargo test --offline   # heavier randomized pass
```

## What is under test

The library is one translation unit exposing one function:

```c
void driver(int x);                      // c_src/include/driver.h
static void print_hex(unsigned char *p, int len);   // file-local
```

`driver` `memcpy`s the object representation of `x` into `char raw[4]` and dumps
it as lowercase hex through libc `printf("%02x", …)`, followed by a newline
(which the C compiler lowers to `putchar('\n')`). There is no return value, no
output parameter and no error code — the entire observable behaviour is the byte
stream written to the C standard library's `stdout`.

## How the comparison is done

`tests/harness/mod.rs` loads **both** shared objects with `libloading` — the C
reference from `c_src/build/libdriver.so`, and the Rust cdylib discovered next to
the test binary (`target/debug/libdriver.so` after `cargo build`, or
`target/debug/deps/libdriver.so` when only `cargo test` was run) — and calls only
their exported `extern "C"` symbols, so the Rust `#[no_mangle]` wrapper is
exercised exactly as an external consumer would exercise it. The Rust functions
are never called directly from the test crate, and
`sanity_two_distinct_implementations_are_loaded` asserts the two `dlsym` results
are different addresses, so the suite cannot silently compare one library with
itself.

Both `.so` files write to the *same* process-global glibc `stdout`. To capture a
call's bytes, the harness `fork()`s and redirects fd 1 **in the child only**:

* the parent's fd 1 is never touched, so libtest's own progress output cannot
  leak into a capture — captures are correct under any `--test-threads` value
  (verified: 5/5 consecutive parallel runs green);
* the child's exit status is compared too, so C and Rust are also matched on
  *how they terminate* (clean exit vs. fatal signal), not just on what they print.

Both buffering regimes of the shared stream are covered: `stdout` on a regular
file (fully buffered) and on a pipe (row C19).

Every comparison is made two ways: **C vs. Rust** (mandatory) and **C vs. an
independently derived expectation** (`x.to_ne_bytes()` formatted as `%02x`,
endianness-agnostic because `to_ne_bytes` mirrors the C `memcpy`), which guards
against the harness silently comparing two empty captures.

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` inventory. C exports exactly one symbol, `driver`; the Rust `.so` exports it under the same name. **Symbol diff is empty.** No C source file was left untranslated (`CMakeLists.txt` compiles only `src/driver.c`), so nothing had to be newly translated. `print_hex` is `static` in C and correctly unexported on both sides. |
| `ERRORS.md` | Error-surface table. Mechanical grep finds **zero** rejection constructs in the C: no valued `return`, no `assert`, no `NULL`/`errno`/error enum, no range check; the only conditional in the library is the `i < len` loop bound. 10 rows (8 testable + 2 justified as not-applicable). |
| `CONFIGS.md` | Configuration-surface table. **Exactly one build configuration exists** (`Cargo.toml` has no `[features]`; `CMakeLists.txt` has no `option()`/`-D`; the C has no `#ifdef` beyond the include guard) and **no runtime option axis** (no flags, modes, globals or setters). 23 rows (C1–C22, incl. C7b) covering the input-shape and entry-point axes the C actually distinguishes. |

## Phase B — valid-path differential tests

All 23 `CONFIGS.md` rows pass (`tests/differential.rs`, 24 tests), driven with
randomized inputs from a fixed-seed xorshift64\* PRNG plus fixed boundary sets.
Coverage highlights:

* **exhaustive** per-byte coverage: all 4 × 256 (position, byte value) cells
  (C9–C13, C21);
* **exhaustive pairwise** coverage: all 6 byte-position pairs × all 2^16 value
  combinations × two filler values = 786,432 inputs (C22);
* a strided walk plus a uniform random sample over the whole 2^32 space (C7b);
* the low-level ABI shape — the symbol invoked through `fn(u32)` and `fn(u64)`
  dlsym signatures, not just `fn(c_int)` (C18);
* sequences of calls in one capture, including C and Rust calls interleaved in
  the same stream (C17).

### Coverage argument for the full 2^32 input space

`print_hex` formats each byte independently, so `driver`'s behaviour is fully
determined by (a) the 4 × 256 per-byte digit cells and (b) the absence of
interaction between byte positions. Rows C9–C13 compare (a) **exhaustively**;
row C21 builds that table from each `.so` and then asserts every output is
exactly the concatenation of its own cells, establishing (b); row C22 rules out
any pairwise interaction exhaustively. Together these extend the differential
result from the sampled inputs to the entire input space.

**Honest limitation.** A divergence that fires for one specific full 4-byte value
(e.g. only for `x == 0x5a5a1234`) and for no smaller-arity combination cannot be
found by sampling: a truly exhaustive sweep needs ~1.7 × 10^10 `printf` calls
per side (hours), and per-byte plus pairwise exhaustiveness is the practical
limit. This was measured, not assumed — see the mutation table below, where the
deliberately planted single-value backdoor is the one mutant that a 32k-sample
sweep missed while the pairwise-interaction mutant was caught precisely.

## Phase C — error-path differential tests

All 10 `ERRORS.md` rows are accounted for (`tests/errors.rs`, 8 tests; E9/E10 are
justified as not-applicable and that justification is itself asserted
mechanically against the C sources, so it cannot silently rot):

* E1/E2 — the loop bound never degenerates: every call emits exactly 9 bytes and
  the unconditional trailing newline, on both sides;
* E3–E7 — `INT_MIN` (no sign extension of the `0x80` byte), `-1`, `INT_MAX`,
  every high-bit byte value in every position, and the `%02x` zero-padding path;
* E8 — the generic FFI boundary: a value out of range for the declared `int`
  parameter (garbage in the upper 32 bits of the argument register, the analogue
  of an out-of-range enum for an API that declares no enum). Both sides must
  ignore the upper half identically;
* E9/E10 — no pointer, length or enum exists in the public API, so there is no
  null/oversize/invalid-variant argument to abuse. Asserted by parsing
  `driver.h` and checking that neither `.so` exports a second entry point.

## Phase D — symbol parity and feature combinations

* `tests/symbols.rs` re-runs the `nm -D` comparison as a test: the C→Rust
  exported-symbol diff is empty, the Rust `.so` has no undefined symbol outside
  libc/libgcc, and `print_hex` is exported by neither.
* Feature combinations are enumerated mechanically from `Cargo.toml` by
  `run_verification.sh`. There is **one** combination (the empty set); `cargo
  check --all-targets`, `cargo build` and the full test suite are run for it with
  `--no-default-features`. No `#[cfg(feature = …)]` gating was required because no
  feature exists and every module applies to the single configuration.

## Fault-injection validation of the suite

The suite was validated by planting bugs in the Rust and confirming they are
caught (each mutant reverted afterwards; `src/lib.rs` is byte-identical to its
original state):

| # | injected bug | caught by |
|---|--------------|-----------|
| 1 | signed (`c_char`) byte deref → sign extension | 18 valid-path rows + E1, E3, E4, E5, E6, E8 |
| 2 | byte order swapped | 17 valid-path rows + E1, E3, E5, E6, E7, E8 |
| 3 | trailing newline dropped | 20 valid-path rows + all 7 error rows |
| 4 | `%x` instead of `%02x` (padding lost) | 18 valid-path rows + 6 error rows |
| 5 | `len - 1` (3 bytes printed) | 20 valid-path rows + all 7 error rows |
| 6 | uppercase `%02X` | 17 valid-path rows + 6 error rows |
| 7 | reads the upper 32 argument bits | **E8 only** — no valid-path row detects it, which is exactly why the error-surface table is a separate gate |
| 8 | `abort()` for one input value | C5, C9 — via the child-exit-status comparison |
| 9 | pairwise-only bug (`byte0==0x12 && byte1==0x34`) | **C22**, pinpointed to input `0x00003412` (C `12340000` vs Rust `12350000`) |
| 10 | export dropped (`#[no_mangle]` removed) | `exported_symbol_diff_is_empty` |
| 11 | single-value backdoor (`x == 0x5a5a1234`) | *not caught* — the documented sampling limitation above |

## Result

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols in the Rust `.so`.
- [x] Phase B: all 23 `CONFIGS.md` rows pass across randomized inputs.
- [x] Phase C: all `ERRORS.md` rows have a passing error-path differential test.
- [x] Phase D: holds under every feature combination (there is exactly one).

**No divergence between the C and the Rust implementation was found.** The Rust
translation in `src/lib.rs` was not modified: it was already correct, including
the two details most likely to be mistranslated here — the `unsigned char` → `int`
promotion in `printf("%02x", p[i])` (no sign extension) and the use of the same
libc `printf`/`putchar` entry points, which keeps buffering and flush ordering
identical to the C.

Final test counts: 24 valid-path + 8 error-path + 3 symbol-parity = **35 tests,
all passing** (~12 s; ~67 s with `DRIVER_TEST_CASES=4096`).
