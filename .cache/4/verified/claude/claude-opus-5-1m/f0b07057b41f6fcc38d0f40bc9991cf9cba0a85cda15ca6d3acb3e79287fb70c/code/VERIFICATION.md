# VERIFICATION.md — final report

C-to-Rust differential verification of the `c_src/` GJK library (a `cute_c2`
derivative: 1 header, 1 translation unit, 530 lines, 31 exported symbols).

Every test loads **both** shared libraries with `libloading` and calls them
through their exported symbols. No Rust function is ever called directly, so the
`#[unsafe(no_mangle)] extern "C"` wrappers and the SysV struct-by-value ABI
(`c2v`/`c2r`/`c2x` returned in `XMM0`/`XMM1`) are themselves under test.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** missing symbols and 0 undefined
      non-libc symbols in the Rust `.so` (31 / 31 C exports present).
- [x] **Phase B** — all **102** rows of `CONFIGS.md` pass across randomised
      inputs (fixed seeds).
- [x] **Phase C** — all **70** rows of `ERRORS.md` have a passing error-path
      differential test.
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]` and
      `c_src` has no `#ifdef`/`option()`, so the power set is exactly ONE
      combination; `check_all_features.sh` derives it mechanically and verifies
      check + build + symbol parity + full test run for it.

**149 tests, 0 failures.** No change was made to anything in `c_src/`.

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | 31-row symbol parity table + build-configuration enumeration |
| `ERRORS.md` | 70-row error/rejection surface table, each row -> a named test |
| `CONFIGS.md` | 102-row configuration surface table, each row -> a named test |
| `check_all_features.sh` | enumerates feature combos, runs check/build/symbols/tests for each |
| `mutation_check.sh` | injects 20 known bugs into `src/lib.rs`, asserts the suite catches them |
| `tests/common/mod.rs` | dual-`.so` loader, mirrored C structs, seeded PRNG, bit-exact compare macros |

Cross-checks that the tables are honest (not just prose):

```sh
# every ERRORS.md / CONFIGS.md row names a test that actually exists
grep -oP '^\| \d+ \|[^|]*\|[^|]*\|[^|]*\| `\K[A-Za-z0-9_]+' ERRORS.md  | sort -u > a
grep -hoP '^fn \K[A-Za-z0-9_]+' tests/*.rs | sort -u > b
comm -23 a b     # -> EMPTY
grep '^| [0-9]' ERRORS.md  | grep -vc '\[x\]'   # -> 0 unchecked rows
grep '^| [0-9]' CONFIGS.md | grep -vc '\[x\]'   # -> 0 unchecked rows
```

## Test layout

| binary | tests | covers |
|--------|------:|--------|
| `phase_b_level0.rs` | 13 | CONFIGS 1-13: pure vector/scalar helpers |
| `phase_b_level1.rs` | 4 | CONFIGS 14-22: `c2BBVerts`, `c2MakeProxy`, `c2Support` |
| `phase_b_level2.rs` | 9 | CONFIGS 23-35: `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/metric + composed pipeline |
| `phase_b_gjk.rs` | 20 | CONFIGS 36-71: `c2GJK` option cross-product |
| `phase_b_wrapper.rs` | 9 | CONFIGS 72-79: the public `gjk()` |
| `phase_b_nan_payload.rs` | 10 | CONFIGS 80-88: dense NaN payloads |
| `phase_b_nan_sparse.rs` | 10 | CONFIGS 89-98: sparse NaN injection |
| `phase_c_iteration_cap.rs` | 4 | CONFIGS 99-102 / ERRORS 25: iteration-count reachability |
| `phase_c_errors.rs` | 34 | ERRORS 1-31, 68-70 |
| `phase_c_errors2.rs` | 36 | ERRORS 32-67 |

Everything is compared **bit-for-bit**: `f32::to_bits()` for scalars (so the sign
of zero and NaN payloads count) and raw byte slices for the mirrored structs,
which were checked to be padding-free (`c2sv` = 36 B, `c2Simplex` = 152 B,
`c2Proxy` = 72 B, `c2GJKCache` = 36 B). `PartialEq` on floats is deliberately
avoided — `NaN != NaN` would both mask real divergences and invent fake ones.

## Branch coverage actually achieved

Not assumed — counted at runtime and asserted:

* `c22` — all **3** Voronoi arms hit: `[21606, 12608, 65786]`.
* `c23` — all **7** Voronoi arms hit: `[6409, 4616, 4910, 40652, 31705, 30545, 34715]`,
  with **0** unclassified.
* `c2MakeProxy` / `c2Support` / `c2D` / `c2L` / `c2Witness` /
  `c2GJKSimplexMetric` — every `case` **and** every `default:` arm, driven with
  out-of-range `count` values (`0, 4, 5, -1, 7, 100, INT_MAX, INT_MIN`).
* `c2GJK` — the hit path, the radius-midpoint path, the shrink-collapse path,
  the `d1 > d0` break, the degenerate-direction break, the duplicate-support
  break, and both sides of the cache-metric guard were each confirmed reached
  (the tests assert a nonzero hit count, they do not assume it).

## Mutation testing — proof the suite can actually detect divergence

Passing tests prove nothing unless they can fail. `./mutation_check.sh` injects
20 deliberate bugs into `src/lib.rs` one at a time and re-runs everything:

```
mutation score: 18 killed / 2 known-equivalent / 0 unexplained survivors
```

The two survivors are **provably semantics-preserving**, not blind spots:

1. **`while (iter < 20)` -> `iter < 19`.** The highest iteration count *any*
   input produces is **5**, because a proxy holds at most 4 verts so the simplex
   always resolves quickly. Established by bisection, not assumption: replacing
   the C's `20` with `5` DOES make tests fail, while `6`..`19` are
   indistinguishable. The literal `20` is unreachable code.
2. **`s.a.u = 1.0f` (cold start) -> `0.5f`.** `verts[0].u` is only ever read by
   `c2Witness`/`c2L`, and both ignore `u` in their `count == 1` arms. Any exit
   with `count >= 2` has already run `c22`/`c23`, and every arm of those writes
   `verts[0].u`. So the cold-start value can never be observed. Confirmed by a
   control mutation on the neighbouring line: `s.div = 1.0f -> 2.0f` in the same
   block is killed by **14** tests, so the block itself is well covered.

Mutation testing also **found a real gap** in an earlier draft of the suite and
drove a fix. Swapping the operand order of a `mulp`/`addp` (a NaN-payload-only
difference) initially survived, because the first NaN-payload test filled *every*
input with NaN — and one NaN then wins early and masks the operand-order choice
being tested. That is why `tests/phase_b_nan_sparse.rs` exists: it injects NaNs
into exactly one or two slots with everything else finite, for every slot pair,
which isolates each individual `ADDSS`/`MULSS`. With it in place, swapping any
observable operand order is caught (e.g. `c2Dot`'s `addp` destination -> 3
failing tests; its second `mulp` -> 1).

## Notable behaviours of the C that the Rust reproduces exactly

These look like bugs but are the ground truth and are preserved verbatim:

* `c2GJKSimplexMetric`'s `switch` has `default:` **falling through into**
  `case 1:`, so every out-of-range `count` returns `+0.0f`.
* `c2Support` dereferences `verts[0]` *before* the loop guard, so `count == 0`
  and even `count < 0` still read `verts[0]` and return index `0`.
* `c2Support` uses a strict `>`, so an exact tie keeps the **lowest** index.
* `c2Maxv`/`c2Minv` are built from `a > b ? a : b`, which returns `b` whenever
  either operand is NaN — an asymmetry the Rust matches with the same ordering.
* `c2Clampv` does not validate `lo <= hi`; with `lo > hi` it always returns `lo`.
* `c2Norm` of the zero vector divides by zero, giving `(NaN, NaN)`.
* `c2GJK` treats a negative `cache->count` as "cache is good" (`!!count`), then
  loops zero times, leaving `s.count` negative so every later `switch` takes its
  `default:` arm.
* No function validates a radius, so negative radii flow straight through and
  `dist -= rA + rB` *increases* the distance.
* `gjk()`'s `reverse` is a `char`, so only its low byte matters.

## Undefined behaviour in the C — deliberately excluded from byte-exactness

Two input classes reach C UB, where the "correct" answer is whatever stack
garbage the C compiler leaves behind. These are still tested for *"does not
crash and the defined observables agree"*, but their indeterminate bytes are not
— and must not be — asserted equal:

1. **Out-of-range `typeA`/`typeB` in `c2GJK`** (`ERRORS.md` rows 12/13).
   `c2Proxy pA;` (L371) is an uninitialised local and `c2MakeProxy`'s `switch`
   has no `default:`, so `pA.count`/`pA.radius`/`pA.verts` are indeterminate.
   Rust zero-initialises them. The tests therefore assert only which
   out-parameters get written and that `*iterations` stays in `0..=20`.
   The *defined* form of the same behaviour — `c2MakeProxy` called directly with
   a caller-owned buffer — IS asserted byte-for-byte (row 14), including that a
   valid type leaves `verts[count..8]` still holding their poison bytes (row 15).
2. **`cache->count > 3`, or `cache->iA[i]` >= the proxy's vertex count.**
   `cache->iA`/`iB` are `int[3]`, and the simplex has room for 4 vertices, so a
   larger `count` overruns both the cache arrays and `c2Simplex` on the C stack;
   an out-of-range index reads proxy verts the chosen shape type never wrote.
   Tests keep `count <= 3` and indices `< count` so both libraries stay in
   defined territory (`ERRORS.md` rows 68/69 and the note beneath the table).

## The C's NaN payloads are not stable across its own optimisation levels

Worth recording, because it bounds what "byte-identical" can mean here.

`x86` `ADDSS`/`MULSS` return the **destination** operand's NaN in preference to
the source's. For commutative `a + b` / `a * b` the compiler picks which operand
lands in the destination register, so that choice — and therefore which NaN
payload survives — is an *optimisation-level artefact*. `src/lib.rs` models it
explicitly with `addp(dst, src)` / `mulp(dst, src)` (for non-NaN operands these
are exactly `+` and `*`, so ordinary arithmetic is untouched).

Verified by building the C both ways and re-running the suite against each:

| C build | result |
|---------|--------|
| CMake default (no `CMAKE_BUILD_TYPE`, no `-O`) — **the configuration under test** | **149 / 149 pass** |
| `-DCMAKE_BUILD_TYPE=Release` (`-O3`) | 47 tests differ — and **every** difference is NaN-payload-only |

Classified mechanically over all 41 scalar reports and all 4 struct-byte reports
from the `-O3` run: **0 genuinely different values**; in every case both sides
are NaN and only the payload/sign bits differ. All non-NaN arithmetic, all
control flow, all indices, all iteration counts and all struct layouts agree
under both C builds. The Rust is pinned to the default configuration, which is
the one this task specifies; the `-O3` deltas are the C disagreeing with itself.

The Rust `.so` was also verified in **both** its own profiles — `--release` and
debug — against the default C build: 149 / 149 in each.

## Reproducing

```sh
# 1. build the C reference (default configuration)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

# 2. full differential suite (auto-builds the Rust cdylib into target/difftest)
cargo test --release

# 3. every feature combination, with symbol parity per combination
./check_all_features.sh

# 4. prove the suite can detect divergence
./mutation_check.sh
```

Optional overrides: `GJK_C_SO` / `GJK_RUST_SO` point the harness at specific
`.so` files (used above to test the `-O3` C build and the debug Rust build).
