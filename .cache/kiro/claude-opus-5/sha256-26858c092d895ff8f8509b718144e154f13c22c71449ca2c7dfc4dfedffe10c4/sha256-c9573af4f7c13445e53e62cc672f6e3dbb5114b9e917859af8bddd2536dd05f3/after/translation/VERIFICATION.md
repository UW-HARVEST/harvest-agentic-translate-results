# VERIFICATION.md — results

C reference: `c_src/src/lib.c` (single translation unit) built by
`c_src/CMakeLists.txt` with **no** `CMAKE_BUILD_TYPE` and **no** `CMAKE_C_FLAGS`,
i.e. unoptimised SSE scalar code.
Rust: `translation/src/lib.rs` → `libcollided_lib.so` (`crate-type = ["cdylib"]`).

Every test loads **both** `.so`s with `libloading` and calls only `dlsym`ed
exports — the Rust functions are never called directly, so the
`#[no_mangle]`/`extern "C"` wrappers and the SysV struct-passing ABI (8-byte
`c2v` in one XMM, 12-byte `c2Circle` and 16-byte `c2AABB` across two) are under
test as well. All comparisons are on **raw bit patterns**, so `+0.0` vs `-0.0`
and differing NaN payloads count as divergences.

Nothing in `c_src/` was modified (source mtimes unchanged; only `c_src/build/`
was added).

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` lists 10 defined symbols; the Rust
      `.so` exports all 10 with identical names. **Symbol diff is empty.** No
      C module was left untranslated, so nothing had to be added or stubbed.
      `nm -D -u` on the Rust `.so` shows only glibc / libgcc-unwind imports —
      **0 missing non-libc symbols**.
- [x] **Phase B** — all **30** `CONFIGS.md` rows pass. Randomized rows use 4000
      inputs each from a fixed-seed SplitMix64 (`0x2545F4914F6CDD1D`), plus
      hand-written boundary corpora.
- [x] **Phase C** — all **5** `ERRORS.md` rows have a passing differential test
      that asserts the *exact* sentinel (`0`) rather than "both failed", plus 4
      generic-boundary tests including an exhaustive tag sweep over
      `-64..=64 ∪ {i32::MIN, i32::MAX, ±0x1000}` (≈35 000 rejections).
- [x] **Every configuration** — `Cargo.toml` declares no `[features]`, so the
      only configurations are `default`, `--no-default-features` and
      `--all-features`. `scripts/verify_all.sh` runs symbol parity + the full
      suite for all three **× both profiles** (debug and release): 6 runs,
      all passing.

## Test inventory

| file | tests | what |
|------|-------|------|
| `tests/harness/mod.rs` | — | loads both `.so`s, ABI mirrors, PRNG, special-value corpus |
| `tests/smoke.rs` | 1 | both libraries resolve all 10 symbols |
| `tests/configs.rs` | 30 | Phase B, one test per `CONFIGS.md` row |
| `tests/errors.rs` | 9 | Phase C, 5 row tests + 4 generic-boundary tests |
| `tests/heavy.rs` | 9 | `#[ignore]`d structured grids and exhaustive 2^32 sweeps |

Run: `cargo test`, `./scripts/verify_all.sh`, `./scripts/verify_all.sh --heavy`.

## Exhaustive results

Beyond sampling, these ran to completion against the C `.so` with **zero**
divergences:

| sweep | domain | result |
|-------|--------|--------|
| `heavy_c2dot_exhaustive_single_lane` | all 2^32 `f32` values of `x` in `c2Dot((x,0),(x,0))` | 0 diverged (164 s) |
| `heavy_c2sub_exhaustive_single_lane` | all 2^32 `f32` dst values × rotating special src | 0 diverged (164 s) |
| `heavy_c2maxv_exhaustive_single_lane` | all 2^32 `f32` left operands | 0 diverged (161 s) |
| `heavy_c2minv_exhaustive_single_lane` | all 2^32 `f32` left operands | 0 diverged (161 s) |
| `heavy_c2clampv_exhaustive_single_lane` | all 2^32 `f32` values of `a` | 0 diverged (279 s) |
| `heavy_c2dot_structured_grid` | 168-value (sign × 12 exponents × 7 mantissas) grid², 5 y-phases | 0 diverged |
| `heavy_predicates_random_bulk` | 400 000 iterations × 3 predicates × 4 `collided` tag pairs | 0 diverged |

The comparison sweep was split into three tests because running them together
took 9 m 40 s, over the 600 s per-command budget; each part now takes ≈3 min.

## Mutation sanity check

`scripts/mutation_check.sh` proves the suite is not vacuous: it injects one
behavioural change at a time into `translation/src/lib.rs`, rebuilds, and re-runs
the suite. **15/15 mutants detected.**

| mutant | detected by |
|--------|-------------|
| `c2Dot` `addss` operand order flipped | `cfg_row12`, `cfg_row24` |
| `c2Dot` second `mulss` operand order flipped | `cfg_row12` |
| `c2AABBtoAABB` `<` → `<=` | `cfg_row23` |
| `c2AABBtoAABB` drops `d3` | 6 rows |
| circle predicates `<` → `<=` | 10 rows |
| `c2Maxv` uses `f32::max` | 10 rows |
| `c2Minv` uses `f32::min` | 11 rows |
| `c2Clampv` min/max swapped | 10 rows |
| SNaN quieting removed | `cfg_row09`–`12`, `cfg_row24` |
| wrong QNaN-indefinite constant | `cfg_row10`–`12` |
| `c2Sub` operand order flipped | `cfg_row09`, `cfg_row10`, `cfg_row24` |
| `collided` AABB×CIRCLE swap "corrected" | `cfg_row27`, `cfg_row30` |
| outer `default:` dispatches instead of rejecting | `errors.rs` (SIGABRT on null deref) |
| inner `default:` dispatches instead of rejecting | `errors.rs` (SIGABRT) |
| tag normalised with `& 1` | `errors.rs` (SIGABRT) |

### One provably-equivalent mutant

Changing `addss(B.r, A.r)` to `addss(A.r, B.r)` in `c2CircletoCircle` survives,
and this is **not** a test gap. Float addition is commutative for every non-NaN
input (including `inf + -inf` → the same QNaN-indefinite either way, and
`0 + -0` → `+0` either way), so the two orders can only differ in the NaN
*payload*. That payload is consumed solely by `d2 < r2`, which is false for any
NaN regardless of payload, and `r2` is never returned. The difference is
therefore unobservable through the public ABI. The order in `src/lib.rs` still
matches the C's emitted `addss %xmm1,%xmm0` (dst = `B.r`).

## What the C actually does that the Rust reproduces

Confirmed by disassembling `c_src/build/CMakeFiles/*/src/lib.c.o` rather than
inferred:

- `c2Dot` pins `mulss dst=a.x,src=b.x`, `mulss dst=b.y,src=a.y`,
  `addss dst=(b.y*a.y),src=(a.x*b.x)`. SSE two-operand scalar arithmetic resolves
  NaNs by preferring **dst**, so this order decides which payload survives.
- `c2CircletoCircle` emits `addss %xmm1,%xmm0` with dst = `B.r`, src = `A.r`.
- `c2Sub` emits `subss dst=a.x,src=b.x`.
- `c2Maxv`/`c2Minv` use `comiss`+`jbe`, i.e. genuine `a>b ? a : b` — on an
  unordered compare the branch is taken and the **second** operand wins. This
  differs from `f32::max`/`f32::min`, which is why the Rust uses explicit
  comparisons.
- `C2_TYPE` is passed as a 4-byte `int` (`cmpl $0x0,-0xc(%rbp)`), and each
  `switch` arm dereferences its operands **inside the taken case only** — which is
  what makes a null pointer well-defined on the rejecting arms.
- `collided`'s `AABB × CIRCLE` arm calls
  `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` — the operands are swapped
  relative to the parameter names. Reproduced verbatim, and `cfg_row27` asserts
  the swap is still there.

Behaviours that look like bugs but are the C's ground truth, and are pinned by
tests rather than "fixed":

- Touching shapes (`d2 == r2`, `A.max.x == B.min.x`) count as **colliding** for
  AABB/AABB and **not colliding** for the circle predicates, because both use a
  strict `<` on different sides.
- A NaN in any AABB component makes every `<` false, so `c2AABBtoAABB` reports a
  **collision**.
- Negative radii still produce a positive `r2`, so `c2CircletoCircle` can report a
  collision for them.
- Inverted boxes (`min > max`) are never normalised.
- A zero-radius circle at a box centre does **not** collide (`0 < 0` is false).

## Known non-goals

`collided` with a **valid** tag and a null/unmapped pointer dereferences
unconditionally in the C — undefined behaviour, not a defined rejection. There is
no C result to match, so it is deliberately not asserted; `ERRORS.md` records the
omission and `err_generic_null_is_only_safe_on_the_rejecting_arm` covers the part
that *is* defined.
