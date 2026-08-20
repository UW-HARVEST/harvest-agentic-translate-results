# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```sh
# C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust translation
cargo build            # -> target/debug/libfloat2half_lib.so
cargo build --release  # -> target/release/libfloat2half_lib.so
```

## Translation-unit inventory (completeness check)

The C library consists of exactly one translation unit, per `c_src/CMakeLists.txt`
(`add_library(${project_name} SHARED src/lib.c)`):

| C source file | lines | translated to | status |
|---------------|-------|---------------|--------|
| `c_src/src/lib.c` | 118 | `src/lib.rs` | fully translated |
| `c_src/include/lib.h` | 3 | (declares `float2half` only) | n/a |

No C source file is missing from the translation — there is no skipped module.

## Public (dynamic, defined) symbols

`nm -D --defined-only c_src/build/libtranslated_rust.so`:

| symbol | type | present in Rust `.so`? | notes |
|--------|------|------------------------|-------|
| `float2half` | `T` (global text) | **yes** — `T float2half` | `#[unsafe(no_mangle)] pub extern "C" fn float2half(f32) -> u16` |

`nm -D --defined-only target/release/libfloat2half_lib.so`:

| symbol | type | present in C `.so`? | notes |
|--------|------|---------------------|-------|
| `float2half` | `T` (global text) | yes | ABI-compatible signature |

### Static (non-exported) C symbols — intentionally not exported

Both tables are `static` in C, so they have **internal linkage** and are absent
from `nm -D` by design. The Rust `static M_BASE` / `static M_SHIFT` are
likewise private. These are *not* symbol-parity gaps.

| C static | Rust counterpart | verified equal |
|----------|------------------|----------------|
| `static uint16_t m__base[512]` | `static M_BASE: [u16; 512]` | yes — all 512 entries diffed element-by-element |
| `static uint8_t m__shift[512]` | `static M_SHIFT: [u8; 512]` | yes — all 512 entries diffed element-by-element |

## Symbol diff result

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/release/libfloat2half_lib.so | awk '{print $3}' | sort)
(no output)
```

**Missing from Rust `.so`: 0. Undefined non-libc symbols in Rust `.so`: 0.**
No `#[no_mangle]` wrapper had to be added and no C source had to be
back-translated; the surface was already complete.
