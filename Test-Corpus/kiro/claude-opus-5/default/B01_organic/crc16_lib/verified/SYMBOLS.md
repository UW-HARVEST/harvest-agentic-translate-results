# SYMBOLS.md — Symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-TbzjUt.so`
- Rust `.so`: `translation/target/release/libcrc16_lib.so`

## Exported (defined) dynamic symbols

`nm -D --defined-only` on the C `.so`:

| # | symbol | type | exported by Rust `.so`? |
|---|--------|------|-------------------------|
| 1 | `crc16` | `T` (global text) | YES — `T crc16` |

Total C exported symbols: **1**. Total present in Rust: **1**.

### Symbol diff

```
comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
         <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`.

## Why there is only one symbol

`c_src/include/lib.h` declares exactly one function:

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
```

Everything else in the header is:

- three `typedef`s (`tflac_u8`/`tflac_u16`/`tflac_u32`) — types, no linkage;
- `static const tflac_u16 tflac_crc16_tables[8][256]` — `static`, i.e. **internal
  linkage**, therefore deliberately *not* part of the ABI and correctly not
  exported by the Rust `.so` either.

`c_src/src/lib.c` re-declares the table as `static const tflac_u16
tflac_crc16_tables[8][256];` (a redundant declaration after the initialised
definition pulled in from the header) and defines `crc16`. No other translation
units exist (`CMakeLists.txt` lists only `src/lib.c`), so no C module was left
untranslated.

The table data was verified byte-for-byte, not by inspection:

```
grep -oE '0x[0-9a-fA-F]{4}' c_src/include/lib.h            > c_tab
grep -oE '0x[0-9a-fA-F]{4}' translation/src/tables.rs      > r_tab
diff c_tab r_tab   # -> identical, 2048 == 8*256 entries each
```

## Undefined symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc (`memcpy`, `malloc`, `free`,
`abort`, `open64`, …) and the platform unwinder (`_Unwind_*@GCC_*`) imports
pulled in by `libstd`. There are **0 missing/undefined non-libc, non-runtime
symbols**. The C `.so` imports only the four weak CRT hooks
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`), all of which the Rust `.so`
also has.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** — there is exactly one
build configuration (default). Symbol parity therefore holds for every feature
combination trivially; see `CONFIGS.md`.
