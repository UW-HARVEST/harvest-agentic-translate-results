# SYMBOLS.md — Phase A: exported-symbol surface

## What the C project actually is

`c_src/CMakeLists.txt` contains exactly three directives:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

There is **no** `add_library`, no `option()`, no `add_definitions`, no
`target_compile_definitions`, no `CMAKE_C_FLAGS` and no `if()`.  The project is
a **single-translation-unit executable** built from one 33-line file,
`c_src/src/main.c`, whose entire non-comment body is:

```c
#include <stdio.h>
#include <stdlib.h>

int main() {
    int x = 1, y = 1;
    scanf("%d %d", &x, &y);
    div_t result = div(x, y);
    printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
    return 0;
}
```

So the *public surface* of this program is **one function** (`main`) plus the
process contract around it: bytes on stdin → bytes on stdout → exit status /
terminating signal.

## Getting a `.so` out of it without touching `c_src/`

`main` is the only symbol the translation unit defines, so the shared library is
produced by renaming it on the compiler command line (a flag — **no file in
`c_src/` is modified**):

```sh
gcc -shared -fPIC -Dmain=driver_main c_src/src/main.c \
    -o target/<profile>/c_ref/libcdriver.so
```

(no `-O` flag, mirroring CMake's empty `CMAKE_C_FLAGS`.  `tests/differential.rs`
runs this itself, and rebuilds whenever `c_src/src/main.c` is newer.)

The Rust side mirrors this exactly: `src/lib.rs` is built as a `cdylib` and
exports the same name from the same body:

```rust
#[no_mangle]
pub extern "C" fn driver_main() -> c_int { ... }
```

`src/main.rs` is a three-line shim (`std::process::exit(driver::driver_main())`)
so that the executable and the `.so` run *the same code*, and the
`#[no_mangle]` export wrapper is what the differential tests actually call.

## `nm -D` diff

### Defined / exported

| symbol | C `libcdriver.so` | Rust `libdriver.so` |
|--------|-------------------|---------------------|
| `driver_main` | `T` | `T` |

```
$ nm -D --defined-only target/release/c_ref/libcdriver.so
0000000000001129 T driver_main

$ nm -D --defined-only target/release/libdriver.so
0000000000012be0 T driver_main
```

**Symbols exported by the C `.so` but missing from the Rust `.so`: 0.**
**Extra non-libc symbols exported by the Rust `.so`: 0.**

The diff is empty, so no module was skipped by the translation and no
`#[no_mangle]` wrapper is missing.  (There is nothing to stub: `main`/
`driver_main` is genuinely the *only* function the C source defines.  Every
other name in the C binary — `scanf`, `div`, `printf` — is an *imported* libc
symbol, not something the project defines.)

### Undefined / imported

Every undefined symbol on both sides resolves to libc / libgcc, so there are
**0 missing non-libc symbols**:

| side | undefined symbols |
|------|-------------------|
| C | `__isoc99_scanf`, `div`, `printf` + the standard weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` |
| Rust | `signal`, `read`, `write`, `writev`, `malloc`, `free`, `calloc`, `realloc`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `open64`, `close`, `lseek64`, `fstat64`, `stat64`, `statx`, `readlink`, `realpath`, `getcwd`, `getenv`, `mmap64`, `munmap`, `syscall`, `dl_iterate_phdr`, `__errno_location`, `__tls_get_addr`, `pthread_key_*`, `pthread_setspecific`, `gettid`, `_Unwind_*` (libgcc) + the same standard weak symbols |

The larger Rust import list is just the Rust standard library's own
dependencies (allocator, `std::io`, panic/backtrace machinery); it introduces no
unresolved project symbol.

Verified by row `SYM` in `tests/differential.rs` (`symbol_parity`), which shells
out to `nm -D` on both `.so`s and asserts the defined-symbol sets are identical
and that `ldd` reports no unresolvable import on either side.

Mutation-checked: deleting the `#[no_mangle]` attribute makes row `SYM` fail
with `symbols exported by the C .so but MISSING from the Rust .so:
["driver_main"]`, and rows `R26`/`R1`/`E1` fail with
`undefined symbol: driver_main` from `dlsym`.

## Also verified: the two `.so`s are interchangeable through `dlopen`

`tests/differential.rs` never calls a Rust function directly.  For every test
case it re-execs itself twice as a *runner* child which
`libloading::Library::new(<so>)` → `lib.get(b"driver_main")` → calls it, once
for `c_ref/libcdriver.so` and once for `target/<profile>/libdriver.so`.  The
identical harness code drives both libraries, so the `#[no_mangle]`/`extern "C"`
wrapper, the calling convention and the return value are all on test.
