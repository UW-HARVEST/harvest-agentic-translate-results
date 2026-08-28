# Verification report — `c_src/src/lib.c` → `translation/src/lib.rs`

## How to reproduce

```bash
# everything: builds the C .so, then runs every phase in every feature
# combination, for both the dev and release profiles
cd translation && ./run_verification.sh

# the ~84 M-call exhaustive sweeps (about 55 s)
cargo test --release --test fuzz -- --ignored
```

Logs land in `translation/target/verification/`.

## Test suites

| file | phase | what it does |
|---|---|---|
| `tests/common/mod.rs` | — | harness: `dlopen`s the C `.so` **and** the Rust `.so`, resolves `memchra2` (and the `harness_*` symbols) with `dlsym`, fixed-seed xorshift64\* PRNG. No Rust function is ever called directly. |
| `tests/differential.rs` | B | one test per `CONFIGS.md` row (32) |
| `tests/errors.rs` | C | one test per `ERRORS.md` row (34) + 1 bonus bitwise check |
| `tests/fuzz.rs` | B | 7 randomized/structured sweeps + 3 `#[ignore]`d exhaustive sweeps |
| `tests/symbol_parity.rs` | D | `nm -D` diff, `ldd -r` unresolved-symbol check, symbol kind check |
| `tests/feature_matrix.rs` | D | feature set is exactly what the matrix covers; behaviour is feature-independent |
| `tests/c_optimization_levels.rs` | B | recompiles the untouched C source at `-O0/-O1/-O2/-O3/-Os` and compares all of them plus the CMake build against Rust (guards against matching a single `-O` level's take on the UB in `lib.c`) |
| `tests/c_harness/harness.c` | C | `#include`s the **unmodified** `c_src/src/lib.c` and re-exports its `static` helpers as `harness_*` so the error surface can be driven across FFI. Nothing in `c_src/` is modified. |

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** C symbols missing from the Rust
      `.so` in every feature combination and both profiles; `ldd -r` reports **0**
      unresolved symbols (all 49 imports are glibc/libgcc). With the default
      feature set the two symbol sets are *identical* (`{memchra2}`).
      No stubs, no `unimplemented!()`: all 9 C functions are really translated.
- [x] **Phase B** — every one of the 32 `CONFIGS.md` rows passes across its
      randomized inputs (fixed seed). Additional volume: ≈ 1.9 M randomized calls
      in the default fuzz suite and ≈ 84 M in the exhaustive sweeps
      (all 2^24 low-byte triples of `b,c,d`; 3 × 2^24 contiguous values of `a`;
      4 × 4.2 M strided over the whole `int` range). **0 divergences.**
- [x] **Phase C** — every one of the 34 `ERRORS.md` rows has a passing
      differential test asserting the *same sentinel* (`-1` vs `0` vs
      "branch skipped"), including all NULL-pointer, zero-length,
      oversized-length, one-past-the-range and out-of-`char`-range-needle cases.
- [x] **Every feature combination** — `run_verification.sh` runs Phases B–D for
      `--no-default-features`, `--no-default-features --features test_internals`
      and `--all-features`, in both `dev` and `release`: 6 configurations,
      all PASS (57 tests with default features, 80 with `test_internals`).

## Outcome

No behavioural divergence was found between the C and Rust implementations.
The translation reproduces, verbatim:

* the `int`↔`float` union type pun (`f32::from_bits`), including the treatment of
  subnormals, ±0, ±inf and NaN by `f > 0.0f && f < 1000.0f`;
* the little-endian `(int *)` reinterpretation of `unsigned char bytes[4]`;
* the two's-complement wraparound of `sum` and `result` on signed overflow;
* the *asymmetric* sentinels (`count_occurrences` returns `0` for an empty
  string while `process_buffer` / `complex_iteration` return `-1`);
* `(char)c` truncation of the `memchra` needle;
* `snprintf`'s "at most `size - 1` bytes + always NUL-terminate" contract,
  including `size == 0`, and its would-be-length return value;
* the sign-extension of `char` in `result += (int)(*i)`;
* the exact order of the guards and of the accumulation into `result`.

The only change made to the crate during verification was additive and
test-only: the `test_internals` feature and its `harness_*` exports (off by
default). `src/lib.rs`'s translation of `lib.c` was not modified, because no
divergence required it.
