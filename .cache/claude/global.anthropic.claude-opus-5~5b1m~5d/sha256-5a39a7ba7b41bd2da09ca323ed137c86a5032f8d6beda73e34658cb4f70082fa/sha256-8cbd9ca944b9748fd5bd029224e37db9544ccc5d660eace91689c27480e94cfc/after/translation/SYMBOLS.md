# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libharvest-work-dX6ZkX.so
cd translation && cargo build --release
#  -> translation/target/release/libfloat2half_lib.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libharvest-work-dX6ZkX.so`

| # | symbol | type | notes |
|---|--------|------|-------|
| 1 | `float2half` | `T` (global text) | the only public symbol; declared in `include/lib.h` as `uint16_t float2half(float flt)` |

Not exported by the C `.so` (and therefore not required of Rust):

| C object | linkage | why not exported |
|---|---|---|
| `m__base[512]` (`uint16_t`) | `static` | file-local, no dynamic symbol |
| `m__shift[512]` (`uint8_t`) | `static` | file-local, no dynamic symbol |

There are no macro-generated symbols in the C source (no function-defining
macros, no `#define`-generated entry points, no visibility attributes).

## Rust `.so` exported (defined) dynamic symbols

`nm -D --defined-only translation/target/release/libfloat2half_lib.so`

| # | symbol | type |
|---|--------|------|
| 1 | `float2half` | `T` (global text) |

## Parity diff

```
comm -3 <(nm -D --defined-only <c.so>  | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only <rs.so> | awk '{print $NF}' | sort -u)
```

Result: **empty**.

- Symbols in C but missing from Rust: **0**
- Symbols in Rust but not in C: **0** (Rust exports no extra public symbols)

No module of the C source was skipped: `c_src` consists of exactly
`include/lib.h` (3 lines, 1 declaration) and `src/lib.c` (118 lines, 2 static
tables + 1 function), all of which are present in `translation/src/lib.rs`.
The two lookup tables were compared element-by-element against the C source and
match at all 512 positions each (see `tests/differential.rs::tables_match_c_source`
for the in-test re-verification via observable behaviour).

## Undefined symbols in the Rust `.so`

`nm -D -u translation/target/release/libfloat2half_lib.so`

All undefined entries are libc / libgcc-unwind imports pulled in by the Rust
standard library, not unresolved project code:

- glibc: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `__errno_location`,
  `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`,
  `write`, `writev`, `lseek64`, `stat64`, `fstat64`, `statx`, `mmap64`,
  `munmap`, `dl_iterate_phdr`, `syscall`, `gettid`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__tls_get_addr`
- libgcc unwinder: `_Unwind_*`
- weak toolchain hooks: `_ITM_deregisterTMCloneTable`,
  `_ITM_registerTMCloneTable`, `__gmon_start__`

**Non-libc undefined symbols: 0.**

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the only
configuration is the default (empty) feature set. Verified with
`grep -c '^\[features\]' Cargo.toml` -> 0.

## Binary / driver

Neither project builds an executable: `c_src/CMakeLists.txt` contains only
`add_library(... SHARED src/lib.c)` (no `add_executable`), and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` with no
`[[bin]]` target and no `src/main.rs`. The "compare binary stdout" gate is
therefore not applicable.
