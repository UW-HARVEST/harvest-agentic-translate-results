# SYMBOLS.md — dynamic-symbol parity

C library: `c_src/build/libharvest-work-8sO4iL.so` (built from `src/lib.c`)
Rust library: `translation/target/release/libcrc16_lib.so`

## `nm -D --defined-only` (non-weak, non-`V`) on the C `.so`

| # | symbol | C type | present in Rust `.so`? |
|---|--------|--------|------------------------|
| 1 | `crc16` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn crc16` |

Symbol diff (C-defined minus Rust-defined): **empty**.

## Notes on non-exported C entities

* `tflac_crc16_tables[8][256]` is declared `static const` in `include/lib.h`,
  so it has *internal* linkage and contributes **no** dynamic symbol. It is
  reproduced privately in `translation/src/tables.rs`; all 2048 constants were
  compared mechanically against the header and are byte-identical.
* `tflac_u8` / `tflac_u16` / `tflac_u32` are typedefs (no symbols).
* No namespacing/renaming macros exist in the header, so there are no
  macro-generated symbol aliases to mirror.

## Undefined (imported) symbols

The Rust `.so` imports only libc/`ld` runtime symbols (memcpy/unwind/etc.);
there are **0 missing/undefined non-libc symbols**.

## Verification commands

```sh
nm -D --defined-only c_src/build/*.so                  | grep -v ' [wV] '
nm -D --defined-only translation/target/release/*.so    | grep -v ' [wV] '
```
