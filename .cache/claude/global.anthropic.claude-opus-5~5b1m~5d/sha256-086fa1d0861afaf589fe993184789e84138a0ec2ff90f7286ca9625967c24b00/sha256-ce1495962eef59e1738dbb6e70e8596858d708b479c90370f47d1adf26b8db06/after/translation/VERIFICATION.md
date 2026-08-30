# Verification report

Differential verification of the Rust translation in `translation/` against the
C ground truth in `c_src/`.

**Verdict: the Rust translation is byte-for-byte equivalent to the C across
every configuration and error path enumerated from the C source. No divergence
was found, so no change to `src/lib.rs` was needed.**

## How to reproduce

```bash
# build the C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# everything: symbol parity + Phases B/C x {debug,release} x {feature combos}
cd translation && ./verify.sh

# prove the test suite is actually sensitive (injects 25 bugs, all must be caught)
cd translation && python3 mutation_check.py
```

## Method

Both libraries are loaded as shared objects with `libloading` and driven
**only** through their exported C symbols, so the `#[no_mangle] extern "C"`
wrappers are part of what is under test. The Rust crate is never linked or
called directly.

`driver.c`'s `static house_t the_house` is process-global and mutated by every
call, so each loaded `.so` carries its own accumulating state. Tests therefore
execute one logical call at a time under a global mutex: redirect fd 1, call C,
restore, redirect, call Rust, restore, compare. Both libraries observe the
identical call sequence and so remain in lock-step regardless of test order.
`errno` is reset to a known value before each side. `translation/.cargo/config.toml`
pins `RUST_TEST_THREADS=1`, and the harness asserts it, because concurrent
libtest progress output would otherwise contaminate the fd-1 capture.

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | Phase A symbol surface; `nm -D` diff for both `.so`s |
| `ERRORS.md`  | Phase C error-surface table (18 rows) + row→test map |
| `CONFIGS.md` | Phase B configuration-surface table (28 rows) + row→test map |
| `tests/common/mod.rs` | lock-step differential harness, stdout capture, fork probe, seeded PRNG |
| `tests/phase_b_configs.rs` | `CONFIGS.md` rows 1–22 |
| `tests/phase_b_deep_state.rs` | `CONFIGS.md` rows 23–24 (deep global state) |
| `tests/phase_b_env_axes.rs` | `CONFIGS.md` rows 25–28 (locale, threads, const-correctness) |
| `tests/phase_c_errors.rs` | `ERRORS.md` rows 1–18 + generic boundaries |
| `verify.sh` | symbol parity + full suite per feature combo per profile |
| `mutation_check.py` | sensitivity check: 25 injected bugs |

## Results

### Phase A / D — symbol parity

The C `.so` exports exactly two symbols, both present in the Rust `.so`:

| symbol | C | Rust | source |
|--------|---|------|--------|
| `driver` | `T` | `T` | declared in `include/driver.h` |
| `run` | `T` | `T` | not in the header, but non-`static` in `driver.c`, so part of the ABI — tested directly as the low-level entry point |

* Symbol diff (`comm -23`) is **empty**.
* **0** undefined non-libc symbols in either `.so`.
* Neither `.so` leaks the C's `static` symbols (`the_house`, `parse_val`,
  `add_floor`, `add_bedrooms`, `add_floor_to_the_house`, `print_the_house`).
* `CMakeLists.txt` builds exactly one translation unit (`src/driver.c`), so no
  module could have been skipped — nothing had to be translated or stubbed.

### Phase B — 28/28 configuration rows pass

Every row runs many randomized inputs from a fixed seed
(`0x243F6A8885A308D3`, splitmix64), driving both the low-level `run` and the
`driver` wrapper: full-`int`-range arguments, `INT_MIN`/`INT_MAX` boundaries,
signed wraparound of `bedrooms`, 300- and 500-call state accumulation,
randomized interleavings of `run`/`driver`/rejecting-`driver`, and the whole
`strtol` accept-shape space (leading whitespace, `+`/`-`, leading zeros,
trailing garbage, base-10-only prefixes, 1 MiB inputs).

### Phase C — 18/18 error rows pass

All four falsifiable conjuncts of `parse_val`'s guard, both `ERANGE`
directions, both `int`-range violations (fixed *and* 500 randomized
out-of-range values), plus the generic FFI boundaries: NULL pointer (compared
via `fork`+`waitpid` — both fault with the same signal), zero length,
oversized lengths, one step past each documented range, arbitrary 32-bit
patterns in the only non-pointer parameter (the "invalid enum" analogue — this
API takes no enums), and `errno` both pre-poisoned and observed after return.

Also exhaustive rather than sampled where feasible: **all 2 380 strings** of
length 0–3 over `0192+- \t.xeE\n`, each classified by the C and required to
match.

### Sensitivity — 24/24 killable mutants caught

Passing tests only mean something if they can fail. 25 deliberate bugs were
injected into `src/lib.rs` one at a time:

```
mutants run: 25   caught: 24   MISSED: 0   provably-equivalent: 1   skipped: 0
```

Mutation testing found **two real blind spots** in the first version of the
suite, which is why rows 23–28 exist:

1. **`bathrooms` narrowed to `f32` survived.** `bathrooms` is only ever
   `k + 0.5`, which `f32` represents *exactly* below 2^23, so no shallow test
   could see the narrowing. Fixed by row 23, which drives `bathrooms` past
   2^23 (8 388 608 `run` calls, ~40 s) and steps across the limit one call at
   a time.
2. Three further classic translation errors were unreachable by the row tests
   and now have dedicated rows: `thread_local!` instead of `static mut` global
   state (row 27), `printf` reimplemented in Rust (row 25, caught only under
   `LC_NUMERIC=de_DE.utf8` where the separator is `,`), and writing through the
   cast-away `const char *` (row 28, caught with an `mprotect(PROT_READ)` page).

The one surviving mutant — deleting `parse_val`'s `errno == 0` check — is
**provably** behaviourally equivalent on LP64, not a gap: glibc's `strtol`
sets `ERANGE` only on overflow, when it returns `LONG_MAX`/`LONG_MIN`, and both
already fail the `int` range check. Verified by probing every digit-run of
length 1–500 for all 10 leading digits and both signs, all `long`/`int`
boundary literals, and 3 000 000 random byte strings: **0** inputs where
`errno != 0` while `tmp` is in `int` range. (The `errno` *reset* is not
redundant — removing it is caught.)

### Feature combinations

`Cargo.toml` declares **no `[features]` table** and no optional dependencies,
so the complete combination set is `default` and `--no-default-features`
(identical, since there is no `default` feature). `verify.sh` derives this from
`Cargo.toml` programmatically and would enumerate every non-empty subset if
features were added later. Both combinations pass symbol parity and the full
Phase B/C suite in **both** the `debug` and `release` profiles — the release
`.so` is differentially tested too, via `RUST_DRIVER_SO`.

## Notes on the translation

The translation is faithful, including several details worth calling out:

* It calls libc `printf`/`strtol`/`__errno_location` directly rather than
  reimplementing formatting or parsing, so it shares the very same `stdout`
  buffer and locale state as the C. This is why it is byte-identical under a
  comma-decimal locale.
* GCC rewrote the C's `printf("An error occurred\n")` into
  `puts("An error occurred")` — visible in `nm -D` — while the Rust keeps
  `printf`. Same bytes on the same stream, so not observable; confirmed by
  test.
* Signed-overflow UB in `floors++` and `bedrooms += extra_bedrooms` is
  translated as `wrapping_add`, matching the `addl` GCC emits.
* `int x;` in `driver` is uninitialised in the C and `0` in the Rust; it is only
  read when `parse_val` returned true and therefore wrote it, so this is not
  observable.

## Known limits of this verification

* **Concurrency is out of scope.** `the_house` is a plain non-atomic global, so
  concurrent calls are a data race in the C with no defined output ordering and
  no byte-for-byte oracle. All calls are serialised. (That the state is *shared*
  across threads rather than per-thread *is* tested — row 27.)
* **`floors` overflow is unreachable**, needing 2^31 `run` calls.
* **Non-NUL-terminated input buffers** are unbounded UB (out-of-bounds read)
  with no defined C result, so no differential assertion is possible; excluded
  by design, and noted in `ERRORS.md`.
