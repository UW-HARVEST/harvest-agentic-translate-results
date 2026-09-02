# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-AWtRRE.so

cd translation && cargo build --release
# -> translation/target/release/libto_barycentric_lib.so
```

## C `.so` exported (defined) symbols

`nm -D --defined-only c_src/build/libharvest-work-AWtRRE.so`

```
00000000000011a3 T to_barycentric
```

`lm_v2`, `lm_sub2` and `lm_dot2` are `static` in `c_src/src/lib.c`, so they are
local symbols (`t`, not `T`) and are deliberately **not** part of the ABI. They
must NOT be exported from Rust either.

## Rust `.so` exported (defined) symbols

`nm -D --defined-only translation/target/release/libto_barycentric_lib.so`

```
0000000000011690 T to_barycentric
```

## Parity table

| # | C symbol | type | present in Rust `.so` | note |
|---|----------|------|------------------------|------|
| 1 | `to_barycentric` | `T` (global text) | yes | `#[unsafe(no_mangle)] pub extern "C" fn to_barycentric` |

### Symbol diff

`comm -3` over the sorted global-symbol name lists of both objects:

```
(empty)
```

* Symbols in C but missing from Rust: **0**
* No module of `c_src` was left untranslated: `c_src/src/lib.c` is the only
  translation unit in `c_src/CMakeLists.txt`, and all four of its functions
  (`lm_v2`, `lm_sub2`, `lm_dot2`, `to_barycentric`) have Rust counterparts.
* No stubs / `unimplemented!()` are used anywhere.

### Undefined (imported) symbols in the Rust `.so`

All undefined symbols are libc/compiler-runtime only (no missing
library-internal symbol). Verified with:

```sh
nm -D --undefined-only translation/target/release/libto_barycentric_lib.so
```

Result: 0 non-libc undefined symbols.

## ABI notes

`lm_vec2` is `struct { float x, y; }`. Under the SysV x86-64 ABI two
consecutive `float`s classify as a single SSE eightbyte, so each `lm_vec2`
argument is passed packed in one XMM register (`p1`→`xmm0`, `p2`→`xmm1`,
`p3`→`xmm2`, `p`→`xmm3`) and the return value comes back packed in `xmm0`.
`#[repr(C)]` makes rustc apply the identical classification, which is what
allows the differential test to declare one common
`extern "C" fn(Vec2, Vec2, Vec2, Vec2) -> Vec2` signature for both libraries.
