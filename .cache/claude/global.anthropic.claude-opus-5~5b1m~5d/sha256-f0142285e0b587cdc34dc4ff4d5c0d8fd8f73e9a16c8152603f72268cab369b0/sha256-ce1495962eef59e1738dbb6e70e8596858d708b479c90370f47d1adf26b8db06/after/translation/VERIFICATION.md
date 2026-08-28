# Verification report — `ldexp_q2` C-to-Rust translation

## Subject

The library is a single 12-line C function:

```c
float ldexp_q2(float y, int exp_q2);          /* c_src/include/lib.h */
```

`c_src/CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`), so
the whole library is this one function. Nothing was left untranslated.

## Result

**The Rust translation matches the C ground truth bit-for-bit on every input
tested (~58.5 million differential call pairs, zero divergences).** No changes
to the translation's logic were required; `src/lib.rs` is unchanged from the
state it was handed over in. All fixes in this pass were to the *test harness*
and to my own initially-wrong expectations, never to the Rust logic.

## How it is tested

Every assertion loads **both** shared objects with `libloading` and calls
`ldexp_q2` through its exported C symbol. The Rust implementation is never
called directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper and the
x86-64 C ABI (`y` in `xmm0`, `exp_q2` in `edi`) are part of what is verified.
Results are compared with `f32::to_bits()`, so `+0.0` vs `-0.0` and NaN
sign/payload differences count as failures rather than being smoothed away by
`==` (under which `NaN != NaN` would silently pass).

| suite | file | tests | what it covers |
|---|---|---|---|
| Phase B | `tests/phase_b_configs.rs` | 29 | one test per `CONFIGS.md` row (28 rows + a harness self-check) |
| Phase C | `tests/phase_c_errors.rs`  | 18 | one test per `ERRORS.md` row |
| Phase D | `tests/phase_d_symbols.rs` | 5  | `nm -D` symbol parity, `dlsym` resolution, no leaked internals |
| Phase E | `tests/phase_e_soak.rs`    | 4  | heavy sweeps (`#[ignore]`d; ~58M pairs) |

Scripts: `scripts/symdiff.sh` (symbol diff), `scripts/verify_all.sh` (all
profile x feature combos), `scripts/mutation_check.sh` (harness validation).

## The C behaviour that had to be replicated exactly

| C construct | why it is delicate | how the Rust matches it |
|---|---|---|
| `e = 120 > exp_q2 ? exp_q2 : 120` | a **clamp**, not a validation — `e` may be negative | same ternary; `cmovg` in the C disassembly |
| `g_expfrac[e & 3]` | for negative `e`, indexes via two's-complement low bits (`-1 & 3 == 3`), which happens to stay in bounds | `G_EXPFRAC[(e & 3) as usize]` |
| `1 << 30 >> (e >> 2)` | for negative `e` this is a **negative shift count = undefined behaviour in C** | gcc emits `sar %cl`, and x86 masks the count to 5 bits, so the observable shift is by `(e>>2) & 31`; Rust masks explicitly, which both reproduces the C *and* keeps the Rust shift well-defined |
| `e >> 2` | implementation-defined for negative `e` | gcc uses arithmetic shift (`sar $0x2`); Rust `>>` on `i32` is arithmetic |
| `y *= frac * scale` | FP association matters | `frac * (scale as f32)` first, then into `y` — matches the two `mulss` in the disassembly |
| `while ((exp_q2 -= e) > 0)` | `do/while`: the body always runs at least once | `loop { ... if exp_q2 <= 0 { return y } }` |

Consequences worth recording, all confirmed against the compiled C:

- The **total multiplier** is `frac[e&3] * 2^(30-k)` where `k = (e>>2) & 31`.
  Since `frac[0]` is exactly `2^-30`, the function is the **identity** exactly
  when `k == 0` — i.e. at `exp_q2 == 0` and on the negative lattice
  `e % 128 == 0` (which includes `INT_MIN`). A *scale* of 1 (`k == 30`) is
  **not** the identity; it still multiplies by `~2^-30`.
- For negative `exp_q2` the behaviour is **periodic with period 128**, so
  sweeping any contiguous 128 negative values covers every distinct negative
  code path. `exp_q2 in {-1,-2,-3,-4}` gives `k == 31`, so the scale is `0` and
  the result is a signed zero (or NaN for `+/-inf`, via `inf * 0`).
- `exp_q2 -= e` can never overflow: either `e == exp_q2` (giving 0) or
  `e == 120` with `exp_q2 > 120`. `INT_MIN - INT_MIN == 0` exactly.
- `exp_q2 == INT_MAX` drives the loop **17,895,698** times and still terminates.

## There is no error surface

`ldexp_q2` is a **total function**. Grepping the C source finds zero
`return -1`, zero `NULL`, zero error enums, zero `assert`, zero range checks,
and exactly one `return` (the success path). It takes no pointer, no length,
and no enum, so the generic null-pointer / oversized-length / out-of-range-enum
boundary classes are structurally inapplicable — `ERRORS.md` documents this with
the grep evidence rather than skipping it, and `e_na_no_pointer_length_or_enum_parameters`
asserts it against the actual header so the claim cannot silently rot. Every
`int32` value is valid input, so the closest analogue (values far past the
internal `120` clamp, and both domain extremes) is fuzzed instead.

## Two real problems found and fixed

Both were in the verification apparatus, not the translation. Recording them
because a green test run that proves nothing is worse than a red one.

### 1. The tests were silently loading a stale `.so` (false passes)

`crate-type` was `["cdylib"]` only. Integration tests therefore had no
Rust-level dependency on the lib target, and **`cargo test` never rebuilt the
`.so`** — the harness loaded whatever a previous `cargo build` had left behind.
Caught by mutation testing: **all 10 injected bugs escaped detection.** The
`.so` was timestamped 23:03 against a source modified at 23:19.

Fixed by (a) adding `rlib` to `crate-type` so `cargo test` rebuilds the cdylib
(this does not alter the cdylib's exported ABI — still exactly `ldexp_q2`), and
(b) an `assert_not_stale()` guard that fails loudly if the loaded `.so` predates
any source file. The guard was itself verified to fire, by pointing
`RUST_SO_PATH` at a back-dated copy.

### 2. Two of my own documented expectations were wrong

Not Rust bugs — my `ERRORS.md` predictions disagreed with the compiled C, and
the C is ground truth:

- I claimed `exp_q2 == 119` yields `0x30d744fd`; the C yields `0x309837f0`.
- I claimed a *scale* of 1 (`exp_q2 == -8`) leaves a subnormal untouched. It
  does not: the total multiplier is still `~2^-30`, so subnormals flush to
  signed zero. Only `k == 0` is the identity.

Both were corrected in `ERRORS.md` to match observed C behaviour.

## Harness validation (mutation testing)

Because a passing differential suite is only meaningful if it can fail,
`scripts/mutation_check.sh` injects deliberate bugs and confirms each is caught:

```
CAUGHT=12  EQUIVALENT=4  ESCAPED=0
```

Caught: wrong residue operator (`e % 4`), regrouped multiplication, flipped
clamp comparison, 1-ULP perturbation of a `g_expfrac` constant, clamp constant
`120 -> 124`, `g_expfrac` residue mix-up, `e >> 3` instead of `e >> 2`,
`1 << 29` instead of `1 << 30`, and two mutations that cause infinite loops
(detected via per-mutation timeout, since the C always terminates).

Four mutations are **provably semantics-preserving**, so the suite is correct
not to flag them — documented so they are not mistaken for blind spots:

- `e & 3` == `e.rem_euclid(4)` for all `i32` (power-of-two modulus).
- `(e>>2) & 31` == `((e as u32)>>2) & 31` (arithmetic vs logical shift differ
  only in bit positions 30-31, which the 5-bit mask discards).
- Removing `& 31`, and widening it to `& 63`, are both no-ops **in release**
  (LLVM lowers `>>` to `sar`, which the CPU masks anyway) but **panic in
  debug**, where Rust's shift-overflow check is enabled. So the explicit mask is
  load-bearing for debug builds, and the debug run catches both — which is why
  every configuration is tested in both profiles.

## Completion gate

- [x] `cargo check` clean; `cargo clippy --all-targets` reports **0 warnings**.
      The one lib-level lint (`excessive_precision` on the `g_expfrac` literals)
      is explicitly `#[allow]`ed: the literals are verbatim from the C and
      re-rounding them would risk a 1-ULP divergence.
- [x] `SYMBOLS.md`: `nm -D` shows 0 missing and 0 undefined non-libc symbols.
      C exports 1 symbol (`ldexp_q2`); Rust exports the same 1.
- [x] Phase B: all **28** `CONFIGS.md` rows pass across randomized inputs
      (~110k cases in the row tests; no row is vacuous — each prints its count).
- [x] Phase C: all **18** `ERRORS.md` rows have a passing differential test.
- [x] Phase D: verified under **every** feature combination x **both** profiles
      (6 configurations: `default`, `--no-default-features`, `--all-features`
      x `debug`, `release`). The crate declares no `[features]` and the source
      has no `cfg`, so this is the complete power set.
- [x] Harness proven capable of detecting bugs (12/12 non-equivalent mutations).
- [x] Soak: ~58.5M additional differential pairs, 0 divergences, both profiles.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

./scripts/verify_all.sh        # all 6 configurations + symbol parity
./scripts/mutation_check.sh    # prove the suite can fail
cd translation && cargo test --release --test phase_e_soak -- --ignored
```

Note: `translation/.cargo/config.toml` sets `offline = true` because this
environment has no crates.io access; `libloading 0.8.9` resolves from the local
cargo cache.
