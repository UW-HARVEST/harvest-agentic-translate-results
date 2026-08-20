# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Generated mechanically from `nm -D` on both shared objects.

* C `.so`  : `c_src/build/libtranslated_rust.so` (built by `c_src/CMakeLists.txt`)
* Rust `.so`: `target/debug/libcapsule_lib.so` (`crate-type = ["cdylib"]`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so   | awk '$2=="T"{print $3}' | sort > c.txt
nm -D --defined-only target/debug/libcapsule_lib.so      | awk '$2=="T"{print $3}' | sort > r.txt
comm -23 c.txt r.txt      # symbols in C but not in Rust  -> MUST be empty
```

## Result

| metric | value |
|--------|-------|
| public (`T`) symbols exported by C `.so` | **38** |
| public (`T`) symbols exported by Rust `.so` | **38** |
| C symbols **missing** from Rust `.so` | **0** |
| Rust `.so` undefined non-libc / non-runtime symbols | **0** |

`comm -23 c.txt r.txt` output is empty. Parity is complete, and it holds for
**both** builds of the cdylib:

| Rust build | exported `T` symbols | missing vs C |
|------------|---------------------|--------------|
| `target/debug/libcapsule_lib.so` | 38 | 0 |
| `target/release/libcapsule_lib.so` (optimised, `panic = "abort"`) | 38 | 0 |

Parity is additionally enforced *at test time* rather than only offline:
`tests/smoke.rs::symbol_parity_via_dlsym` shells out to `nm -D` on the C `.so`
and then `dlsym`s every symbol it reports out of the Rust `.so` (opened with
`RTLD_NOW`, so an unresolved symbol would be a load failure). It also asserts
that the differential harness *binds* all 38, i.e. no C symbol is exported but
left untested.

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** ⇒ the only valid feature
  combination is the empty/default one. `cargo check --no-default-features`,
  `cargo check` and `cargo check --all-features` are therefore the complete
  matrix (verified `--all-targets`, all clean, zero warnings). `verify_all.sh`
  derives the power set of declared features from `Cargo.toml` mechanically, so
  it stays correct if features are ever added.
* Because there is only one feature combination, the second axis actually worth
  varying is the **codegen** one: the whole differential suite is run against the
  debug cdylib *and* against the release (optimised) cdylib. Both pass — which
  matters here, since the only tolerated difference in the whole verification is
  compiler-chosen NaN operand order (see `ERRORS.md`).
* `c_src/CMakeLists.txt` defines no options/`-D` switches: one target,
  one translation unit (`src/lib.c`), links `m`. There are **no `#if` /
  `#ifdef` / `#define` conditionals anywhere in `c_src`** (verified by grep),
  so the C side likewise has exactly one configuration.

## Symbol table

| # | symbol (C `.so`) | type | present in Rust `.so` |
|---|------------------|------|-----------------------|
| 1 | `c22` | T | yes |
| 2 | `c23` | T | yes |
| 3 | `c2AABBtoAABB` | T | yes |
| 4 | `c2AABBtoCapsule` | T | yes |
| 5 | `c2Add` | T | yes |
| 6 | `c2BBVerts` | T | yes |
| 7 | `c2CCW90` | T | yes |
| 8 | `c2CapsuletoCapsule` | T | yes |
| 9 | `c2CircletoAABB` | T | yes |
| 10 | `c2CircletoCapsule` | T | yes |
| 11 | `c2CircletoCircle` | T | yes |
| 12 | `c2Clampv` | T | yes |
| 13 | `c2Collided` | T | yes |
| 14 | `c2D` | T | yes |
| 15 | `c2Det2` | T | yes |
| 16 | `c2Div` | T | yes |
| 17 | `c2Dot` | T | yes |
| 18 | `c2GJK` | T | yes |
| 19 | `c2GJKSimplexMetric` | T | yes |
| 20 | `c2L` | T | yes |
| 21 | `c2Len` | T | yes |
| 22 | `c2MakeProxy` | T | yes |
| 23 | `c2Maxv` | T | yes |
| 24 | `c2Minv` | T | yes |
| 25 | `c2Mulrv` | T | yes |
| 26 | `c2MulrvT` | T | yes |
| 27 | `c2Mulvs` | T | yes |
| 28 | `c2Mulxv` | T | yes |
| 29 | `c2Neg` | T | yes |
| 30 | `c2Norm` | T | yes |
| 31 | `c2RotIdentity` | T | yes |
| 32 | `c2Skew` | T | yes |
| 33 | `c2Sub` | T | yes |
| 34 | `c2Support` | T | yes |
| 35 | `c2V` | T | yes |
| 36 | `c2Witness` | T | yes |
| 37 | `c2xIdentity` | T | yes |
| 38 | `capsule` | T | yes |

`include/lib.h` only declares `capsule`, but the C `.so` exports all 38
non-`static` functions, so all 38 are part of the ABI under test and all 38 are
driven through `dlopen`/`dlsym` by the differential tests.

## Undefined-symbol audit

C `.so` imports: `sqrtf@GLIBC` plus the usual weak `_ITM_*`, `__cxa_finalize`,
`__gmon_start__`.

Rust `.so` imports: only libc (`memcpy`, `malloc`, `free`, `abort`, …), the
Rust std runtime's unwinder (`_Unwind_*`) and glibc/pthread symbols pulled in by
`std`. **No unresolved project symbols** — nothing that a loader would fail on
(verified: the tests successfully `dlopen` the Rust `.so` with `RTLD_NOW`).

## Harness integrity

Two defects in the *test harness* were found and fixed while verifying; both
could have produced false "all passed" results, so they are recorded here.

1. **Stale shared object.** `cargo test` does not rebuild a `crate-type =
   ["cdylib"]`-only library, because no test target depends on it — the suite
   happily `dlopen`ed a `libcapsule_lib.so` from an earlier build. Fixes:
   * `crate-type = ["cdylib", "rlib"]`, so the library *is* a dependency of the
     integration tests and cargo rebuilds it (the shipped cdylib and its 38
     exported symbols are unchanged — re-verified above);
   * during `cargo test` cargo writes the cdylib to `target/<profile>/deps/` but
     only *uplifts* it to `target/<profile>/` for `cargo build`, so
     `common::rust_so_path()` now picks the **newest** of the two;
   * `common::load_pair()` asserts that each `.so` is at least as new as its
     source (`src/lib.rs`, `c_src/src/lib.c`) and fails loudly otherwise.
2. **Crash counted as success.** `mutation_check.sh` originally decided
   "mutation caught?" by grepping for `N failed`, which misses a mutant that
   makes the test process *abort* (e.g. a NULL dereference: `SIGABRT`). It now
   uses cargo's exit status, and distinguishes a build break from a behavioural
   failure.

## Mutation battery — evidence the suite is not vacuous

`./mutation_check.sh` injects 20 plausible translation slips into `src/lib.rs`
one at a time and requires each to be **caught**; `src/lib.rs` is restored after
every mutation and on exit (verified byte-identical afterwards by `md5sum`).

```
mutations caught: 20, NOT caught: 0
MUTATION BATTERY PASSED
```

Covered slips include: `c2Skew` ↔ `c2CCW90`, a flipped `c2Det2` sign, `<` → `<=`
at the touching boundary in `c2CircletoCircle` / `c2AABBtoAABB`, "fixing" the C's
swapped `AABB × CIRCLE` operands in `c2Collided`, `>` → `>=` in the GJK
no-progress and radius guards, dropping the `-1e8` clause of the cache guard,
mis-checking a NULL out-parameter, swapping the `c22` collapse branches, a wrong
barycentric sign in `c23`, `c2Support` tie-breaking the other way, giving
`c2MakeProxy` a `default:` arm the C does not have, rotating the `c2BBVerts`
vertex order, dropping a `c2Witness` term, a wrong endpoint in
`c2CircletoCapsule`, and changing `capsule()`'s reference shapes and result bits.

Two mutations were found to be **provably equivalent** rather than undetectable,
and were replaced by non-equivalent variants (both documented in the script):

* `while (iter < 20)` → `19`: unobservable, since the highest reachable iteration
  count is 5 (`ERRORS.md` row E29). Lowering the cap into the reachable range
  (`< 2`) *is* caught, by 38 tests.
* `da < 0` → `da <= 0` in `c2CircletoCapsule`: the three distance formulas agree
  exactly on the branch boundary (at `da == 0` the perpendicular branch computes
  `ap - n*(0/|n|²) == ap`), so no input can distinguish them. Perturbing the
  formula instead *is* caught, by 6 tests.
