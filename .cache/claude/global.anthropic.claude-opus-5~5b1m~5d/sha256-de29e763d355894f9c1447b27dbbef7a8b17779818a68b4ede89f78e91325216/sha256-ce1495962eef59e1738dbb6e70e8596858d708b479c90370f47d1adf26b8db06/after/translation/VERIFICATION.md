# Verification summary

Reference C build: `cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
Reproduce everything: `cd translation && ./run_all.sh`

All differential tests load **both** `.so` files with `libloading` and call only
exported symbols — the Rust `#[no_mangle] extern "C"` wrappers are exercised as
an external consumer would, never via direct Rust calls.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` diff between the C and Rust `.so` is **empty**
      (38/38 `T` symbols). No stubs. The Rust `.so`'s undefined symbols are
      libc / unwinder only.
- [x] **Phase B** — all **83** rows of `CONFIGS.md` pass, each across many
      seeded-random inputs (~1.5 M differential calls total), compared with
      `f32::to_bits()` so `±0` and NaN payloads must match too.
- [x] **Phase C** — all **47 `E` rows + 2 `B` rows** of `ERRORS.md` have a
      passing error-path differential test asserting the *same* sentinel value
      (`0`, `(0,0)`, `0.0f`, NaN, `+inf`), not merely "both failed".
      4 `B` rows are marked `n/a` with justification: they are **undefined
      behaviour in C** (out-of-bounds reads/writes, NULL dereference), so there
      is no defined C result to be differential against.
- [x] **Feature combinations** — `translation/Cargo.toml` declares no
      `[features]` table, so `<default>` and `--no-default-features` are the
      only configurations; `run_all.sh` runs the whole suite for both, in the
      **dev and release profile** (4 runs), and re-checks symbol parity each
      time. Result: `ALL GREEN`.

## Test inventory (58 tests)

| file | tests | covers |
|------|-------|--------|
| `tests/common/mod.rs` | — | harness: `.so` loading, C-layout POD types, bitwise comparators, splitmix64 RNG with float-zoo generators |
| `tests/smoke.rs` | 1 | both libraries load; constants agree |
| `tests/phase_b_math.rs` | 20 | CONFIGS C1..C47 (vector maths, proxies, `c2Support`, `c22`/`c23`/`c2D`/`c2L`/`c2Witness`) |
| `tests/phase_b_gjk.rs` | 15 | CONFIGS C48..C83 (`c2GJK` with every transform/`use_radius`/cache/out-param combination, the boolean routines, the `c2Collided` 3×3 matrix, `capsule`) |
| `tests/phase_c_errors.rs` | 21 | ERRORS E1..E47, B1, B4 |
| `tests/e31_search.rs` | 1 | ERRORS E31: 400 k randomized `c2GJK` configurations verifying the iteration bound and counter |

## Deviations found and fixed

None in the Rust source: the translation was already bit-exact against the
reference C build. Three *test* assertions were wrong and were corrected
(documented in `ERRORS.md`):

1. `c2BBVerts` "input unmodified" used `PartialEq`, which is false for NaN —
   switched to bitwise comparison.
2. "Overlapping AABBs always report `dist == 0`" is **not** true of the C: the
   GJK loop sometimes exits early on a degenerate search direction and reports
   a tiny non-zero distance (e.g. `3.8e-6`). The C is the ground truth; the
   assertion was replaced by a branch counter.
3. ERRORS row E46 claimed the wrappers can observe a NaN distance. They cannot:
   with `use_radius = 1` the `dist > rA+rB` test is false for NaN, so the
   midpoint branch clamps it to `0`. The test now asserts that invariant, and
   verifies the C truthiness rule (`if (dist) return 0; else return 1;`) with
   both outcomes actually produced.

## Known non-issue: C compiler optimisation level and NaN sign bits

Against the specified (`-O0`) C build every value matches bit-for-bit.
Rebuilding the C with `-DCMAKE_BUILD_TYPE=Release` (`-O3`) and re-running via
the `C2_C_SO` environment variable leaves everything green except one
quiet-NaN **sign bit** in `c72_gjk_specials` (`0x7fc00000` vs `0xffc00000`):
GCC swaps the operands of a commutative `mulss`/`addss` at `-O3`, changing which
operand's NaN is propagated. That is C instruction selection, not a semantic
difference; all non-NaN values still match exactly at `-O3`.
