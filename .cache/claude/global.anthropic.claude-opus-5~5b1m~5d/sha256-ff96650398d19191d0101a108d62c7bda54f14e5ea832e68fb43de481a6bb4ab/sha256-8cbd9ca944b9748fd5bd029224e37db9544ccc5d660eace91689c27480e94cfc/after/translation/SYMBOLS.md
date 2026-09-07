# SYMBOLS.md — Phase A symbol surface

C shared object: `c_src/build/libharvest-work-Xoijma.so`
Rust shared object: `translation/target/release/libupdate_frame_header_lib.so`

## `nm -D --defined-only` on the C `.so`

```
00000000000010f9 T update_frame_header
```

## `nm -D --defined-only` on the Rust `.so`

```
00000000000116b0 T update_frame_header
```

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | notes |
|---|--------|------|-----------|--------------|-------|
| 1 | `update_frame_header` | `T` (global text) | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn update_frame_header(*mut tflac)` |

**Symbol diff (C − Rust): EMPTY.** 0 missing symbols, 0 undefined non-libc
symbols in the Rust `.so`.

## Non-exported C entities (deliberately not symbols)

| C entity | file | linkage | Rust counterpart |
|----------|------|---------|------------------|
| `struct tflac` / `typedef tflac` | `include/lib.h` | type only | `#[repr(C)] pub struct tflac` |
| `typedef uint8_t tflac_u8` | `include/lib.h` | type only | `pub type tflac_u8 = u8` |
| `typedef uint32_t tflac_u32` | `include/lib.h` | type only | `pub type tflac_u32 = u32` |
| `enum TFLAC_CHANNEL_MODE` | `src/lib.c` | type only, file-local | `mod channel_mode` consts |

## ABI layout verification

Compiled probe against `c_src/include/lib.h` (gcc, x86-64):

```
size=24 align=4 off=0 4 8 12 16 20
```

Rust `#[repr(C)] struct tflac { u32, u32, u32, u8, u32, u32 }` yields
`size_of == 24`, `align_of == 4`, offsets `0, 4, 8, 12, 16, 20` — identical.
The 3 padding bytes after `channel_mode` (offsets 13..16) are untouched by both
implementations; the differential tests assert the full 24-byte image matches,
including those bytes.

## Build / feature configurations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so there is exactly ONE feature combination: the default (empty)
one. `cargo check --no-default-features` and `cargo check` are the same build.
`crate-type = ["cdylib"]` only — no binary target, so there is no driver
executable to compare stdout for.
