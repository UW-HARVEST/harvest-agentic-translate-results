# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-TuWem7.so

cd translation && cargo build --release
# -> translation/target/release/libfloat2half_lib.so
```

## Exported (defined, dynamic) symbols

`nm -D --defined-only <so>`

| # | symbol | in C `.so` | in Rust `.so` | note |
|---|--------|-----------|---------------|------|
| 1 | `float2half` | `T` | `T` | the only public symbol; declared in `c_src/include/lib.h` |

**Symbol diff (C exports not present in Rust): EMPTY.**
**Symbol diff (Rust exports not present in C): EMPTY.**

## Deliberately NOT exported

These are `static` (internal linkage) in `c_src/src/lib.c`, so they are absent
from the C `.so` dynamic symbol table and must likewise be absent from the Rust
`.so`. Both hold:

| C name | Rust name | linkage in C | present in either `.so`? |
|--------|-----------|--------------|--------------------------|
| `m__base[512]` (`static uint16_t`) | `M__BASE: [u16; 512]` | `static` (file-local) | no / no |
| `m__shift[512]` (`static uint8_t`) | `M__SHIFT: [u8; 512]` | `static` (file-local) | no / no |

Table contents were verified byte-for-byte equal between `c_src/src/lib.c` and
`translation/src/lib.rs` by mechanical extraction of every hex literal
(512/512 elements equal for both tables; `m__shift` value range 13..=24).

## Undefined symbols

C `.so` undefined: only weak ABI/loader stubs
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC`, `__gmon_start__`).

Rust `.so` undefined: the same weak stubs plus libc
(`malloc`, `free`, `memcpy`, `write`, `abort`, …) and the libgcc unwinder
(`_Unwind_*@GCC_*`). **All are libc / platform runtime symbols — 0 missing
non-libc symbols.** They come from the Rust standard library that is statically
linked into the `cdylib`, not from any untranslated C module.

## Missing-implementation audit (Phase A rule)

`c_src` consists of exactly one translation unit (`src/lib.c`, 118 lines) and
one header (`include/lib.h`, 3 lines). Both are fully translated in
`translation/src/lib.rs`. No module, file, or function of the C source is
untranslated, and no symbol is stubbed.

## Feature / configuration matrix

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default (empty) feature set. Verified by
inspection; there is nothing to iterate over for
`--no-default-features --features <combo>` beyond:

| # | feature combo | `cargo check` | tests |
|---|---------------|---------------|-------|
| 1 | *(default = no features)* | pass | Phase B + C pass |
| 2 | `--no-default-features` (identical to #1) | pass | Phase B + C pass |

`[lib] crate-type = ["cdylib"]` only — the crate builds **no binary
executable**, and `c_src/CMakeLists.txt` declares only `add_library(... SHARED)`
with no `add_executable`. The "compare C and Rust binary stdout" gate is
therefore not applicable to this project.
