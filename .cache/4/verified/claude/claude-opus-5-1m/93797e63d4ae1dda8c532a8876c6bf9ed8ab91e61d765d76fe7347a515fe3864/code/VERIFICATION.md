# VERIFICATION.md — differential verification of the C→Rust translation

## What this project is

`c_src/CMakeLists.txt` builds an **executable** (`add_executable(driver src/main.c)`)
from a single 82-line translation unit. There are therefore:

* **no build-time configurations in the C**: no `#if`/`#ifdef` (`grep -c '#if' c_src/src/main.c` → 0),
  no CMake `option()`, `CMAKE_BUILD_TYPE` empty (so the reference is built with
  no optimisation flags);
* **no Cargo features**: `Cargo.toml` has no `[features]` section, so the full
  set of valid feature combinations is `{default}` ≡ `{--no-default-features}`
  ≡ `{--all-features}` — all three are still run, so this is verified and not
  assumed.

Because the deliverable is an executable, the comparison happens on two levels:

| level | C artefact | Rust artefact | what is compared |
|---|---|---|---|
| FFI | `c_build/libcdriver.so` (`gcc -fPIC -shared c_src/src/main.c`) | `target/<profile>/libdriver.so` (`crate-type = ["cdylib"]`) | both `.so`s are `dlopen`ed with `libloading` and their exported `run` / `main` called through the C ABI; stdout captured via an fd-1 redirect, the `house_t` compared bit-for-bit, exit status / fatal signal compared |
| process | `c_src/build/driver` | `target/<profile>/driver` | same stdin → stdout, stderr, exit status and terminating signal compared byte for byte |

The Rust implementation is **never** called directly from the tests — always
through `dlsym` on the `.so` (which also tests the `#[no_mangle] extern "C"`
export wrappers) or by spawning the executable.

Crate layout that makes this possible:

* `src/imp.rs` — the translation itself (shared by both targets).
* `src/main.rs` — the `driver` binary (+ the SIGPIPE fidelity fix, see below).
* `src/lib.rs` — `cdylib` exporting `run` and `main`, exactly the C `.so`'s surface.

## Phase A — surface mapping

| artefact | content |
|---|---|
| `SYMBOLS.md` | `nm -D` on both `.so`s: the C `.so` defines exactly `main` and `run`; the Rust `.so` defines exactly `main` and `run`. Symbol diff empty, no non-libc undefined symbols. |
| `ERRORS.md` | 21 rows, derived by grepping every `return`/`errno`/`if`/bound in the C: the 4 independent conditions of the single `parse_val` guard, the ignored `fgets` return value, the 99-byte truncation, embedded NULs, the unchecked pointer dereference, the signed-overflow spots, plus the generic FFI boundaries. |
| `CONFIGS.md` | 8 axes the C branches on and 48 rows over their pruned cross-product (17 `run` rows, 26 stdin rows, 5 exhaustive sweeps). |

`./check_symbols.sh <profile>` re-checks symbol parity mechanically.

## Phase B — valid-path differential tests

* `tests/ffi_run.rs` — rows R01–R17 through the **lowest-level** exported entry
  point `run(house_t*, int)`: integer edges (`INT_MAX`/`INT_MIN` `++` and `+=`
  overflow, full 7×7 cross-product), and the `%.1f` surface (exact one-decimal
  values, exact ties, near-ties, ±0.0, subnormals, the 2⁵³/10 neighbourhood,
  `1e300`/`DBL_MAX`, ±Inf, NaN payloads), single call and the two consecutive
  calls `main` performs. ~48 000 randomised cases from three generators
  (uniform bit patterns, "mixed" magnitudes, `±m/10^d` decimals) with fixed
  seeds.
* `tests/cli_diff.rs` — rows M01–M21 through the exported `main` **and** both
  executables: sign forms, every whitespace class, leading zeros, trailing
  garbage, `int`/`long`/beyond-`long` ranges, the 96–150-byte length sweep
  around the 99-byte `fgets` cap, multi-line stdin, embedded NULs, high bytes,
  CRLF, 100 KiB inputs, and ~2 700 randomised inputs.
* `tests/env_diff.rs` — rows M22–M26: locale environment (9 locales × 4 inputs),
  argv variants, stdout to a regular file, stdin from a regular file, and stdin
  arriving in slow chunks across several `read()` syscalls.
* `tests/exhaustive.rs` — rows S01–S05: all 256 byte values alone / before /
  after a number, all 484 two-byte and all 4096 three-byte strings over the
  interesting alphabets, and every digit length 1…25 × sign × newline plus all
  the `int`/`long`/`unsigned` boundary texts ±2.

Every batch asserts the C side really produced the expected shape of output
(4 lines per `run`, 8 lines or the error line per `main`), so a mismatch can
never be hidden by an empty capture.

## High-volume pass — `tests/heavy.rs` (`#[ignore]`d, run explicitly)

Beyond the per-row randomisation, a one-off heavy pass was executed and passed:

| test | volume | result |
|---|---|---|
| `heavy_random_bit_patterns` | 200 000 uniformly random `f64` bit patterns × random `int` fields through `run` | identical |
| `heavy_random_decimalish` | 200 000 random `±m/10^d` doubles | identical |
| `heavy_random_mixed` | 200 000 random mixed-magnitude doubles | identical |
| `heavy_all_one_decimal_values` | every `k/10` for k ∈ [-200 000, 200 000] (400 001 values) | identical |
| `heavy_random_stdin` | 50 000 random stdin byte strings through both executables | identical |

```sh
cargo test --test heavy -- --ignored --test-threads=1   # ~3 min
```

## Phase C — error-path differential tests

`tests/errors.rs` has one test per `ERRORS.md` row (E1–E21) and each test also
asserts that the **C reference** actually rejected (or actually accepted) the
input, so no row can pass vacuously. Covered rejections: `endp == str` (5
distinct trigger classes), `errno == ERANGE` (both signs), `> INT_MAX`,
`< INT_MIN`, `LONG_MAX`/`LONG_MIN` exactly, off-by-one around every boundary,
99-byte truncation changing the outcome, embedded NUL, 0-byte and 100 KiB
stdin, closed stdin (EBADF), stdout pipe with no reader (SIGPIPE), NULL and
misaligned `house_t*`, and the signed-overflow paths of `floors++` / `bedrooms +=`.

## Independent audits (adversarial, by separate agents)

Two independent audits were run against the translation, each asked to *find*
divergences rather than confirm correctness, and each allowed to compile its own
glibc reference programs:

1. **`fgets` / `strtol` audit** — transcribed the Rust `fgets`, `strtol10`,
   `parse_val` and `program_main` into C mirrors and diffed them against real
   glibc: 16.6 M exhaustive `parse_val`-vs-`strtol` cases (all byte strings of
   length 1–3), 51.2 M full `(value, endptr, errno)` triples, 268 K `fgets`
   comparisons over a real pipe (all 65 536 byte pairs, lengths 95–105 with
   `\n`/NUL at every position), 826 K end-to-end runs and 9 094 runs against the
   shipped `c_src/build/driver`. **Result: 0 divergences.** It confirmed glibc's
   base-10 `strtol` never sets `errno` to anything but `0`/`ERANGE` (so modelling
   the `errno == 0` guard as `!erange` is sound) and that `fgets` NUL-terminates
   only when it stored at least one byte.
2. **`%.1f` audit** — read glibc's `__printf_fp`/`round_away` and Rust's
   `flt2dec` Dragon4 rounding step and established that *both* round
   half-to-even on the exact binary value, then fuzzed ~280 M doubles against
   real glibc: every `k/10 ± 1 ulp` for k ≤ 2·10⁶, every `k/2^j` for j ≤ 7,
   ±3000 doubles around every power of two, 800 K consecutive doubles straddling
   2⁵³/10, 20 M consecutive doubles in the half-ulp-tie binade, 40 M exact ties
   `v = odd/4`, and 80 M random bit patterns. **Result: 0 divergences**, and it
   proved the fast path's `< 2^53` guard is exactly the right cut (removing the
   guard produced 11 150 320 mismatches in the same corpus, i.e. the guard is
   load-bearing and correct). It also confirmed `{:.1}` uses exact (not
   shortest) mode, so huge values print glibc's full 300+ digit expansion.

Every candidate input/value either audit produced was then re-checked through
the *real* differential harness (both `.so`s and both executables) in
`tests/audit_regressions.rs` — 7 tests, all passing: the 41 stdin candidates
(99-byte `fgets` boundary, degenerate streams, embedded NULs, range boundaries,
syntax edges) and all 77 `f64` bit patterns × their sign-flipped variants × 3
integer contexts × {one call, two calls}.

The one substantive remark from the audits (a comment in `imp.rs` that described
Rust's `{:.1}` rounding mode incorrectly — it is half-to-even, not half-away-
from-zero) was corrected; it had no behavioural effect. The audit also noted
that `format_f64_1`'s fast path is behaviourally redundant (the fallback alone
is already byte-exact). It was deliberately left in place: it is byte-identical
to glibc on every input tested here (≈1.5 M differential cases) and on the
audit's 280 M-value corpus, and the values the program itself can reach
(2.5/3.5/4.5) go through it, so removing working, verified code would only add
risk.

## Divergences found and fixed

| # | symptom | fix |
|---|---|---|
| 1 | `run(NULL, x)`: C died with **SIGSEGV**, Rust (dev profile) aborted with **SIGABRT** because building `&mut *ptr` trips Rust's debug null-check | `src/lib.rs` now uses `ptr::read`/`ptr::write` instead of forming a reference — a plain load/store, so NULL faults like C and misaligned pointers work like C, in both profiles |
| 2 | stdout being a pipe with no reader: the C driver was killed by **SIGPIPE** (signal 13) while the Rust driver exited **0**, because Rust's runtime sets SIGPIPE to `SIG_IGN` before `main` | `src/main.rs` records the *inherited* disposition in an ELF `.init_array` constructor (which runs before the Rust runtime) and restores it as `main`'s first statement; the `.so` export deliberately does not, matching the C `.so` |
| 3 | `cargo test` could not link the lib target (the cdylib exports a C-ABI `main`, colliding with libtest's `main`) | `[lib] test = false` — the library is exercised through `dlopen`, not by linking |

The differential harness itself was mutation-tested: changing `{:.1}` to
`{:.2}` in the translation made 10/15 `run` rows fail, and perturbing the
fast-path digit made 15/15 fail, confirming both code paths of
`format_f64_1` are actually observed by the tests.

## Phase D — completion gate

Run `./verify.sh` (builds the C artefacts, then loops over every feature
combination × profile):

- [x] **`SYMBOLS.md`** — `nm -D` diff between the C `.so` and the Rust `.so` is
      empty in every configuration; `ldd -r` reports no undefined symbols
      (all imports are libc / libgcc_s).
- [x] **Phase B** — every row of `CONFIGS.md` (R01–R17, M01–M26, S01–S05)
      passes, each over its randomised / enumerated input set.
- [x] **Phase C** — every row of `ERRORS.md` (E1–E21) has a passing error-path
      differential test that also proves the C reference rejects/accepts as
      claimed.
- [x] **All feature combinations** — `{default}`, `{--no-default-features}`
      (and `--all-features` for `cargo check`) × `{dev, release}` profiles:
      82 tests, 0 failures in each of the four build configurations (plus the
      5 `#[ignore]`d heavy tests, ~1.05 M additional comparisons, run once).

```
$ ./verify.sh | tail -1
ALL CONFIGURATIONS PASSED
```

## Reproducing

```sh
./build_c.sh                    # c_src/build/driver + c_build/libcdriver.so
cargo build                     # target/debug/{driver,libdriver.so}
cargo test -- --test-threads=1  # fd-1 redirection is process-wide
./check_symbols.sh debug
./verify.sh                     # everything, for every configuration
```

`c_src/` is never modified; the shared object is built from it into `c_build/`.
