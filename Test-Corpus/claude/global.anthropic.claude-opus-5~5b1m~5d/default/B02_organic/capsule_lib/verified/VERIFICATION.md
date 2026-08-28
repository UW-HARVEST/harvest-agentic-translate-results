# VERIFICATION.md — completion gate roll-up

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth). Every assertion loads **both** shared objects with `libloading` and calls
them through their exported symbols, so the Rust `#[no_mangle]` export wrappers
and the C ABI are on the test path.

```
C   .so : c_src/build/libharvest-work-huLMrZ.so   (cmake, 38 exported symbols)
Rust.so : translation/target/{release,debug}/libcapsule_lib.so   (cdylib, 38)
```

## Artifacts

| file | what it is |
|------|-----------|
| `SYMBOLS.md` | all 38 `nm -D` symbols, C ↔ Rust, with source line numbers |
| `ERRORS.md` | 83 error-surface rows mechanically derived from the C |
| `CONFIGS.md` | 87 configuration rows (options × input shapes) |
| `scripts/check_symbols.sh` | Phase D symbol diff, must be empty |
| `scripts/check_features.sh` | every feature combo × release/debug |
| `vendor/` + `.cargo/config.toml` | `libloading` vendored, so `cargo test` needs no network and no warm registry cache |

## Test suite

| file | tests | covers |
|------|------:|--------|
| `tests/common/mod.rs` | — | harness: dual `dlopen`, POD types, bit-exact compare, seeded PRNG |
| `tests/harness_selfcheck.rs` | 4 | **proves the harness detects divergence** (see below) |
| `tests/phase_b_group1_math.rs` | 14 | C1–C15 leaf vector maths |
| `tests/phase_b_group2_proxy.rs` | 5 | C16–C20 `c2BBVerts`, `c2MakeProxy` |
| `tests/phase_b_group3_simplex.rs` | 7 | C21–C45 simplex internals (low-level entry points) |
| `tests/phase_b_group4_gjk.rs` | 12 | C46–C77 `c2GJK` option cross-product |
| `tests/phase_b_group5_collision.rs` | 10 | C78–C87 predicates + `capsule()` |
| `tests/phase_c_errors.rs` | 28 | every non-excluded `ERRORS.md` row |
| `tests/phase_bd_fuzz.rs` | 4 | high-volume cross-symbol fuzz (~100M calls) |
| **total** | **84** | |

### The harness is self-checking

`harness_selfcheck.rs` deliberately cross-compares C `c2Skew` against Rust
`c2CCW90` and asserts the comparison macro **panics**; it also asserts that
`+qNaN` vs `−qNaN` and `+0.0` vs `−0.0` are reported as divergences. Without
this, "all tests pass" could mean "the assertions never fire".

## Comparison strictness

* Every `f32` is compared by `to_bits()`, so `−0.0` ≠ `+0.0` and **NaN payloads
  and sign bits must match exactly**.
* Every struct is compared as raw bytes (`c2Simplex` = all 152 bytes,
  `c2Proxy` = all 72 bytes including the `verts[count..8]` slots the C never
  writes, `c2GJKCache` = all 36 bytes).
* Out-parameters are pre-filled with distinctive sentinels, so a *missing* store
  is detected rather than silently accepted.
* `*iterations` and the post-call cache contents are compared on every `c2GJK`
  call, not just the return value.

The bit-exactness bar was justified by disassembling the C: GCC emits
`mulss`/`addss`/`subss`/`divss`/`sqrtss` with a specific **destination operand**
at each site, which decides NaN-payload propagation. The Rust `mul_ss`/`add_ss`/
`sub_ss`/`div_ss` helpers were verified against the actual instruction stream for
`c2Dot`, `c2Det2`, `c2Add`, `c2Sub`, `c2Mulvs`, `c2Mulrv`, `c2MulrvT`, `c2Div`,
`c2Neg`, `c2Skew`, `c2Len`, `c23` and the `c2GJK` radius block — and then
confirmed empirically by exhaustive 20⁴ = 160 000-case special-value sweeps on
`c2Dot` and `c2Det2` (`0x7f80_0001` sNaN, `0x7fab_cdef` odd payload, `±0`,
denormals, `±inf`, `FLT_MAX`).

## Findings

Three kinds of finding came out of the work. **No divergence between the C and
the Rust was ever observed** — the Rust translation is bit-exact everywhere it
is defined. What the process *did* find were errors in my own initial reading of
the C, which were corrected in the artifacts (the C is always right):

1. **`ERRORS.md` E17/E18 are UB, not a defined behaviour.** A cache index in
   `[proxy.count, 8)` reads the indeterminate tail of the uninitialised
   `c2Proxy pA;` (L376). A probe over `iA ∈ 1..8` showed the C returning stack
   residue (`rc = +inf`, `outA = (-1.2e30, 3.09e-38)`) for 5 of 7 indices. Moved
   to the excluded list with the evidence; `CONFIGS.md` C64 was repurposed to the
   *defined* cache-validity axis instead.
2. **`ERRORS.md` E60/E65/E67 over-generalised NaN propagation.** `c2Maxv`/`c2Minv`
   are `x > y ? x : y`, so a NaN in the **first** operand *loses* and is
   discarded — a NaN in `B.min` does **not** propagate through `c2Clampv`.
   Likewise `c2CircletoCapsule`'s final `bp` arm uses neither `n` nor `ap`, so a
   NaN in `B.a`/`B.b` is discarded; and a single NaN in `c2AABBtoAABB` only
   falsifies the comparisons it participates in. Rows split into
   E60/E60b, E65/E65b/E65c, E67/E67b, and the tests now derive the expected
   sentinel from a transliteration of the C body instead of a guess.
3. **`ERRORS.md` E26 (`iter == 20`) is unreachable** through the public API. All
   three shape types have ≤ 4 vertices; the 3-slot duplicate-support check plus
   the monotone `d1 > d0` guard bound the loop. 3.4 M randomized calls (finite,
   wild, NaN, warm-started, rotated, non-normalised rotations) never exceeded
   `*iterations == 4`. The test therefore asserts the two libraries' full
   iteration **histograms** and their reachable **maxima** are identical.

## Volume

`phase_bd_fuzz.rs` (seeded, wall-clock-bounded) executed on the last run:

```
fuzz_all_leaf_functions        : 4 354 000 rounds x 19 symbols  (~83M calls)
fuzz_gjk_full_option_space     : 10 312 000 c2GJK calls (4-call warm-start chains)
fuzz_simplex_pipeline          : 3 939 000 rounds (c22/c23 -> c2D/c2L/c2Support/c2Witness)
fuzz_predicates_and_entry_point:   688 000 rounds x 10 symbols
```

Roughly **10⁸ paired C/Rust calls**, zero divergences.

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / 0 unresolved non-libc symbols in
      Rust.** 38 C exports, 38 Rust exports, `comm -3` diff empty in both
      directions; every Rust import is glibc or the platform unwinder.
      Re-checkable with `scripts/check_symbols.sh`.
- [x] **Phase B: every row in `CONFIGS.md` (C1–C87) passes across randomized
      inputs.** Each row is driven with thousands to hundreds of thousands of
      seeded pseudo-random inputs plus exhaustive special-value sweeps, not a
      single hand-picked value. Coverage counters assert that both outcomes of
      every predicate, all 3 arms of `c22`, all 7 arms of `c23`, all 3 arms of
      `c2CircletoCapsule`, the `hit`/midpoint/shrink paths of `c2GJK`, and ≥ 5
      distinct `capsule()` results were actually reached.
- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential
      test.** E1–E15, E20–E41, E43–E74 plus the E60b/E65b/E65c/E67b refinements.
      Each asserts the *same specific sentinel* (`0`, `1`, `+0.0f`, `{0,0}`, a
      given NaN), never merely "both failed". Out-of-range enum values
      (`3`, `−1`, `0x7fffffff`, `INT_MIN`, …), null pointers in all 6 nullable
      parameters (8 combinations), out-of-range `count` values
      (`0`, `4`, `−1`, `INT_MIN`, `INT_MAX`), `±0` divisors and one-ULP boundary
      steps are all covered. E16/E17/E18/E19/E42 are excluded as C-level UB with
      empirical evidence and a tested defined neighbour each.
- [x] **All of the above hold under every feature combination.**
      `Cargo.toml` declares **no `[features]` table**, so the only combination is
      the default; `scripts/check_features.sh` derives this mechanically from
      `Cargo.toml` and still runs the full suite under
      `--no-default-features`, `--all-features` and the default, **× release and
      debug profiles** = 6 configurations. All 6 green (84/84 tests each).
- [x] Zero compiler warnings from `cargo test --release`.
- [x] A bare `cargo test` on a **clean tree** works: `cargo test` alone does not
      emit the cdylib artifact for a `crate-type = ["cdylib"]` library, so the
      harness builds it on demand into `target/harness-cdylib/<profile>/` (a side
      target dir, because cargo already holds a lock on `target/`).  `RUST_SO=`
      and `C_SO=` override the discovered paths.

## Reproduce

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation
./scripts/check_symbols.sh      # Phase D symbol parity
./scripts/check_features.sh     # every feature combo x release/debug
cargo test --release -- --nocapture   # coverage counters printed
```
