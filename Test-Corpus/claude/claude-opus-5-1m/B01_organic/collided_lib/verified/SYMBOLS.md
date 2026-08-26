# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared libraries.

* C   `.so`: `c_src/build/libtranslated_rust.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `target/debug/libcollided_lib.so` (built via `cargo build --no-default-features`)

Commands used:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | sort
nm -D --defined-only target/debug/libcollided_lib.so   | sort
nm -D -u             target/debug/libcollided_lib.so   | sort
```

## Defined dynamic symbols

| # | C symbol | C source | exported by Rust `.so` | Rust item |
|---|----------|----------|------------------------|-----------|
| 1 | `c2V`              | `c_src/src/lib.c:18` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2V` |
| 2 | `c2Maxv`           | `c_src/src/lib.c:25` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2Maxv` |
| 3 | `c2Minv`           | `c_src/src/lib.c:30` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2Minv` |
| 4 | `c2Clampv`         | `c_src/src/lib.c:35` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2Clampv` |
| 5 | `c2Sub`            | `c_src/src/lib.c:39` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2Sub` |
| 6 | `c2Dot`            | `c_src/src/lib.c:45` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2Dot` |
| 7 | `c2CircletoCircle` | `c_src/src/lib.c:49` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2CircletoCircle` |
| 8 | `c2CircletoAABB`   | `c_src/src/lib.c:57` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2CircletoAABB` |
| 9 | `c2AABBtoAABB`     | `c_src/src/lib.c:65` | yes | `#[unsafe(no_mangle)] pub extern "C" fn c2AABBtoAABB` |
| 10 | `collided`        | `c_src/src/lib.c:73` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn collided` |

**Symbol diff (C defined − Rust defined): EMPTY.** No stubs were required; every
C function has a real Rust translation in `src/lib.rs`.

There is no C source file in `c_src/` that lacks a Rust translation:
`c_src/CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`),
and all 10 of its external functions are translated and exported.

## ABI notes verified by the tests

| type | C definition | size / passing (x86-64 SysV) |
|------|--------------|------------------------------|
| `c2v`      | `struct { float x, y; }`     | 8 B — packed in one SSE register |
| `c2Circle` | `struct { c2v p; float r; }` | 12 B — `xmm0` (`p.x`,`p.y`) + `xmm1` (`r`) |
| `c2AABB`   | `struct { c2v min, max; }`   | 16 B — `xmm0` (`min`) + `xmm1` (`max`) |
| `C2_TYPE`  | `enum { C2_TYPE_CIRCLE, C2_TYPE_AABB }` | 4 B int-sized enum, passed in a GPR; modelled as `c_int` in Rust |

`c2v` is returned in `xmm0` as two packed floats; the Rust `extern "C"` +
`#[repr(C)]` combination reproduces this, which the differential tests confirm
by comparing raw bit patterns of returned structs.

## Undefined symbols in the Rust `.so`

`nm -D -u target/debug/libcollided_lib.so` lists only C-runtime / unwinder
imports pulled in by `libstd` (`malloc`, `memcpy`, `pthread_key_create`,
`_Unwind_*`, `dl_iterate_phdr`, …) plus the usual weak
`_ITM_*Table` / `__gmon_start__` / `__cxa_finalize` entries that the C `.so`
also has.

**0 missing / undefined non-libc symbols.**
