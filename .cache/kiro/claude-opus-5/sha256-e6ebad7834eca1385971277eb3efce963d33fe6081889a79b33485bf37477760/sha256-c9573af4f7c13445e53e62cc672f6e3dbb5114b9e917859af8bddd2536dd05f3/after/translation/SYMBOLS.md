# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

## Commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libharvest-work-ZcmQya.so

# Rust
cd translation && cargo build --release
nm -D --defined-only translation/target/release/libdequantize_granule_lib.so
```

Both are re-run automatically by `translation/run_all.sh` and asserted by
`tests/phase_d_symbols.rs`.

## C source surface

`c_src/` contains exactly one translation unit (`src/lib.c`, 43 lines) and one
public header (`include/lib.h`, 14 lines). There are **no untranslated
modules**: `CMakeLists.txt` lists `src/lib.c` as the only source file, so the
whole library is covered.

| C source entity | linkage | exported? | Rust counterpart |
|---|---|---|---|
| `dequantize_granule` | external | yes | `dequantize_granule` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |
| `get_bits` | `static` | no (internal) | private `unsafe fn get_bits` — correctly NOT exported |
| `bs_t` (typedef struct) | type | n/a | `#[repr(C)] pub struct bs_t` |
| `L12_scale_info` (typedef struct) | type | n/a | `#[repr(C)] pub struct L12_scale_info` |

No macro-generated symbols, no function-pointer tables, no global or static
data objects (`nm` reports no `D` / `B` / `R` entries in the C `.so`), no weak
definitions, no versioned exports.

## Defined-symbol diff

C `.so`:

```
00000000000011d1 T dequantize_granule
```

Rust `.so`:

```
00000000000116d0 T dequantize_granule
```

```sh
comm -23 <(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u) \
         <(nm -D --defined-only "$RUST_SO"| awk '{print $3}' | sort -u)
# -> (empty)
```

`comm -13` (extra in Rust) is also empty — the Rust `cdylib` exports nothing
beyond `dequantize_granule`.

**Result: 0 missing symbols, 0 extra symbols. Symbol parity is exact.**

Asserted by:
* `phase_d_symbols::every_c_symbol_is_exported_by_rust`
* `phase_d_symbols::static_c_helper_is_not_exported_by_either` (`get_bits` must
  stay internal in both)

## Undefined (imported) symbols

C `.so` — 4 weak entries, all loader/glibc boilerplate:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

Rust `.so` — 49 entries. Every one is either version-tagged `@GLIBC_*` (36) or
`@GCC_*` (11 `_Unwind_*` entries from the panic runtime), plus the same two weak
`_ITM_*` entries and `__gmon_start__` the C `.so` has:

```sh
nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
  | grep -v -E '@GLIBC_|@GCC_|^_ITM_|^__gmon_start__$'
# -> (empty)
```

**0 undefined non-libc symbols in the Rust `.so`.**

Asserted by `phase_d_symbols::rust_so_has_no_undefined_non_libc_symbols`.

## ABI layout parity

`src/lib.rs` pins the two struct layouts with compile-time `const` assertions
(`size_of`, `offset_of`). Those are checked *behaviourally* against the C as
well, because `dequantize_granule` reads `sci->bitalloc[i]` out of bounds and
the exact bytes it lands on depend on the layout:

| property | value | how it is verified against the C |
|---|---|---|
| `sizeof(bs_t)` | 16 | `phase_d_symbols::bs_t_layout_matches_c` |
| `offsetof(bs_t, pos)` / `limit` | 8 / 12 | same test — a wrong offset would break the `limit` rejection |
| `sizeof(L12_scale_info)` | 900 | `phase_d_symbols::l12_scale_info_layout_matches_c` |
| `offsetof(total_bands)` | 768 | any wrong offset changes the `i`-loop bound |
| `offsetof(bitalloc)` | 770 | `phase_c_error_paths::row15_out_of_bounds_bitalloc_read` |
| `offsetof(scfcod)` | 834 | `l12_scale_info_layout_matches_c` — `bitalloc[64]` must land exactly on `scfcod[0]` |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the complete
set of feature combinations is a single one: the default (empty) set.
`run_all.sh` derives the list from `Cargo.toml` (so it keeps working if features
are added later) and currently runs `cargo check --all-targets`,
`cargo build --release`, the `nm -D` diff, and the full test suite under both
`<default features>` and `--no-default-features`. Both report
`0 missing symbols` / `0 undefined non-libc symbols` and all 47 tests passing.
