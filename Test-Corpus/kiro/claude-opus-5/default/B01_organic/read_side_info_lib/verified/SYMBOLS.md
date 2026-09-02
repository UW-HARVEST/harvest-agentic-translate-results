# SYMBOLS.md — public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` — `c_src/build/libharvest-work-9L4ZMY.so`

```
$ nm -D --defined-only c_src/build/libharvest-work-9L4ZMY.so
00000000000011d1 T read_side_info
```

## Rust `.so` — `translation/target/release/libread_side_info_lib.so`

```
$ nm -D --defined-only translation/target/release/libread_side_info_lib.so
00000000000119f0 T read_side_info
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `read_side_info` | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` in `src/lib.rs` |

**Symbol diff (C-exported symbols missing from Rust): EMPTY.**

```
$ diff <(nm -D --defined-only <c.so>  | awk '{print $NF}' | sort -u) \
       <(nm -D --defined-only <rs.so> | awk '{print $NF}' | sort -u)
(no output)
```

## Non-exported C internals (verified present in the Rust translation)

`c_src/src/lib.c` contains exactly one `static` helper and three `static` tables
local to `read_side_info`. These have internal linkage in C, so they are not in
`nm -D`, but they must still be translated for behavioural parity:

| C internal | linkage | Rust counterpart |
|------------|---------|------------------|
| `static uint32_t get_bits(bs_t*, int)` | internal | `unsafe fn get_bits` |
| `static const uint8_t g_scf_long[8][23]` | internal (fn-local) | `static G_SCF_LONG: [[u8; 23]; 8]` |
| `static const uint8_t g_scf_short[8][40]` | internal (fn-local) | `static G_SCF_SHORT: [[u8; 40]; 8]` |
| `static const uint8_t g_scf_mixed[8][40]` | internal (fn-local) | `static G_SCF_MIXED: [[u8; 40]; 8]` |

No C source file / module is untranslated: the library is one `.c` file
(`src/lib.c`, 163 lines) and one header (`include/lib.h`, 16 lines), both fully
covered by `translation/src/lib.rs`.

## ABI / layout parity

`translation/src/layout_check.rs` const-asserts the two FFI structs. The
expected values were confirmed independently against gcc with an `offsetof`
probe:

```
bs_t          size=16 align=8   buf=0 pos=8 limit=12
L3_gr_info_t  size=32 align=8
  sfbtab 0  part_23_length 8  big_values 10  scalefac_compress 12
  global_gain 14  block_type 15  mixed_block_flag 16  n_long_sfb 17
  n_short_sfb 18  table_select 19  region_count 22  subblock_gain 25
  preflag 28  scalefac_scale 29  count1_table 30  scfsi 31
```

Both structs are fully packed (no interior or trailing padding), so the whole
32-byte granule record can be compared byte-for-byte across the FFI boundary.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configurations that exist are the default one and `--no-default-features`
(identical code). Both are checked/tested by `check_features.sh`.
