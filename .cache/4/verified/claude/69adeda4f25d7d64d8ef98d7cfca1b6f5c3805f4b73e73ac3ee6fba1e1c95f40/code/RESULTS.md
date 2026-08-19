# RESULTS.md — Verification summary

**Verdict: the Rust translation matches the C byte-for-byte on every input tested.
No divergence was found, so no change to `src/lib.rs` was required.**

Library under test: `my_pow(double base, double exponent)` — the entire public
surface of `c_src` (one header, one `.c` file, one exported symbol).

## How it was tested

Both implementations are loaded as shared objects with `libloading` and called
**only** through their exported `my_pow` symbol, so the Rust
`#[unsafe(no_mangle)] extern "C"` wrapper is exercised exactly as an external C
caller would exercise it. The Rust crate is never called directly.

`my_pow`'s behaviour is not fully captured by its return value, so every
comparison asserts the whole observable **triple**:

1. **return bit pattern** (`f64::to_bits`) — necessary because `-1.0` is both the
   error sentinel *and* a legal result (`pow(-1, 3)`), and because `-0.0`/`+0.0`
   and NaN payloads must not be conflated;
2. **exact stderr bytes** — the `fprintf("%.2f")` messages, captured by
   redirecting fd 2 to a file around each call;
3. **residual `errno`** — read immediately after the call, before any other libc
   call can clobber it.

## Reproducing

```sh
# C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# every configuration: cargo check + build + nm parity + cargo test
./verify_all.sh
```

`verify_all.sh` exits 0. Last run: **ALL CONFIGURATIONS PASSED**.

## Phase A — surface map

| artifact | content |
|---|---|
| `SYMBOLS.md` | 1 exported symbol (`my_pow`) in the C `.so`; the Rust `.so` exports it too. Symbol diff is **empty**. Both `.so`s also import the *same* 4 libc symbols with identical glibc version tags, including `pow@GLIBC_2.29`. |
| `ERRORS.md` | 22 rows — every distinct way the C rejects input, derived from its 2 `return -1` branches (`errno == EDOM`, `errno == ERANGE`) plus every trigger that reaches them, the `errno = 0` discard semantics, and the negative controls. |
| `CONFIGS.md` | 46 rows — the library has **no** runtime options or compile-time flags, so the configuration surface is entirely input **shape**; rows enumerate the pruned cross-product of the 8 axes the code branches on. |

No C source was missing: `CMakeLists.txt` builds exactly one file (`src/pow.c`),
which contains exactly one function and no `static` helpers. Nothing had to be
translated or stubbed.

## Phase B / C / D — test results

| suite | tests | comparisons | result |
|---|---|---|---|
| `tests/smoke.rs` | 7 | (self-checks) | pass |
| `tests/configs.rs` (Phase B, rows C1–C45) | 45 | 25,652 | pass |
| `tests/errors.rs` (Phase C, rows E1–E22 + class cross-product) | 23 | 3,656 | pass |
| `tests/threads.rs` (row C46, concurrency) | 1 | 32,000 | pass |
| **total per configuration** | **76** | **61,308** | **pass** |

Configurations verified (Phase D): `Cargo.toml` declares **no** `[features]`, so
the feature powerset is the single empty set; `verify_all.sh` additionally treats
the declared default set and both cargo profiles as distinct configurations
(`release` sets `panic = "abort"`, and its optimiser could legally have folded the
`pow` call away):

| profile | features | cargo check | symbol parity | `pow` version tag | tests |
|---|---|---|---|---|---|
| dev | `--no-default-features` | clean | 0 missing | `pow@GLIBC_2.29` | 76/76 |
| dev | default | clean | 0 missing | `pow@GLIBC_2.29` | 76/76 |
| release | `--no-default-features` | clean | 0 missing | `pow@GLIBC_2.29` | 76/76 |
| release | default | clean | 0 missing | `pow@GLIBC_2.29` | 76/76 |

→ **245,232 differential comparisons across all four configurations, zero
divergences.** The suite was also run 5× consecutively to confirm it is not
flaky.

## Why the translation is correct (the two things that could have gone wrong)

1. **It calls libm `pow`, not `f64::powf`.** `f64::powf` never sets `errno`, so a
   translation using it would silently make *both* error branches dead code:
   every domain and range error would return the mathematical result instead of
   `-1.0`, and nothing would ever be printed. `nm -D` confirms both `.so`s import
   the *same versioned* symbol `pow@GLIBC_2.29` — this matters because glibc also
   ships a `pow@GLIBC_2.2.5` compat wrapper with different `errno` semantics.
2. **It calls libc `fprintf` with the same format strings**, rather than
   reimplementing `%.2f` in Rust. Rust's `{:.2}` differs from C's `%.2f` on
   several of the values that actually occur here — most visibly `DBL_MAX`, which
   `%.2f` expands to a 309-digit decimal, and `-0.0`, which must render as
   `-0.00`. Both are verified byte-for-byte (rows E9, E5, C40).

It also correctly reproduces `errno = 0` *before* calling `pow` (rows E16–E18): a
caller with a pre-existing `errno` of `EDOM` must still get a clean result.

## Notable C behaviours found by testing and preserved

Each of these was discovered by a test failing against my *expected* value, then
confirmed against the C and encoded as the C's behaviour. In every case the Rust
already agreed with the C.

| finding | where |
|---|---|
| A **signaling** NaN is *not* covered by the `pow(x,0)==1` / `pow(1,y)==1` rules — glibc returns the quieted sNaN (`0x7FF0…01` → `0x7FF8…01`), payload preserved. | C6, C7, C8, C27 |
| A negative base with a **subnormal** exponent is `EDOM`, not "exponent ≈ 0, so 1.0": a subnormal is not an integer, so the domain check wins. | C33 |
| `%.2f` **collides** distinct inputs: `pow(5e-324, -1)` and `pow(0.0, -1)` emit textually identical messages, and `DBL_MIN` prints as `0.00`. Preserved, not "fixed". | E10, E12, C40 |
| `pow(largest_subnormal, -1)` is a finite `4.49e307` — **not** an overflow; overflow needs `\|exp\| >= 2`. | E10 |
| `-1.0` is a legal result with `errno == 0`, indistinguishable from the error sentinel by return value alone. | E19 |
| The `inf`/`nan` spellings of `%.2f` are **unreachable** in these messages, because glibc's `pow` never sets `errno` for non-finite arguments. Proven, not assumed. | C40 |

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined
      non-libc/non-libgcc symbols in the Rust `.so`.
- [x] Phase B: every row in `CONFIGS.md` (C1–C46) passes across randomized inputs.
- [x] Phase C: every row in `ERRORS.md` (E1–E22) has a passing error-path
      differential test asserting the same error code *and* sentinel *and* message.
- [x] All of the above hold under every configuration (2 feature sets × 2 profiles).
