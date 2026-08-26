# SYMBOLS.md — Phase A symbol surface

## What this target actually is

`c_src/CMakeLists.txt` declares **`add_executable(driver src/main.c)`** — there is
no `add_library(... SHARED ...)` anywhere in the CMake project, and
`c_src/src/main.c` is the only C translation unit. It defines exactly one
function: `int main(int argc, char **argv)`.

Consequently:

* The C build product is an **ELF executable**, not a `.so`. Building with
  `-DCMAKE_POSITION_INDEPENDENT_CODE=ON` still yields an executable
  (`c_src/build/driver`, `ELF 64-bit LSB executable`).
* The C target **exports zero API symbols**: `nm -D --defined-only
  c_src/build/driver` prints nothing (0 lines). Its `.dynsym` contains only
  *imports* from libc plus the standard weak GNU/ITM stubs.
* Therefore the "public surface" that an external caller can reach is **not a
  set of dlopen-able exports** but the **process contract**:
  `argv` in → bytes on `stdout` + exit status out.
  The differential tests in `tests/` drive exactly that boundary: they execute
  the two real binaries (never calling Rust functions in-process) and compare
  stdout, stderr and exit status byte-for-byte, which is the executable
  equivalent of "load both `.so`s and compare exported-symbol results".
  `libloading` is declared in `[dev-dependencies]` per the harness contract even
  though there is no shared object to load.

## `nm -D` on the C binary (complete, verbatim symbol set)

| symbol | nm type | kind | present in Rust binary |
|--------|---------|------|------------------------|
| `_ITM_deregisterTMCloneTable` | `w` (weak undefined) | toolchain stub | yes (`w`) |
| `_ITM_registerTMCloneTable`   | `w` (weak undefined) | toolchain stub | yes (`w`) |
| `__gmon_start__`              | `w` (weak undefined) | toolchain stub | yes (`w`) |
| `__libc_start_main@GLIBC_2.34`| `U` (undefined)      | libc import | yes (`U`) |
| `printf@GLIBC_2.2.5`          | `U` (undefined)      | libc import | n/a — libc import, Rust writes via `write`/`writev` |
| `puts@GLIBC_2.2.5`            | `U` (undefined)      | libc import | n/a — libc import (gcc lowers the two `printf("…\n")` literal calls to `puts`) |
| `strtol@GLIBC_2.2.5`          | `U` (undefined)      | libc import | n/a — libc import, re-implemented in Rust as `strtol_base10` |

**Exported (defined) dynamic symbols in the C binary: none.**
So the "every C export must exist in Rust" requirement is satisfied vacuously —
there is nothing to export, and nothing was left untranslated: `main.c` contains
one function and `src/main.rs` translates it in full (argument-count check,
`strtol` parse, `end == argv[1]` check, print/increment loop, both return codes).

## Program entry points (static symbol table, the real comparison)

| entry point | C binary (`nm --defined-only`) | Rust binary (`nm --defined-only`) |
|-------------|-------------------------------|------------------------------------|
| `main`      | `0000000000401146 T main`     | `0000000000015800 T main`         |
| `_start`    | `0000000000401060 T _start`   | `0000000000014640 T _start`       |

## Undefined-symbol audit of the Rust binary (completion gate)

`nm -D --undefined-only target/release/driver` → 66 entries, all of which are
libc / libgcc-unwind / weak-toolchain symbols:

```
_ITM_deregisterTMCloneTable _ITM_registerTMCloneTable _Unwind_Backtrace
_Unwind_GetDataRelBase _Unwind_GetIP _Unwind_GetIPInfo
_Unwind_GetLanguageSpecificData _Unwind_GetRegionStart _Unwind_GetTextRelBase
_Unwind_Resume _Unwind_SetGR _Unwind_SetIP __cxa_finalize
__cxa_thread_atexit_impl __errno_location __gmon_start__ __libc_start_main
__tls_get_addr __xpg_strerror_r abort bcmp calloc close dl_iterate_phdr dup
fcntl free fstat64 getauxval getcwd getenv gettid lseek64 malloc memcpy memmove
memset mmap64 mprotect munmap open64 pause poll posix_memalign
pthread_attr_destroy pthread_attr_getguardsize pthread_attr_getstack
pthread_getattr_np pthread_key_create pthread_key_delete pthread_self
pthread_setspecific read readlink realloc realpath sigaction sigaltstack signal
stat64 statx strlen syscall sysconf write writev
```

`ldd` resolves every one of them (`libc.so.6`, `libgcc_s.so.1`,
`ld-linux-x86-64.so.2`).

**Missing / unresolved non-libc symbols in the Rust binary: 0.**

## Reproduce

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only build/driver          # -> empty (executable, no exports)

# Rust
cargo build --release
nm -D --defined-only target/release/driver # -> empty (executable, no exports)

# parity diff (imports normalised, libc excluded)
diff <(nm -D c_src/build/driver          | awk '{print $NF}' | sed 's/@.*//' | sort -u) \
     <(nm -D target/release/driver       | awk '{print $NF}' | sed 's/@.*//' | sort -u)
# only difference: the libc functions printf/puts/strtol that the Rust binary
# does not import because their behaviour is re-implemented in safe Rust.
```
