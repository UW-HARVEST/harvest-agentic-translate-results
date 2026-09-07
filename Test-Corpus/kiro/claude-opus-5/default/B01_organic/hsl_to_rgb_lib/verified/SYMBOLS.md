# SYMBOLS.md — public-symbol parity

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libharvest-work-tFkHTu.so`
* Rust `.so`: `translation/target/release/libhsl_to_rgb_lib.so`

## C source inventory

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED src/lib.c)
```

`c_src/include/lib.h` declares exactly one function:

```c
void hsl_to_rgb(float *dest, const float *src);
```

There are no namespacing/renaming macros, no `#ifdef` feature gates, no
additional `.c` files, and no macro-generated symbol families. So the expected
defined-symbol surface is a single name: `hsl_to_rgb`.

## `nm -D` on the C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U fmodf@GLIBC_2.2.5
0000000000001109 T hsl_to_rgb
```

Defined (`T`) non-libc symbols: **`hsl_to_rgb`** (1).
`U fmodf` is an undefined import from libm/libc, not part of the surface.
The `w` entries are the standard glibc/gcc crt weak hooks, not library API.

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `hsl_to_rgb` | `T` (defined) | `T` (defined) | MATCH |

## Missing-symbol analysis

None. There is no C source file left untranslated: `src/lib.c` is the only
translation unit and its only function is `hsl_to_rgb`, which
`translation/src/lib.rs` exports via `#[unsafe(no_mangle)] pub unsafe extern "C" fn`.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/libharvest-work-tFkHTu.so \
        | awk '{print $3}' | grep -v '^_' | sort) \
     <(nm -D --defined-only translation/target/release/libhsl_to_rgb_lib.so \
        | awk '{print $3}' | grep -v '^_' | sort)
```

Result: empty diff (see `check_symbols.sh`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo check` are the same build. There is likewise no `#ifdef` in the C, so
there is no conditional symbol surface to cross-check.
