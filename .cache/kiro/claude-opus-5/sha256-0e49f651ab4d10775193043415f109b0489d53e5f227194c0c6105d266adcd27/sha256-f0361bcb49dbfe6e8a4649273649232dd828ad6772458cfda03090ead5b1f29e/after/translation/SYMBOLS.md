# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C:    `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## C source inventory (completeness check)

`find c_src -type f` (excluding `build/`):

| file | translated? | where |
|------|-------------|-------|
| `c_src/CMakeLists.txt` | n/a (build script) | `translation/Cargo.toml` |
| `c_src/include/driver.h` | yes — declares `void driver(int)` | `translation/src/lib.rs` |
| `c_src/src/driver.c` | yes — defines `driver` | `translation/src/lib.rs` |

No C translation unit is missing, so no Phase A "translate the skipped module"
work applies.

## Exported (defined, dynamic) symbols

`nm -D --defined-only <so>`, ignoring linker-synthesised local/data symbols.

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T` (0x1109) | `T` (0x116f0) | MATCH |

The C `.so` exports exactly one public symbol. The Rust `.so` exports it with
the identical name via `#[unsafe(no_mangle)] pub extern "C" fn driver`.

There are no macro-generated symbols in the C source (no function-defining
macros are used), so there is no hidden name to reproduce.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Output: *(empty)* — 0 symbols exported by C and missing from Rust.

## Undefined (imported) symbols

| symbol | C | Rust | note |
|--------|---|------|------|
| `printf@GLIBC_2.2.5` | U | U | Rust intentionally calls the same libc `printf`, so formatting and `stdout` buffering are byte-identical. |
| `_ITM_*TMCloneTable`, `__cxa_finalize`, `__gmon_start__` | w | w | standard weak glibc/ITM hooks |
| `_Unwind_*`, `__errno_location`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`, `__cxa_thread_atexit_impl`, `__tls_get_addr` | — | U/w | Rust `std` runtime (panic machinery, allocator, std::io). All resolve from libc/libgcc; **0 unresolved non-libc symbols**. |

Verified resolvable:

```
ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

## Gate

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the exact same name.
- [x] `nm -D` shows 0 missing symbols and 0 undefined non-libc symbols for the Rust `.so`.
- [x] No symbol is stubbed / `unimplemented!()`.
