# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from the
branches the C actually takes, not from what looks important.

## Build-time configuration surface

### `Cargo.toml` features

```
$ grep -n '\[features\]' Cargo.toml
(no match)
```

There is **no `[features]` section** and no `default` feature, and no
`#[cfg(feature = ...)]` anywhere in `src/`:

```
$ grep -rn 'cfg(feature' src/ ; echo "exit=$?"
exit=1
```

Therefore the **complete** enumeration of valid feature combinations is the
single empty combination:

| # | feature combination | `cargo check` command | result |
|---|---------------------|-----------------------|--------|
| F1 | *(none — the only one)* | `cargo check --no-default-features` | clean, 0 warnings |
| F1′ | *(same set, reached differently — sanity checks that F1 really is exhaustive)* | `cargo check`, `cargo check --all-features` | clean, 0 warnings |

Phases B and C are run against F1 in **both** `dev` and `release` profiles.
`release` matters independently because `Cargo.toml` sets
`[profile.release] panic = "abort"`, and because `debug_assertions` changes
Rust's arithmetic-overflow behaviour — the CWE-190 truncation in `bad()` must
stay a wrapping `as` cast in both (rows C7/C8 below).

### `c_src/CMakeLists.txt` options

```
$ grep -nE 'option|add_definitions|target_compile_definitions|CMAKE_BUILD_TYPE|if *\(' c_src/CMakeLists.txt
(no match)
```

No `option()`, no `-D` definitions, no `#ifdef` in the C source other than the
`DRIVER_H_` include guard. The C library likewise has exactly **one**
configuration. There is no `#ifdef OMITBAD`/`OMITGOOD` pair (unlike many
Juliet-suite files) — both `bad` and `good` are always compiled in.

## Runtime configuration axes (what the C branches on)

| axis | source | distinct states |
|------|--------|-----------------|
| A1 `useGood` flag | `driver.c:91` `if (useGood)` | zero / nonzero |
| A2 entry point | `driver.h` + non-`static` defs | `driver`, `good`, `bad`, `printHexCharLine`, `printLine` (5) — A2 includes the **low-level** leaf functions, not just the `driver` one-shot wrapper |
| A3 `printLine` payload shape | `driver.c:32,34` (`%s`) | NULL / empty / 1 byte / many / high-bit bytes / embedded `\n` / embedded `%` / > `BUFSIZ` |
| A4 `printHexCharLine` value shape | `driver.c:40` (`%02x` + `char`→`int` promotion) | `0` / `1..15` (1 digit, needs zero pad) / `16..127` (2 digits) / `-1..-128` (sign-extended, 8 digits) / `CHAR_MIN` / `CHAR_MAX` |
| A5 call multiplicity & ordering | no state anywhere | single / repeated / interleaved across the two libraries |

`goodG2B` / `goodB2G` are `static`, so they are only reachable through A2 =
`good` or `driver(≠0)`; they are covered transitively (rows C5, C6, C9–C11).

## Configuration-surface table

Cross-product of A1–A5, pruned to combinations the C distinguishes.
Every row is exercised with **many randomized inputs** (seeded, deterministic
`SplitMix64`, seed `0x5EED_1234_ABCD_0001`) where the axis has a value domain,
and asserted byte-for-byte between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printHexCharLine` | **exhaustive** sweep of the entire `char` domain: all 256 values `-128..=127` (covers A4's zero / 1-digit / 2-digit / sign-extended / `CHAR_MIN` / `CHAR_MAX` classes in one pass) | [x] |
| C2 | `printHexCharLine` | 4096 seeded-random `char` values, each call captured & compared individually (value-dependent formatting) | [x] |
| C3 | `printLine` | seeded-random printable-ASCII payloads, lengths 0..=64 (A3 empty / 1 / many), 512 iterations | [x] |
| C4 | `printLine` | seeded-random **arbitrary-byte** payloads (0x01..=0xFF, no interior NUL), lengths 1..=256 — high-bit / invalid-UTF-8 / `%`-bearing / embedded-`\n` shapes arise naturally, 512 iterations | [x] |
| C5 | `printLine` | payload lengths at and around the stdio buffering boundary: 1, 2, 1023, 1024, 4095, 4096, 4097, 8191, 8192, 65536 bytes (A3 `> BUFSIZ`) | [x] |
| C6 | `bad` | no options; nullary. Fixed internal data (`CHAR_MAX`) ⇒ single path, but run repeatedly (A5) to prove statelessness | [x] |
| C7 | `bad` | same, built in **`release`** profile (`panic = "abort"`, `debug_assertions` off) — the CWE-190 truncation must stay wrapping | [x] |
| C8 | `bad` | same, built in **`dev`** profile (overflow checks **on**) — must not panic/abort | [x] |
| C9 | `good` | no options; nullary ⇒ exercises `goodG2B` (`data=2`, arithmetic performed) **and** `goodB2G` (`data=CHAR_MAX`, arithmetic rejected) in one composed pipeline, in that order | [x] |
| C10 | `driver` | A1 = zero ⇒ `bad()` branch. `useGood = 0` | [x] |
| C11 | `driver` | A1 = nonzero ⇒ `good()` branch. `useGood ∈ {1, -1, 2, 42, 256, 0x10000, INT_MAX, INT_MIN, 0xFFFFFF00u as i32}` + 1024 seeded-random nonzero `i32` | [x] |
| C12 | `driver` | 2048 seeded-random **unconstrained** `i32` (both truthiness classes mixed, so the zero/nonzero decision boundary is hit from both sides in one stream) | [x] |
| C13 | `driver` ∘ `good` ∘ `bad` | equivalence composition: `driver(0)` output ≡ `bad()` output, and `driver(k≠0)` output ≡ `good()` output, for random `k` — checks the dispatch wiring, not just each leaf | [x] |
| C14 | `good` ∘ `printHexCharLine` ∘ `printLine` | composition: `good()`'s bytes ≡ `printHexCharLine(4)` bytes followed by `printLine("data value is too large to perform arithmetic safely.")` bytes — pins the exact internal constants (`2*2`, the diagnostic text) reached through the low-level API | [x] |
| C15 | all 5 exports | A5 interleaved random call sequence: 3000 randomly-chosen calls with random arguments, captured as **one** contiguous stdout stream per library and compared as a whole (catches state/buffering divergence that per-call tests hide) | [x] |
| C16 | all 5 exports | A5 same random sequence, but the C and Rust calls **interleaved into a shared stdout stream** — proves both libraries drive the *same* libc `FILE` with the same buffering, and that neither leaves the stream in a different state | [x] |

## Run log

`./run_all_configs.sh` enumerates the configurations, re-checks that the
enumeration is still exhaustive (no `[features]` section, no CMake options, no
`cfg(feature = ...)` in `src/`), then runs `cargo check` and the full
differential suite for each, and finally diffs `nm -D`:

```
############ enumerating configurations ############
Cargo.toml: no [features] section -> exactly 1 feature combination
c_src/CMakeLists.txt: no options/defines -> exactly 1 C configuration
src/: no cfg(feature = ...) gates

############ phase A(2): cargo check every combination ############
PASS  cargo check --no-default-features <dev>       (warnings: 0)
PASS  cargo check --no-default-features --release   (warnings: 0)
PASS  cargo check <default> <dev>                   (warnings: 0)
PASS  cargo check <default> --release               (warnings: 0)
PASS  cargo check --all-features <dev>              (warnings: 0)
PASS  cargo check --all-features --release          (warnings: 0)

############ phases B/C/D: differential suite per configuration ############
PASS  cargo test --no-default-features <dev>       (56 tests passed)
PASS  cargo test --no-default-features --release   (56 tests passed)
PASS  cargo test <default> <dev>                   (56 tests passed)
PASS  cargo test <default> --release               (56 tests passed)
PASS  cargo test --all-features <dev>              (56 tests passed)
PASS  cargo test --all-features --release          (56 tests passed)

############ phase D: nm -D symbol parity ############
PASS  symbol diff empty: c_src/build/libdriver.so vs target/ffi-so/debug/libdriver.so
        bad  driver  good  printHexCharLine  printLine
PASS  symbol diff empty: c_src/build/libdriver.so vs target/ffi-so/release/libdriver.so
        bad  driver  good  printHexCharLine  printLine

OVERALL: PASS -- every configuration checks, tests clean, and symbols match.
```

Test-count breakdown (56 per configuration):

| file | tests | covers |
|------|-------|--------|
| `tests/phase_b_configs.rs` | 21 | rows C1–C16 of this table |
| `tests/phase_c_errors.rs`  | 26 | rows E1–E20 of `ERRORS.md` + 4 generic-boundary tests |
| `tests/phase_d_symbols.rs` | 9  | `nm -D` parity, undefined-symbol audit, `RTLD_NOW` load, `-O2` C cross-check |

Rows C1–C16 (Phase B) and E1–E20 (Phase C) all pass under the single feature
combination F1, in **both** profiles, so the Phase D completion gate "all of the
above under EVERY feature combination" is satisfied exhaustively.

Both profiles matter in practice, not just in principle: the release profile is
what exposed the one real translation bug this verification found (the
`printHexCharLine` `char`-parameter ABI divergence documented in `ERRORS.md`).

## Suite validity (negative control)

Matching symbols and passing happy-path tests are necessary but not sufficient,
so `./mutation_check.sh` injects 15 deliberate bugs into `src/lib.rs` and
requires the suite to fail for each, then confirms the pristine tree passes:

```
$ ./mutation_check.sh                        # dev
RESULT: all 15 mutations were caught by the differential suite.
pristine: PASS

$ PROFILE_FLAG=--release ./mutation_check.sh
RESULT: all 15 mutations were caught by the differential suite.
pristine: PASS
```

The script also refuses to count a mutation that failed to match its pattern
(`NO-OP MUTATION`) or that only edited a comment (`COMMENT-ONLY MUTATION`), both
of which would silently overstate coverage. See the table in `ERRORS.md` for
what each mutation was and which test caught it.
