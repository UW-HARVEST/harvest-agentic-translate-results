# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust `.so`: `target/release/libflac_validate_lib.so` (`crate-type = ["cdylib"]`)

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```
add_library(${project_name} SHARED src/lib.c)
```

`c_src/` contains only `include/lib.h` and `src/lib.c`. There is **no untranslated
C module** — `src/lib.rs` covers the whole library, so no Phase-A "translate the
missing source" work is required.

## Symbol table

| # | symbol | in C `.so` | in Rust `.so` | kind | notes |
|---|--------|-----------|--------------|------|-------|
| 1 | `flac_validate` | `T` | `T` | global text | declared in `include/lib.h` |
| 2 | `tflac_size_memory` | `T` | `T` | global text | defined in `src/lib.c`, **not** declared in the public header, but non-`static` so it is exported |

`nm -D --defined-only` raw output:

```
# C
00000000000010f9 T tflac_size_memory
000000000000111a T flac_validate

# Rust
0000000000011c70 T flac_validate
0000000000011d40 T tflac_size_memory
```

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so    | awk '{print $NF}' | sort) \
       <(nm -D --defined-only target/release/libflac_validate_lib.so | awk '{print $NF}' | sort)
# (empty)
```

**Missing from Rust: 0.  Extra in Rust: 0.**

## Undefined symbols

The C `.so` has only the four standard weak CRT symbols
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize`, `__gmon_start__`).

The Rust `.so` additionally imports libc / libgcc-unwind symbols pulled in by
`std` (`malloc`, `memcpy`, `_Unwind_*`, `dl_iterate_phdr`, …). Every one of them
is a **libc / language-runtime** symbol, not a library symbol that failed to be
translated.

```
$ nm -D --undefined-only target/release/libflac_validate_lib.so \
    | awk '{print $NF}' | sed 's/@.*//' \
    | grep -vE '^(_Unwind_|__errno_location|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev|_ITM_|__cxa_|__gmon_start__|gettid|statx)$'
# (empty)
```

**0 missing / undefined non-libc symbols in Rust.** ✅

## ABI: `struct tflac` layout

Verified by compiling a probe program against `c_src/include/lib.h` with the
same compiler that builds the `.so` (GCC 11.5.0, x86_64-linux-gnu):

| field | C offset | Rust offset (`repr(C)`) |
|-------|----------|-------------------------|
| `blocksize` (`u32`) | 0 | 0 |
| `samplerate` (`u32`) | 4 | 4 |
| `channels` (`u32`) | 8 | 8 |
| `bitdepth` (`u32`) | 12 | 12 |
| `channel_mode` (`u8`) | 16 | 16 |
| `max_rice_value` (`u8`) | 17 | 17 |
| `min_partition_order` (`u8`) | 18 | 18 |
| `max_partition_order` (`u8`) | 19 | 19 |
| `partition_order` (`u8`) | 20 | 20 |
| *padding* | 21..23 | 21..23 |
| `cur_blocksize` (`u32`) | 24 | 24 |
| **`sizeof` / `alignof`** | **28 / 4** | **28 / 4** |

`src/lib.rs` asserts this at compile time via `const _: () = { assert!(...) }`.

## Feature combinations

`Cargo.toml` has **no `[features]` section** and `src/` contains **no
`#[cfg(feature = ...)]`**, so the complete set of valid build configurations is:

| # | command | result |
|---|---------|--------|
| 1 | `cargo check` (default = no features) | ✅ clean |
| 2 | `cargo check --no-default-features` | ✅ clean |

`c_src/CMakeLists.txt` defines no `option()`, no `target_compile_definitions`,
and `src/lib.c` contains no `#ifdef`, so the C side likewise has a single
build configuration.
