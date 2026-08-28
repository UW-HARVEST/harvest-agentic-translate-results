# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

## How the artifacts were produced

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-3CiLnC.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/librgb_to_hsv_lib.so
```

## C source inventory (completeness check)

The whole C library is three files; there is no untranslated module:

| C file | role | translated? |
|---|---|---|
| `c_src/CMakeLists.txt` | build script (`add_library(... SHARED src/lib.c)`) | n/a |
| `c_src/include/lib.h` | 1 line, 1 declaration: `void rgb_to_hsv(float *dest, const float *src);` | yes |
| `c_src/src/lib.c` | 38 lines, 1 function definition: `rgb_to_hsv` | yes — `translation/src/lib.rs` |

`add_library` lists exactly one translation unit (`src/lib.c`), so the `.so`
cannot contain code from any other C file. There is no macro-generated symbol
family (no `#define`-generated function names anywhere in the C source), no
`static` helper promoted to an export, and no global/`extern` data object.
Therefore the expected export set is the single symbol `rgb_to_hsv`.

## Defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | C `.so` | Rust `.so` | type | status |
|---|--------|---------|------------|------|--------|
| 1 | `rgb_to_hsv` | `T` (0x10f9) | `T` (0x11c40) | `void (float*, const float*)` | **PRESENT in both** |

C `.so` exports: 1 symbol.
Rust `.so` exports: 1 symbol.

### Symbol diff

```
$ comm -3 <(nm -D --defined-only C.so    | awk '{print $NF}' | sort) \
          <(nm -D --defined-only rust.so | awk '{print $NF}' | sort)
<empty>
```

**Missing from Rust: 0. Extra in Rust: 0. The diff is EMPTY.**

No `#[no_mangle]` wrapper had to be added and no C module had to be translated:
the single C translation unit is fully covered by `translation/src/lib.rs`.

## Undefined dynamic symbols (`nm -D --undefined-only`)

The C `.so` imports only weak CRT hooks:
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`.

The Rust `.so` imports the same weak CRT hooks plus the standard Rust runtime
dependencies, **all of which are libc / libgcc\_s (unwinder) symbols**:

* libc: `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`,
  `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`,
  `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`,
  `open64`, `posix_memalign`, `pthread_key_create`, `pthread_key_delete`,
  `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`,
  `statx`, `strlen`, `syscall`, `write`, `writev`,
  `__cxa_thread_atexit_impl`
* libgcc\_s unwinder: `_Unwind_Backtrace`, `_Unwind_GetDataRelBase`,
  `_Unwind_GetIP`, `_Unwind_GetIPInfo`, `_Unwind_GetLanguageSpecificData`,
  `_Unwind_GetRegionStart`, `_Unwind_GetTextRelBase`, `_Unwind_Resume`,
  `_Unwind_SetGR`, `_Unwind_SetIP`

**0 missing / undefined NON-libc symbols in the Rust `.so`.** These extra
imports come from `libstd`'s panic-runtime and allocator glue, not from the
translated code (`rgb_to_hsv` itself allocates nothing and cannot panic).

## Gate

- [x] `nm -D` shows 0 missing / undefined non-libc symbols in Rust.
- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name.
- [x] No stubbed / `unimplemented!()` symbol was introduced.
