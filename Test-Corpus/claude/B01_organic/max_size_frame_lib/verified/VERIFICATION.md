# VERIFICATION.md — final completion gate

Differential verification of the C-to-Rust translation of `max_size_frame`.
The C in `c_src/` is the ground truth and was never modified.

## How to reproduce

```sh
cd translated_rust

# Phase A.2 — cargo check for every feature combination
./check_all_features.sh check

# Phases B, C, D — full differential suite, every feature combo, dev + release
./run_all_tests.sh

# Optional (~4 min): exhaustive 2^32 per-axis sweeps
RUN_EXHAUSTIVE=1 ./run_all_tests.sh
```

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
      The C `.so` exports exactly one API symbol, `max_size_frame`; the Rust
      `.so` exports it under the identical name. The C→Rust symbol diff is
      **empty**. No C source was left untranslated (the entire C library is 15
      lines across two files, and `CMakeLists.txt` compiles only `src/lib.c`).
      Enforced on every run by `tests/symbol_parity.rs` (4 tests), not just
      asserted in prose.
- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      26/26 rows, one `#[test]` each in `tests/differential.rs` (27 tests
      including a harness sanity check). Rows 18, 22, 23, 24 and 26 are
      exhaustive over their stated domains; the rest use a fixed-seed splitmix64
      PRNG (20 000 inputs per row, 200 000 for the unconstrained fuzz row).
- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential test.**
      21/21 rows, one `#[test]` each in `tests/error_paths.rs`.
- [x] **All of the above hold under EVERY feature combination.**
      `Cargo.toml` declares no `[features]` table, so the only valid combination
      is the empty set. `run_all_tests.sh` nevertheless machine-checks
      `--no-default-features`, default, and `--all-features`, each in **dev and
      release** — 6 full suite runs, all passing.

## Results

| suite | tests | result |
|---|---|---|
| `tests/differential.rs` (Phase B, `CONFIGS.md` rows 1–26) | 27 | pass |
| `tests/error_paths.rs` (Phase C, `ERRORS.md` rows 1–21) | 21 | pass |
| `tests/symbol_parity.rs` (Phase D) | 4 | pass |
| `tests/exhaustive_axis.rs` (opt-in 2^32 sweeps) | 4 | pass (224 s) |
| **total** | **56** | **pass** |

Configurations: `--no-default-features`, default, `--all-features` × {dev,
release} = 6 runs, all green, plus `cargo check` green for all 3 feature
invocations.

## Derived C semantics (the specification being matched)

`uint32_t` **is** `unsigned int` on this platform (verified with `_Generic`;
`sizeof(int) == sizeof(unsigned) == 4`). Therefore *every* subexpression in
`c_src/src/lib.c` has at least one `unsigned int` operand, so the `int` results
of `!=`/`==` and the `int` literals are converted to `unsigned int` by the usual
arithmetic conversions. Consequences:

* All overflow is **well-defined modular wraparound** — there is no signed
  integer overflow and therefore **no undefined behaviour**. Confirmed with
  `clang -fsanitize=undefined` over the corner grid plus 5M random triples: zero
  reports; `-fsanitize=integer` reports only `unsigned int` overflow.
* The `/ 8` is an **unsigned** division (the C `.so` compiles it to `shr $0x3`).
* The odd `+ +7` is a no-op unary plus on the literal `7`.

Closed form, all arithmetic mod 2^32:

```text
M = bitdepth*channels*[channels!=2] + bitdepth*[channels==2]
                                    + (bitdepth + [bitdepth!=32])*[channels==2]
f(bs, ch, bd) = 18 + ch + ((bs*M + 7) udiv 8)
```

`src/lib.rs` reproduces this node-for-node with `wrapping_mul`/`wrapping_add`
and an unsigned `u32 / 8`.

## Why these results are trustworthy

Passing tests alone prove little, so each of the following was demonstrated
empirically rather than assumed:

1. **The Rust code is reached only through the FFI boundary.** Both libraries are
   loaded with `libloading` (`dlopen` + `dlsym` on `max_size_frame`); the Rust
   function is never called directly as a Rust function, so the
   `#[unsafe(no_mangle)] extern "C"` export wrapper is itself under test.
2. **A stale-artifact bug was found and fixed.** `cargo test` does **not**
   rebuild a `cdylib`-only lib target, so the first version of the harness
   silently tested an out-of-date `.so` — an injected `+7 → +8` bug passed all
   52 tests. The harness now builds the cdylib itself into `target/diff-so` and
   asserts the artifact is newer than every `.rs` source
   (`assert_fresher_than_sources`).
3. **The C ground truth is likewise guarded.** The C `.so` is rebuilt when older
   than any `c_src` source, and its filename is **pinned** to CMake's naming rule
   (`lib<crate-dir>.so`) so a stray `.so` in `c_src/build` can never be adopted
   as the reference. Both guards were verified on throwaway copies: mutating
   `c_src/src/lib.c` without rebuilding now fails loudly, and a
   `libaaa_decoy.so` planted in `c_src/build` is correctly ignored.
4. **Mutation testing: 17/17 mutants killed.** Deliberate bugs injected into
   `src/lib.rs` — `!=2 → !=3`/`>2`, `==2 → ==3` (both terms), `!=32 → !=31`/`<32`/`==32`,
   `18 → 17`, `+7 → +6`/`+8`, `/8 → /4`, **signed division**, `wrapping → saturating`
   at the T1 multiply / the `+7` / the outer add, dropping the `channels` factor
   from T1, and dropping `+[bitdepth!=32]` from T3 — were each caught (3–28
   failing tests apiece).
5. **The oracle is genuinely independent.** `common::oracle` is written by a
   different route from `src/lib.rs` (`u64` arithmetic masked to 32 bits after
   every operation, using the collapsed multiplier `M`), so a shared misreading
   of the C cannot hide behind a bare `C == Rust` comparison. Every assertion
   checks C == Rust == oracle.
6. **Axis X8 is covered constructively, not by luck.** Reaching
   `numerator + 7 > UINT32_MAX` by random sampling has probability ~1.6e-9, so
   the original 400 000-iteration search found **zero** cases and asserted
   nothing. It now solves for the input via the multiplicative inverse of the odd
   multiplier `M` mod 2^32, verifying **140 000** genuine wrap cases.
7. **Exhaustive per-axis coverage.** `tests/exhaustive_axis.rs` enumerates all
   2^32 values of each argument in turn across 20 pinned parameter pairs
   (~9.9e10 differential comparisons, 224 s in release). This exhaustively covers
   `(P + 7) / 8` for every 32-bit product `P`, `18 + channels` for every
   `channels`, and the stereo multiplier `M` for every `bitdepth`. A negative
   control confirmed these sweeps also catch an injected bug.

## Residual risk (stated honestly)

The input domain is 2^96, so no test suite can be exhaustive over it. The one
region not covered exhaustively by any test is the truncating 32×32→32 multiply
with **both** operands simultaneously large (`channels·bitdepth` and
`blocksize·M`, ~2^64 pairs each). A deliberately hidden "needle" mutant such as

```rust
if bs == 0xDEAD_BEEF && ch == 0x1234_5678 && bd == 0x0BAD_F00D { return 0; }
```

would survive this suite, as it would survive any finite random/exhaustive-slice
strategy. Closing that gap requires SMT/bitvector equivalence checking (no
`z3`/`cvc5` is installed and there is no network access in this environment).
Standing in for it: the two implementations are structurally identical over
`Z/2^32` (both compile to the same `imul`/`add`/`shr $0x3` sequence), the
translation is a node-for-node transcription of an 8-line expression, and
~1.3e11 input triples have been compared with zero divergences.

## Files produced

| file | purpose |
|---|---|
| `SYMBOLS.md` | Phase A.1 — `nm -D` symbol parity and C completeness audit |
| `ERRORS.md` | Phase A.2 — error-surface table (21 rows) |
| `CONFIGS.md` | Phase A.3 — configuration-surface table (26 rows) |
| `VERIFICATION.md` | this final gate |
| `tests/common/mod.rs` | `libloading` harness, freshness/pinning guards, PRNG, independent oracle |
| `tests/differential.rs` | Phase B — one test per `CONFIGS.md` row |
| `tests/error_paths.rs` | Phase C — one test per `ERRORS.md` row |
| `tests/symbol_parity.rs` | Phase D — automated `nm -D` diff |
| `tests/exhaustive_axis.rs` | opt-in exhaustive 2^32 per-axis sweeps |
| `check_all_features.sh` | enumerates feature combinations, runs `cargo check` on each |
| `run_all_tests.sh` | runs the suite for every combination × {dev, release} + symbol diff |

Only `Cargo.toml` (added `libloading` to `[dev-dependencies]`), `Cargo.lock`, the
new test/doc/script files above, and the git-ignored build outputs were touched.
`src/lib.rs` needed **no** correctness changes, and nothing in `c_src/` was
modified.
