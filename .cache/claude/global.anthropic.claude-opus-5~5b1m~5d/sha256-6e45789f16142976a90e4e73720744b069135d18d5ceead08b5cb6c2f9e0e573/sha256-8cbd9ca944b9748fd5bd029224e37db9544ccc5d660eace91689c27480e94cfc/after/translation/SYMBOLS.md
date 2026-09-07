# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   : `c_src/build/libdriver.so`   (built via CMake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

There is no `[features]` section in `translation/Cargo.toml`, therefore exactly
**one** feature combination exists (the empty/default one). `--no-default-features`
is equivalent to the default build.

The C project builds **only** a shared library (`add_library(driver SHARED ...)`).
It builds **no binary executable**, and the Rust crate is `cdylib`-only, so the
"compare the two drivers' stdout" clause of Phase B is satisfied by driving the
exported `driver()` symbol through the FFI boundary and capturing `stdout` with
`dup2` (see `tests/differential.rs::stdout_capture`).

## Exported (dynamic, defined) symbols

| # | symbol | source | C `.so` | Rust `.so` | notes |
|---|--------|--------|---------|------------|-------|
| 1 | `initialize_logger`     | `logger.c`       | T | T | `int initialize_logger()` |
| 2 | `log_info`              | `logger.c`       | T | T | `void log_info(const char*)` |
| 3 | `log_warning`           | `logger.c`       | T | T | `void log_warning(const char*)` |
| 4 | `log_error`             | `logger.c`       | T | T | `void log_error(const char*)` |
| 5 | `finalize_logger`       | `logger.c`       | T | T | `void finalize_logger()` |
| 6 | `create_task_manager`   | `task_manager.c` | T | T | `TaskManager* create_task_manager()` |
| 7 | `add_task`              | `task_manager.c` | T | T | `void add_task(TaskManager*, const char*, int)` |
| 8 | `print_tasks`           | `task_manager.c` | T | T | `void print_tasks(const TaskManager*)` |
| 9 | `destroy_task_manager`  | `task_manager.c` | T | T | `void destroy_task_manager(TaskManager*)` |
| 10 | `driver`               | `driver.c`       | T | T | `int driver(const char*)` |

**Symbol diff (C-exported minus Rust-exported): EMPTY.** 10 / 10 present, exact
names, no mangling, no macro-generated symbols in this library.

Reproduce with:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

## Undefined (imported) symbols

Every undefined symbol in the Rust `.so` resolves to libc / the unwinder;
there are **0 missing or undefined non-libc symbols**.

C imports: `atoi fclose fopen fprintf free fwrite getenv malloc printf puts
stderr strchr strlen strncpy` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust imports the identical libc set — `atoi fclose fopen fprintf free fwrite
getenv malloc printf puts stderr strchr strlen strncpy` — because
`src/cstd.rs` deliberately calls straight into glibc rather than re-implementing
stdio (this is what makes `%s`/`%d` formatting, the `(null)` rendering of a null
`%s`, and stream-buffering behaviour byte-identical).

Rust additionally imports the Rust-runtime/unwinder set, which is expected for
any `cdylib` and is *not* a translation gap:
`_Unwind_* __errno_location __tls_get_addr abort bcmp calloc close
dl_iterate_phdr fstat64 getcwd lseek64 memcpy memmove memset mmap64 munmap
open64 posix_memalign pthread_key_* read readlink realloc realpath stat64
syscall write writev` (+ weak `gettid`, `statx`, `__cxa_thread_atexit_impl`).

## Struct ABI parity (checked by the tests, not by `nm`)

`task_manager.h` exposes two structs by value in the public header, so their
layout is part of the ABI. The tests read the C-allocated `TaskManager` through
the Rust `#[repr(C)]` definition and vice versa.

| type | C layout | Rust layout (`#[repr(C)]`) |
|------|----------|----------------------------|
| `Task`        | `char description[256]; int priority;` → size 260, align 4 | `[c_char; 256]`, `c_int` → 260 / 4 |
| `TaskManager` | `Task *tasks; int max_tasks; int task_count;` → size 16, align 8 | `*mut Task`, `c_int`, `c_int` → 16 / 8 |

## Result

Verified for both profiles after every change:

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(no output — symbol diff is EMPTY)
```

* 10 / 10 exported symbols present in the Rust `.so`, exact names.
* 0 missing or undefined **non-libc** symbols.
* No stubs: every symbol is a real translation of the corresponding C function.
  `tests/phase_c.rs::e08_…` and `::e19_…` additionally assert mechanically that
  all 16 string literals of the C implementation — including those on
  unreachable error branches — are compiled into the Rust `.so`, so a dropped or
  faked branch would be caught here rather than passing as "symbol present".
* Struct ABI parity is exercised for real: the tests hand C-allocated
  `TaskManager*` pointers to the Rust library and vice versa, and read every
  `Task` back as raw bytes.
