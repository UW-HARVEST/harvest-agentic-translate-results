# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How the two shared objects are produced

The C project (`c_src/CMakeLists.txt`) compiles the single translation unit
`c_src/src/main.c` into an **executable**.  The same translation unit compiled
as a shared library (PIC, same `-fno-strict-aliasing` flag the CMake project
uses) is what the differential tests dlopen:

```
gcc -shared -fPIC -fno-strict-aliasing -O0 c_src/src/main.c -o build_c/libcdriver.so
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build c_src/build   # -> c_src/build/driver (exe)
cargo build                                                                                        # -> target/debug/libdriver.so (cdylib) + target/debug/driver (exe)
```

## `nm -D` on the C shared object

```
$ nm -D build_c/libcdriver.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U __isoc99_scanf@GLIBC_2.7
0000000000001193 T driver
00000000000011b8 T main
                 U printf@GLIBC_2.2.5
                 U putchar@GLIBC_2.2.5
```

Defined (`T`) symbols only — i.e. the export surface an external caller sees:

| # | C symbol | C declaration | linkage in `main.c` | exported by Rust `libdriver.so`? |
|---|----------|---------------|---------------------|----------------------------------|
| 1 | `driver` | `void driver(int x)` | external (`T`) | **yes** — `#[no_mangle] pub extern "C" fn driver(x: c_int)` in `src/lib.rs` |
| 2 | `main`   | `int main()`         | external (`T`) | **yes** — `#[no_mangle] pub extern "C" fn main() -> c_int` in `src/lib.rs` |

`print_hex` is declared `static void print_hex(unsigned char *p, int len)` in
`main.c`, so it is **not** part of the dynamic export surface (`nm` on the C
executable shows it as a local `t` symbol).  It is therefore intentionally not
exported from the Rust cdylib either; it is translated as the private helper
`logic::print_hex`, and it is covered indirectly through `driver` (the only
caller in the C source, always with `len == sizeof(int) == 4`).

## `nm -D --defined-only` on the Rust shared object

```
$ nm -D --defined-only target/debug/libdriver.so
0000000000017340 T driver
0000000000017360 T main
```

## Symbol diff

```
$ comm -23 <(nm -D --defined-only build_c/libcdriver.so | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort -u)
<empty>
```

**0 symbols missing from the Rust `.so`.**  (`symbol_parity.sh` re-runs this diff
and fails if it is ever non-empty.)

## Undefined symbols in the Rust `.so`

All `U`/`w` entries of `target/debug/libdriver.so` are libc / libgcc-unwind
imports pulled in by the Rust standard library (`malloc`, `free`, `read`,
`write`, `writev`, `lseek64`, `open64`, `close`, `memcpy`, `memmove`, `memset`,
`bcmp`, `strlen`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`, `stat64`,
`fstat64`, `statx`, `mmap64`, `munmap`, `posix_memalign`, `calloc`, `realloc`,
`syscall`, `dl_iterate_phdr`, `pthread_key_*`, `pthread_setspecific`,
`__errno_location`, `__tls_get_addr`, `__cxa_*`, `gettid`, `_Unwind_*`,
`_ITM_*`, `__gmon_start__`).

**0 missing/undefined non-libc symbols.**

## Nothing was left untranslated

`c_src/` contains exactly one source file (`src/main.c`, 45 lines) with exactly
three functions (`print_hex`, `driver`, `main`).  All three are translated in
`src/logic.rs`; the two externally visible ones are re-exported with the C ABI
from `src/lib.rs`.  No symbol is stubbed, faked, or `unimplemented!()`.
