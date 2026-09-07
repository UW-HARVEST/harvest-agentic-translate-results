# SYMBOLS.md — Phase A symbol surface

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-6ojq4X.so   | awk '{print $3}' | sort -u
nm -D --defined-only translation/target/release/libgjk_cache_lib.so | awk '{print $3}' | sort -u
```

The C `.so` exports **31** symbols (the whole `c_src/src/lib.c` is one
translation unit with no `static` functions, so every function is public).
There is no binary/driver target: `c_src/CMakeLists.txt` builds only
`add_library(... SHARED src/lib.c)` and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]`. The stdout-comparison gate is therefore N/A.

## Symbol table

| # | C symbol | in Rust `.so` | Rust item | notes |
|---|----------|---------------|-----------|-------|
| 1 | `c22` | yes | `c22` | simplex reduction, 2 points |
| 2 | `c23` | yes | `c23` | simplex reduction, 3 points |
| 3 | `c2Add` | yes | `c2Add` | |
| 4 | `c2BBVerts` | yes | `c2BBVerts` | writes 4 verts |
| 5 | `c2CCW90` | yes | `c2CCW90` | |
| 6 | `c2Clampv` | yes | `c2Clampv` | |
| 7 | `c2D` | yes | `c2D` | search direction |
| 8 | `c2Det2` | yes | `c2Det2` | |
| 9 | `c2Div` | yes | `c2Div` | div-by-zero possible |
| 10 | `c2Dot` | yes | `c2Dot` | |
| 11 | `c2GJK` | yes | `c2GJK` | main entry point |
| 12 | `c2GJKSimplexMetric` | yes | `c2GJKSimplexMetric` | |
| 13 | `c2L` | yes | `c2L` | closest point on simplex |
| 14 | `c2Len` | yes | `c2Len` | `sqrtf` |
| 15 | `c2MakeProxy` | yes | `c2MakeProxy` | enum-dispatched |
| 16 | `c2Maxv` | yes | `c2Maxv` | ternary NaN semantics |
| 17 | `c2Minv` | yes | `c2Minv` | ternary NaN semantics |
| 18 | `c2Mulrv` | yes | `c2Mulrv` | |
| 19 | `c2MulrvT` | yes | `c2MulrvT` | |
| 20 | `c2Mulvs` | yes | `c2Mulvs` | |
| 21 | `c2Mulxv` | yes | `c2Mulxv` | |
| 22 | `c2Neg` | yes | `c2Neg` | sign of zero observable |
| 23 | `c2Norm` | yes | `c2Norm` | NaN on zero vector |
| 24 | `c2RotIdentity` | yes | `c2RotIdentity` | |
| 25 | `c2Skew` | yes | `c2Skew` | |
| 26 | `c2Sub` | yes | `c2Sub` | |
| 27 | `c2Support` | yes | `c2Support` | reads `verts[0]` unconditionally |
| 28 | `c2V` | yes | `c2V` | |
| 29 | `c2Witness` | yes | `c2Witness` | |
| 30 | `c2xIdentity` | yes | `c2xIdentity` | |
| 31 | `gjk_cache` | yes | `gjk_cache` | the declared public API in `include/lib.h` |

## Diff result

```
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   ->  (empty)
```

**0 symbols missing from the Rust `.so`.** No module of the C source was
skipped; `lib.c` is fully translated in `translation/src/lib.rs`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind
imports (`malloc`, `memcpy`, `sqrt`-free, `_Unwind_*`, `__cxa_finalize`, …).
**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the only feature
configuration is the default (empty) one. `scripts/verify_all.sh` enumerates
features from `Cargo.toml` and confirms this; it then runs, for each
configuration, a `nm -D` diff plus the whole differential suite. Both the
default configuration and `--no-default-features` show 31/31 symbol parity, 0
undefined non-libc symbols, and 0 test failures.

## How to reproduce

```sh
# 1. build the C shared library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. build the Rust cdylib and verify every configuration
cd ../../translation && cargo build --release && ./scripts/verify_all.sh

# 3. (optional) measure how sensitive the suite is
./scripts/mutation_battery.sh
```

`scripts/verify_all.sh` is the Phase D gate: it regenerates the symbol diff and
runs the full suite for every feature combination. The suite can be pointed at
alternative libraries with `DIFF_C_SO=` / `DIFF_RUST_SO=`, which is how the
debug-profile cdylib and the `-O0`/`-O1`/`-O2`/`-O3` C builds were verified.

## Result summary

| gate | result |
|---|---|
| `nm -D` symbols missing from the Rust `.so` | **0** (31/31) |
| Undefined non-libc symbols in the Rust `.so` | **0** |
| Phase B — every `CONFIGS.md` row, randomized | **50/50 pass** |
| Phase C — every `ERRORS.md` row | **46/46 pass** |
| Binary/driver stdout comparison | **N/A** — neither build produces an executable |
| Feature combinations | **2/2 pass** (default, `--no-default-features`) |
| Mutation battery | 55 mutants: 46 killed, 9 provably-equivalent survivors |
| C at `-O0`/`-O1`/`-O2`/`-O3` | **0 failures at each level** |

Total: **50 test functions** across 4 integration-test files, together making
roughly 1.5 million differential `.so`-to-`.so` calls per run (~0.5 s).
