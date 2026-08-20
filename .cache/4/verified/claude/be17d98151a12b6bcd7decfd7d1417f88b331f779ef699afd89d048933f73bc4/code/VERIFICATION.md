# VERIFICATION.md — status of the C → Rust translation of `pinflate`

`c_src/` is a single-translation-unit DEFLATE decompressor (`pinflate`, derived
from cute_png / cute_headers). `src/lib.rs` is its Rust translation. This file
is the completion gate; the detail lives in `SYMBOLS.md`, `ERRORS.md` and
`CONFIGS.md`.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** missing symbols in the Rust `.so`
      (8 of 8 exported, identical `st_size`/type/binding) and **0** undefined
      non-libc symbols. Enforced by `tests/phase_d_symbols.rs` (7 tests).
- [x] **Phase B** — all **40** rows of `CONFIGS.md` pass across randomized
      inputs. 31 tests in `tests/phase_b_valid.rs`.
- [x] **Phase C** — all **30** rows of `ERRORS.md` have a passing error-path
      differential test. 26 tests in `tests/phase_c_errors.rs`.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]` and
      `c_src/CMakeLists.txt` no `option()`/`CMAKE_BUILD_TYPE`, so there is
      exactly **one** build configuration. `verify/feature_matrix.sh` derives
      that from the files rather than assuming it, enumerates the powerset, and
      runs `cargo check --no-default-features --all-targets` + `cargo test` for
      each; `d7_no_feature_flags_exist` fails the build if a feature is ever
      added without extending the matrix. Additionally verified under the
      `release` profile.

`cargo test`: **64 passed, 0 failed.**

## How the comparison works

Every test loads **both** shared objects with `libloading` and calls their
exported symbols — the Rust implementation is never called directly, so the
`#[no_mangle] extern "C"` wrappers and the exported `static mut` tables are on
the critical path exactly as an external C caller sees them.

`cargo test` builds both `.so`s itself:

* C: `cmake -S c_src -B target/c_build -DCMAKE_POSITION_INDEPENDENT_CODE=ON`
  (nothing is written inside `c_src/`)
* Rust: `cargo build --lib --target-dir target/cdylib_build` (`cargo test` does
  not build `cdylib` output on its own)

Hostile input legitimately **aborts** or **hangs** both libraries, so every
comparison runs in a crash-isolated worker process (`examples/diffworker.rs`,
restarted by the parent after any case that kills it, `alarm()`-bounded). Each
case is compared on:

1. `pinflate`'s return value,
2. the `cp_error_reason` string read back through `dlsym`,
3. the **entire** output buffer — including the padding past `out_bytes`, so
   out-of-bounds writes are caught,
4. the fatal signal, **and** the `assert()` diagnostic
   (`lib.c:<line>: <func>: Assertion `<expr>' failed.`) scraped from the
   worker's stderr.

Point 4 matters: `SIGABRT == SIGABRT` alone would let two libraries that abort
for *different* reasons pass. Three of the mutations below are only caught
because the diagnostic is compared.

## Two findings that changed the translation

### 1. The reference C build has `assert()` **enabled**

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the build the task
prescribes gets no `-DNDEBUG` and `nm -D` shows `U __assert_fail@GLIBC_2.2.5`.
Ten `assert()`s in `lib.c` are therefore *caller-observable*: hostile input
prints a diagnostic and `abort()`s.

The translation had reduced them to comments. Measured on 250 fuzzed inputs,
**89 (36 %) diverged**: the C died with `SIGABRT` while the Rust returned
cleanly. `src/lib.rs` now reproduces each one via `cp_assert_fail()`
(diagnostic on stderr, then `std::process::abort()`), at the same source line,
in the same order — including `cp_decode`'s `assert((search >> len) == (key >>
len))`, where `len` can be 32 and the C shift is undefined; `objdump` on the
reference object file shows gcc 11 `-O0` emits a variable `shr %cl, %esi`, so
the count truncates modulo 32, reproduced as `search >> (len & 31)`.

### 2. `cp_dynamic`'s `lens[288 + 32]` overrun wedges the C library

None of the run-length arms in `cp_dynamic` clamps `n` against `nlit + ndst`, so
a run starting just below the limit writes up to 137 bytes past `lens` — over
`cp_dynamic`'s own locals, including the loop counter and `n` itself. The C
observably **loops forever** on such input (found by fuzzing, confirmed with a
traced build). `src/lib.rs` now models `cp_dynamic`'s stack frame as one byte
array laid out exactly as gcc 11 `-O0` emits it (`sub $0x190,%rsp`; offsets
verified against `objdump -d`), so `n`, `nlit`, `ndst`, the counters and
`lens[-1]` alias the same way and the Rust wedges identically.

`lens[-1]` — read by `case 16` when `n == 0` — lands on the most significant byte
of the spilled `s` pointer, which is always `0x00` on x86-64. Modelling the
frame reproduces that for free instead of guessing.

A third, smaller finding: Rust's dev-profile UB checks turned the C's `SIGSEGV`
on a `NULL in`/`out` into a Rust panic. `[profile.dev] debug-assertions = false`
restores C-faithful behaviour for `cargo build`/`cargo test`.

## Behaviour deliberately preserved, not "fixed"

The C is ground truth even where it is wrong. Reproduced verbatim, with tests:

* `cp_stored`'s length check `bits_left / 8 <= LEN` is inverted, so *any* stored
  block with input after it is rejected — a two-block stored stream never
  decodes (E2, `c4_stored_two_blocks`).
* `cp_stored` has **no** output bound check; `LEN` larger than `out_bytes`
  memcpy's past the end (E24).
* `cp_ptr` derives the payload address from `word_index`, which
  `cp_peak_bits`' final-word branch never advances, so stored blocks decode
  correctly only for some input sizes (E7, `assert_some_decode`).
* Length symbols 286/287 have `cp_len_base == 0`, so they produce a zero-length
  match that is silently accepted (E25).
* `cp_error_reason` is never cleared; a successful call leaves the caller's value
  in place (C32).
* `cp_build` returns `first[15]`, which excludes length-15 codes from
  `cp_decode`'s search range.

## Suite sensitivity

`verify/mutation_check.sh` injects 17 one-line divergences into `src/lib.rs` and
requires the suite to fail for each: **17/17 detected, 0 blind spots.** The
mutations cover `cp_rev16`, each error check, each assert (including
`> 0` weakened to `>= 0`), the frame-model offsets, the `memset` arm, the
`cp_stored` copy offset, a table entry, the input-alignment mask, the
final-word branch, and clearing `cp_error_reason`.

Two earlier iterations of the suite had blind spots that this script exposed and
that are now closed: comparing only the signal number (fixed by comparing the
assert diagnostic) and having no case where `bits_left` is *exactly* 0 (fixed by
constructing `00 00 00 FF FF`).

## Extra sweeps beyond the row tables

| sweep | cases | divergences |
|---|---|---|
| `verify/validonly.py` — real zlib streams, levels 0/1/2/6/9 × 5 strategies × 13 payloads (to 70 000 B) × 4 input × 3 output alignments | 3 900 | 0 |
| `verify/probe_fork.py` — fork-isolated fuzz: bit-flipped, truncated, random, and maximal-dynamic-header inputs, seeds 7/11/12/13/21/22/23 | 2 500 | 0 |
| the same sweeps against the **release** build | 3 900 + 900 | 0 |
| `cargo test` | 64 tests | 0 |

## Reproducing

```sh
cd translated_rust
ulimit -c 0                       # hostile input aborts both libraries by design
cargo test                        # builds both .so files and runs all 64 tests
bash verify/feature_matrix.sh     # every build configuration
bash verify/mutation_check.sh     # prove the suite has teeth (17/17)
```
