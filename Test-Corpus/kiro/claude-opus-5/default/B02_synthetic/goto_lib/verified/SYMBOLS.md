# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

## Commands used

```sh
nm -D --defined-only c_src/build/libdriver.so            # C exports
nm -D --defined-only translation/target/release/libdriver.so  # Rust exports
nm -D --undefined-only translation/target/release/libdriver.so # Rust imports
```

## C `.so` defined dynamic symbols (ground truth)

The C library is built from exactly one translation unit (`c_src/src/goto.c`,
per `c_src/CMakeLists.txt` → `add_library(driver SHARED src/goto.c)`). There are
no macro-generated symbol names in the source. All three non-static functions
are exported:

| # | symbol | C signature | in Rust `.so`? |
|---|--------|-------------|----------------|
| 1 | `forward_goto_example` | `int forward_goto_example(int x)` | YES |
| 2 | `open_with_cleanup`    | `FILE* open_with_cleanup(const char *filename)` | YES |
| 3 | `driver`               | `int driver(int num, const char* filename)` | YES |

Only `driver` is declared in the public header `c_src/include/goto.h`; the other
two have external linkage and are therefore part of the ABI surface as well, so
they must be (and are) exported and differentially tested.

## Symbol diff

```
C defined   : driver forward_goto_example open_with_cleanup
Rust defined: driver forward_goto_example open_with_cleanup
missing in Rust: (none)
extra in Rust  : (none)
```

**Diff is EMPTY.** No module of the C source was skipped: `goto.c` is the only
C file and all three of its functions are translated in
`translation/src/goto.rs` with `#[unsafe(no_mangle)] extern "C"`.

## Undefined (imported) symbols in the Rust `.so`

Every undefined symbol resolves to glibc or the platform unwinder — there are
**0 missing/undefined non-libc symbols**:

* stdio used by the translation (mirrors the C's imports exactly):
  `printf`, `fprintf`, `fopen`, `fclose`, `fgets`, `ferror`, `stderr`, `fwrite`
* Rust runtime / libc support only:
  `_Unwind_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`,
  `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `lseek64`,
  `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
  `posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`,
  `realloc`, `realpath`, `stat64`, `strlen`, `syscall`, `write`, `writev`
* weak: `_ITM_*`, `__cxa_finalize`, `__cxa_thread_atexit_impl`, `__gmon_start__`,
  `gettid`, `statx`

The C `.so` imports the same stdio set (`fclose ferror fgets fopen fprintf
fwrite printf stderr`); `fwrite` appears in the C library because glibc expands
`printf("%s", ...)`/`fprintf` constant formats to `fwrite`, and in the Rust
library because `std` uses it.

## Build / feature configurations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so there is exactly ONE feature combination (the default, which is
also `--no-default-features`). `crate-type = ["cdylib"]` only — the project
builds **no binary executable** (`CMakeLists.txt` has no `add_executable`), so
the "compare C and Rust binary stdout" gate is not applicable; stdout is instead
compared by capturing fd 1 / fd 2 around every FFI call.
