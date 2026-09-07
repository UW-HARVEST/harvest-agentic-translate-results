# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared object
`c_src/build/libharvest-work-ZCIxQf.so` (built via CMake).

## C `.so` exported (defined, global) symbols

| # | symbol | type | declared in | translated in Rust | exported by Rust `.so` |
|---|--------|------|-------------|--------------------|------------------------|
| 1 | `flac_validate`     | `T` (text, global) | `include/lib.h` | `translation/src/lib.rs` | yes (`#[unsafe(no_mangle)] pub unsafe extern "C"`) |
| 2 | `tflac_size_memory` | `T` (text, global) | *not* in public header; defined in `src/lib.c` | `translation/src/lib.rs` | yes (`#[unsafe(no_mangle)] pub extern "C"`) |

Notes:
* The C `.so` exports no data symbols and no macro-generated symbols.
* `enum TFLAC_CHANNEL_MODE` is a compile-time-only C enum (no symbol emitted);
  it is mirrored in Rust as `pub const TFLAC_CHANNEL_*: tflac_u8`.
* Undefined (`U`) symbols in the C `.so` are only the standard glibc
  start/ABI stubs; the Rust `cdylib` has only libc/`std` undefined symbols.

## ABI / layout parity

`struct tflac` measured from C (`sizeof` / `_Alignof` / `offsetof`):

```
size=28 align=4
blocksize=0 samplerate=4 channels=8 bitdepth=12
channel_mode=16 max_rice_value=17 min_partition_order=18
max_partition_order=19 partition_order=20 cur_blocksize=24
```

The Rust `#[repr(C)] struct tflac` reproduces this exactly (asserted by the
integration test `layout_matches_c`), including the 3 bytes of padding between
`partition_order` (20) and `cur_blocksize` (24).

## Completion gate

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name. Symbol diff is EMPTY (see `tests/symbol_parity.rs`
      / the `diff` run in Phase D).
- [x] No stubs / `unimplemented!()` — both functions are full translations.
