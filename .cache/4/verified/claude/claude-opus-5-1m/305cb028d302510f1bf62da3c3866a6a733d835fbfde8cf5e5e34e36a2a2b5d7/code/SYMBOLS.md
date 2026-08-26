# SYMBOLS.md — Symbol parity between the C `.so` and the Rust `.so`

## How this was produced

```sh
# C reference library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust translation
cargo build --lib
# -> target/debug/libpremultiply_lib.so

nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libpremultiply_lib.so
```

## C translation units

The C library is a single translation unit. `c_src/CMakeLists.txt` declares
exactly one source file:

```cmake
add_library(${project_name} SHARED
    src/lib.c)
```

`c_src/include/lib.h` declares exactly one function, `premultiply`, plus two
POD types (`cp_pixel_t`, `cp_image_t`). There is therefore **no C source that
was skipped by the translation** — `src/lib.rs` covers 100% of `src/lib.c`.

## Exported (defined) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `premultiply` | `T` @ `0x10f9` | `T` @ `0x121a0` | **MATCH** |

`nm -D --defined-only` output, verbatim:

```
=== C ===
00000000000010f9 T premultiply

=== Rust ===
00000000000121a0 T premultiply
```

**Symbol diff (C defined − Rust defined): EMPTY.**

There are no macro-generated symbols in the C source (the header contains no
function-like macros and no `#define`d entry points), so there is nothing else
to account for.

## Weak / compiler-supplied symbols

Both libraries additionally expose the standard toolchain weak symbols; these
are not part of the library's API surface:

| symbol | C `.so` | Rust `.so` |
|--------|---------|------------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` |
| `_ITM_registerTMCloneTable` | `w` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |

## Undefined (imported) symbols in the Rust `.so`

Requirement: *0 missing/undefined non-libc symbols*.

`objdump -p target/debug/libpremultiply_lib.so | grep NEEDED`:

```
NEEDED  libgcc_s.so.1
NEEDED  libc.so.6
NEEDED  ld-linux-x86-64.so.2
```

Every `U`/`w` entry in `nm -D target/debug/libpremultiply_lib.so` resolves to
one of those three: glibc (`malloc`, `memcpy`, `open64`, `pthread_*`,
`__errno_location`, `dl_iterate_phdr`, …) or libgcc's unwinder
(`_Unwind_*@GCC_*`). These come from the Rust standard library that is linked
into every `cdylib`; none of them is a C symbol that the translation failed to
provide.

**Undefined non-libc/non-toolchain symbols in the Rust `.so`: 0.**

## Verification gate

- [x] Every symbol defined by the C `.so` is defined by the Rust `.so` with the
      exact same name.
- [x] The symbol diff is empty.
- [x] No `unimplemented!()`, stub, or fake export exists in `src/lib.rs` — the
      single exported symbol is a full translation of the single C function.
- [x] 0 missing/undefined non-libc symbols in the Rust `.so`.

Automated as `tests/differential.rs::phase_d_symbol_parity`.
