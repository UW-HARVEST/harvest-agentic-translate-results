# SYMBOLS.md — Phase A symbol surface

## Source inventory

The whole C library is two files:

| C file | contents |
|--------|----------|
| `c_src/include/lib.h` | `bs_t`, `L12_scale_info` typedefs; declaration of `dequantize_granule` |
| `c_src/src/lib.c` | `static uint32_t get_bits(bs_t*, int)`, `int dequantize_granule(float*, bs_t*, L12_scale_info*, int)` |

No other translation unit exists, so no C module can have been skipped by the
translation. `translation/src/lib.rs` contains a translation of **both**
functions (`get_bits` private, `dequantize_granule` exported) plus both
`#[repr(C)]` structs.

## `nm -D` on the C shared library

```
$ nm -D --defined-only c_src/build/libharvest-work-GGsW52.so
00000000000011d1 T dequantize_granule
```

(the `.so` basename is derived from the parent directory name by
`CMakeLists.txt`: `cmake_path(GET parent FILENAME project_name)`)

## `nm -D` on the Rust shared library

```
$ nm -D --defined-only translation/target/debug/libdequantize_granule_lib.so
00000000000129f0 T dequantize_granule

$ nm -D --defined-only translation/target/release/libdequantize_granule_lib.so
0000000000011c70 T dequantize_granule
```

## Parity table

| # | symbol | C `.so` | Rust `.so` (debug) | Rust `.so` (release) | status |
|---|--------|---------|--------------------|----------------------|--------|
| 1 | `dequantize_granule` | `T` | `T` | `T` | ✅ exported by both |

### Symbols intentionally NOT exported

| symbol | reason |
|--------|--------|
| `get_bits` | `static` in `c_src/src/lib.c`; has internal linkage, absent from the C `.so` dynamic symbol table. The Rust translation keeps it as a private `unsafe fn`, which matches. Exporting it would be a *divergence*. |

### Diff

```
symbols in C .so but not in Rust .so : (none)
symbols in Rust .so but not in C .so : (none — after filtering the linker /
                                        libc boilerplate that both objects
                                        carry, see scripts/symbol_diff.sh)
```

**Result: 0 missing symbols. No C source was left untranslated.**

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the crate has exactly one configuration (the default, which is
also `--no-default-features`). Both are verified in
`scripts/check_feature_combos.sh`.

## Phase D completion gate

| gate | evidence | status |
|------|----------|--------|
| `nm -D`: 0 symbols missing from the Rust `.so` | `scripts/symbol_diff.sh` (exit 0), `tests/layout.rs::symbol_parity_between_c_and_rust_shared_objects` | ✅ |
| 0 undefined non-libc symbols in the Rust `.so` | `scripts/symbol_diff.sh` | ✅ |
| no C module left untranslated | `c_src` is 2 files / 2 functions, both present in `src/lib.rs`; no stubs, no `unimplemented!()` | ✅ |
| ABI layout of both public structs matches the C header | `tests/layout.rs::struct_offsets_match_the_c_header` (`bs_t` = 16 B; `L12_scale_info` = 900 B, fields at 0/768/769/770/834) | ✅ |
| every `CONFIGS.md` row passes over randomized inputs | `cargo test --test phase_b` — 41/41 | ✅ |
| every `ERRORS.md` row has a passing error-path test | `cargo test --test phase_c` — 28/28 | ✅ |
| holds under every feature combination x profile | `scripts/check_feature_combos.sh` — no `[features]` declared, so `{<default>, --no-default-features}` x `{debug, release}` = 4 configurations, all pass | ✅ |
| the suite actually has power (would catch a real mistranslation) | `scripts/mutation_check.py` — 15/15 injected mistranslations caught; the 1 provably-equivalent mutation survives as expected | ✅ |
| robust to a differently-optimised C build | whole suite re-run with `C_SO=` a `-O2` build of `c_src` — all pass | ✅ |

### Grep evidence that nothing was skipped

```
$ grep -c 'unimplemented!\|todo!\|unreachable!\|panic!' translation/src/lib.rs
0
$ grep -c '^\(static\|int\) ' c_src/src/lib.c      # 2 function definitions
2
$ grep -c 'fn get_bits\|fn dequantize_granule' translation/src/lib.rs
2
```
