# CONFIGS.md — Phase A configuration surface table

## Build-time configuration (feature combinations)

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` defines
no `option()`, no `target_compile_definitions`, and the C source contains **zero
`#ifdef`/`#if`** (`grep -c '#if' c_src/src/lib.c` → 0). The complete set of
valid feature combinations is therefore exactly one: the empty set.

| # | feature combo | command |
|---|---------------|---------|
| F1 | *(none — `default` is also empty)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

Both the `dev` and `release` Cargo profiles are additionally verified, because
`[profile.release] panic = "abort"` and the different opt-levels can change
float codegen (NaN-payload operand ordering in particular).

| # | profile | command |
|---|---------|---------|
| P1 | dev     | `cargo test --no-default-features` |
| P2 | release | `cargo test --no-default-features --release` |

## Runtime configuration axes (derived from the C branches)

Mechanically enumerated from `grep -nE "if|else|switch|case|default|\?" c_src/src/lib.c`:

* **A1 — `c2Collided`'s `typeB` mode** (`switch`, 3 valid arms + `default`):
  `C2_TYPE_CIRCLE` / `C2_TYPE_AABB` / `C2_TYPE_CAPSULE`. Note `A` is *always*
  reinterpreted as `c2Circle`; only `B`'s type is selected.
* **A2 — `c2CircletoCapsule`'s arm** (`if (da<0)` / `if (db<0)` / `else`):
  3 mutually exclusive code paths.
* **A3 — `c2Clampv` region** (two independent `?:` pairs inside
  `c2Maxv(lo, c2Minv(a,hi))`): per axis the value is *below `lo`*, *inside*, or
  *above `hi`* → 3 × 3 = 9 regions.
* **A4 — predicate outcome** (`d2 < r2`): hit / miss / exactly-tangent.
* **A5 — struct ABI shape** (SysV argument class actually exercised):
  `c2v` 8 B (SSE, 1 XMM), `c2Circle` 12 B (SSE+SSE, 2 XMM), `c2AABB` 16 B
  (SSE+SSE), `c2Capsule` 20 B (**MEMORY — passed on the stack**).
* **A6 — float value class**: normal, `+0.0`, `-0.0`, subnormal, huge (product
  overflows to `inf`), `±inf`, quiet NaN, signalling NaN, arbitrary random NaN
  payload, and fully random 32-bit patterns.
* **A7 — degeneracy shape**: zero-extent AABB (`min == max`), inverted AABB
  (`min > max`), zero-length capsule (`a == b`), zero radius, negative radius.

## Configuration table

Every row is exercised against **both** `.so`s with many randomized inputs
(fixed seed `0x5EED_C011_1DE_u64`, see `tests/common/mod.rs`), comparing raw
bit patterns (`f32::to_bits`) / `int` values.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| C1  | `c2V` | A5=`c2v` return; A6 = full random 32-bit patterns (pass-through must preserve NaN payloads, `-0.0`, subnormals) | `c1_c2v_passthrough` | [x] |
| C2  | `c2Mulvs` | A6 = normal × normal, and scale = `0.0`, `-0.0`, `1.0`, `inf`, subnormal (underflow), huge (overflow) | `c2_mulvs_value_classes` | [x] |
| C3  | `c2Mulvs` | A6 = NaN in `a`, NaN in `b`, NaN in both (SSE first-operand payload priority) | `c3_mulvs_nan_matrix` | [x] |
| C4  | `c2Maxv` | A6 = all orderings `a<b`, `a>b`, `a==b`, `±0` pairs, NaN in lhs/rhs/both, `±inf` | `c4_maxv_matrix` | [x] |
| C5  | `c2Minv` | same matrix as C4 | `c5_minv_matrix` | [x] |
| C6  | `c2Clampv` | A3 = all 9 below/inside/above regions, driven by randomized `a`/`lo`/`hi` | `c6_clampv_all_regions` | [x] |
| C7  | `c2Clampv` | A7 = inverted bounds (`lo > hi`), zero-width bounds (`lo == hi`), and NaN bounds | `c7_clampv_degenerate_bounds` | [x] |
| C8  | `c2Sub` | A6 = normal, `±0 − ±0` (sign-of-zero rules), `inf − inf`, NaN payload matrix | `c8_sub_value_classes` | [x] |
| C9  | `c2Dot` | A6 = normal magnitudes, mixed huge/tiny (overflow / underflow / cancellation), fully random bit patterns | `c9_dot_value_classes` | [x] |
| C10 | `c2Dot` | A6 = NaN payload matrix over both components (x-term keeps lhs, y-term keeps rhs, sum keeps rhs) | `c10_dot_nan_matrix` | [x] |
| C11 | `c2CircletoCircle` | A5 = 2× 12-byte SSE structs; A4 = hit / miss / tangent, randomized centres+radii in the ±200 range | `c11_circle_circle_random` | [x] |
| C12 | `c2CircletoCircle` | A7 = zero radii, negative radii, `A.r == -B.r`, coincident centres, one circle fully inside the other | `c12_circle_circle_degenerate` | [x] |
| C13 | `c2CircletoCircle` | A6 = `±inf`/NaN/subnormal/huge centres and radii (random bit patterns) | `c13_circle_circle_extremes` | [x] |
| C14 | `c2CircletoAABB` | A3 = all 9 clamp regions × A4 hit/miss, randomized well-formed boxes | `c14_circle_aabb_all_regions` | [x] |
| C15 | `c2CircletoAABB` | A7 = `min == max` (point box), inverted box on x only / y only / both, zero and negative radius | `c15_circle_aabb_degenerate` | [x] |
| C16 | `c2CircletoAABB` | A6 = random bit-pattern boxes and radii (NaN/inf bounds exercise the `?:` clamp) | `c16_circle_aabb_extremes` | [x] |
| C17 | `c2CircletoCapsule` | A2 arm 1 (`da < 0`, before `a`) × A4 hit/miss, randomized segments | `c17_capsule_arm_before_a` | [x] |
| C18 | `c2CircletoCapsule` | A2 arm 2 (`da >= 0 && db < 0`, middle/projection arm, includes the `da / dot(n,n)` divide) × A4 | `c18_capsule_arm_middle` | [x] |
| C19 | `c2CircletoCapsule` | A2 arm 3 (`da >= 0 && db >= 0`, beyond `b`) × A4 | `c19_capsule_arm_beyond_b` | [x] |
| C20 | `c2CircletoCapsule` | A5 = MEMORY-class 20-byte struct on the stack; unconstrained randomized capsules (all arms mixed, axis-aligned / diagonal / zero-length segments) | `c20_capsule_random_mixed` | [x] |
| C21 | `c2CircletoCapsule` | A7 = zero-length capsule (`a == b`, divide by zero), zero/negative radii; A6 = random bit patterns | `c21_capsule_degenerate_and_extremes` | [x] |
| C22 | `c2Collided` | A1=`C2_TYPE_CIRCLE` (0), `B` buffer holds a `c2Circle`; randomized | `c22_collided_circle` | [x] |
| C23 | `c2Collided` | A1=`C2_TYPE_AABB` (1), `B` buffer holds a `c2AABB`; randomized | `c23_collided_aabb` | [x] |
| C24 | `c2Collided` | A1=`C2_TYPE_CAPSULE` (2), `B` buffer holds a `c2Capsule`; randomized | `c24_collided_capsule` | [x] |
| C25 | `c2Collided` | A1=`C2_TYPE_CIRCLE` with `A == B` (same pointer passed twice — aliasing, always a self-collision unless `r == 0`) | `c25_collided_aliased_pointers` | [x] |
| C26 | `c2Collided` | `B` is a raw byte buffer filled with random bytes, reinterpreted per A1 — checks the Rust reads exactly the same bytes/offsets as C for all three tags | `c26_collided_raw_byte_buffers` | [x] |
| C27 | `circle_collide` | fixed built-in geometry; deterministic sweep of a grid over `x,y ∈ [-120,120]`, `r ∈ {0,…,60}` — must reach all 8 result bitmasks | `c27_circle_collide_grid` | [x] |
| C28 | `circle_collide` | randomized `x,y,r` (uniform in ±200) — value-dependent boundary agreement | `c28_circle_collide_random` | [x] |
| C29 | `circle_collide` | randomized fully-random bit patterns for `x,y,r` (NaN/inf/subnormal/huge) | `c29_circle_collide_bitpatterns` | [x] |
| C30 | all 12 exports | called back-to-back in one process from both `.so`s loaded simultaneously (no global state, no cross-talk); confirms every `nm -D` symbol is reachable through `dlsym` | `c30_all_symbols_reachable` | [x] |

## Verification results

Driver: `./run_diff_tests.sh` (builds the C `.so`, enumerates feature combos,
builds the Rust cdylib per profile, then runs every test suite).

```
feature combos: 1 -> [""]
== features=''  profile=dev      -> 31 + 3 + 29 = 63 passed, 0 failed (1 ignored*)
== features=''  profile=release  -> 31 + 3 + 29 = 63 passed, 0 failed (1 ignored*)
ALL CONFIGURATIONS PASSED
```

\* `e31_null_child` is the deliberately-crashing child process driven by
`e31_null_pointer_ub_parity`; it is `#[ignore]`d so only its parent invokes it.

### Divergence found and fixed

One real translation bug was found by row **C3 / E27**:

* `c2Mulvs` — the C compiles `a.x *= b` to `mulss <b>, %xmm(a.x)`, so the vector
  component is the SSE *destination* (first) operand and wins the NaN-payload
  tie-break. LLVM commuted the `fmul` and put `b` in the destination, so with
  NaN in *both* operands the Rust returned `b`'s payload
  (`0xffc00000`) where C returns `a.x`'s (`0x7fc00000`). Fixed in `src/lib.rs`
  by pinning the both-NaN case through the existing `nan_ordered` helper.
  No other file was changed; `c_src/` is untouched (md5s unchanged).

### Test sensitivity (mutation testing)

To prove the suite is not vacuously green, 13 mutations were injected into
`src/lib.rs` and the suite re-run. 12 were caught:

| mutation | tests failed |
|----------|--------------|
| `d2 < r2` → `d2 <= r2` (circle/circle tangency) | 6 |
| `c2Maxv` via `f32::max` (NaN semantics) | 3 |
| `c2Collided` `default` arm returns `1` | 6 |
| `c2Dot` y-term keeps lhs payload | 2 |
| `c2Mulvs` keeps rhs payload (*the bug actually found*) | 2 |
| `circle_collide` AABB shift `<< 1` → `<< 2` | 2 |
| `c2Sub` operands swapped | 2 |
| capsule `da < 0` → `da > 0` | 4 |
| capsule `db < 0` → `db > 0` | 4 |
| capsule `da < 0` → `da.is_sign_negative()` | 3 |
| `c2Clampv` lo/hi order swapped | 4 |
| AABB `r2 = A.r * A.r` → `r2 = A.r` | 5 |
| capsule `da < 0` → `da <= 0` | **0 — equivalent mutant** |

The last one is provably unobservable, not a coverage gap: when `da == 0` and
`n != 0`, the middle arm computes `e = ap - n*(0/|n|²) = ap`, so `d2 = |ap|²`,
exactly what the `da < 0` arm computes; when `n == 0` the beyond-b arm computes
`d2 = |A.p - B.b|² = |ap|²` (since `a == b`), again the same value.

### Staleness guard

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library, so a
stale `.so` could have made every differential test pass vacuously (this was
observed during Phase B — the first `c2Mulvs` fix appeared to have no effect).
`tests/common/mod.rs::assert_fresh` now aborts the run if either `.so` is older
than its newest source file; verified by `touch src/lib.rs` producing
`STALE ARTIFACT: ...`.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` diff is EMPTY (12/12 C symbols exported by the Rust
      `.so`); no unresolved non-libc symbols. Asserted by `tests/symbol_parity.rs`.
- [x] Phase B: all 30 `CONFIGS.md` rows pass across randomized inputs (fixed
      seed), with `Cover::require_all` proving every branch/region bucket was
      actually reached.
- [x] Phase C: all 33 `ERRORS.md` rows have a passing error-path differential
      test (62 documented rows ↔ 62 defined test fns, 1:1, verified by script).
- [x] Both hold under EVERY feature combination (the single valid combo: empty)
      and under both the `dev` and `release` profiles.
