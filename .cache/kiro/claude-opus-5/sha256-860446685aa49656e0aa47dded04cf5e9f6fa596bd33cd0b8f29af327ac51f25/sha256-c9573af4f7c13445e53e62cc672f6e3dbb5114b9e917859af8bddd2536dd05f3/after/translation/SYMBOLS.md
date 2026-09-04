# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Generated mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

Regenerate + diff with `./check_symbols.sh` (crate root).

## C `.so` dynamic symbol table (`nm -D --defined-only`)

The C library is built from `src/task_manager.c`, `src/logger.c`, `src/driver.c`
(see `c_src/CMakeLists.txt`). All ten public functions have default visibility,
so all ten appear in `.dynsym` as `T`. There are no macro-generated exports, no
exported data objects, and no versioned symbols in the C build.

| # | symbol | C declaration | source | in Rust `.so`? |
|---|--------|---------------|--------|----------------|
| 1 | `create_task_manager`  | `TaskManager *create_task_manager(void)`                                | `src/task_manager.c:32` | YES (`src/task_manager.rs`) |
| 2 | `add_task`             | `void add_task(TaskManager *, const char *, int)`                       | `src/task_manager.c:53` | YES (`src/task_manager.rs`) |
| 3 | `print_tasks`          | `void print_tasks(const TaskManager *)`                                 | `src/task_manager.c:67` | YES (`src/task_manager.rs`) |
| 4 | `destroy_task_manager` | `void destroy_task_manager(TaskManager *)`                              | `src/task_manager.c:74` | YES (`src/task_manager.rs`) |
| 5 | `initialize_logger`    | `int initialize_logger(void)`                                           | `src/logger.c:33`       | YES (`src/logger.rs`) |
| 6 | `log_info`             | `void log_info(const char *)`                                           | `src/logger.c:47`       | YES (`src/logger.rs`) |
| 7 | `log_warning`          | `void log_warning(const char *)`                                        | `src/logger.c:53`       | YES (`src/logger.rs`) |
| 8 | `log_error`            | `void log_error(const char *)`                                          | `src/logger.c:59`       | YES (`src/logger.rs`) |
| 9 | `finalize_logger`      | `void finalize_logger(void)`                                            | `src/logger.c:65`       | YES (`src/logger.rs`) |
| 10 | `driver`              | `int driver(const char *)`                                              | `src/driver.c:32`       | YES (`src/driver.rs`) |

`driver` is not declared in any public header; it is nevertheless exported by
the C `.so` and therefore part of the ABI surface that must be reproduced.

## Diff result

```
$ comm -23 c_names.txt rust_names.txt     # exported by C, missing from Rust
<empty>
$ comm -13 c_names.txt rust_names.txt     # exported by Rust, absent from C
<empty>
```

**Missing symbols: 0. Extra symbols: 0.** No `#[no_mangle]` wrapper had to be
added and no C module was found untranslated: `logger.c`, `task_manager.c` and
`driver.c` all have a corresponding Rust module (`src/logger.rs`,
`src/task_manager.rs`, `src/driver.rs`), plus `src/cbind.rs` holding the libc
declarations.

## Undefined (imported) symbols

The C `.so` imports only libc:

```
atoi fclose fopen fprintf free fwrite getenv malloc printf puts
stderr strchr strlen strncpy
(weak) _ITM_deregisterTMCloneTable _ITM_registerTMCloneTable __cxa_finalize __gmon_start__
```

`fwrite`/`puts` are GCC's own optimisation of `fprintf`/`printf` with constant
formats; they are not called from the source.

The Rust `.so` imports the same libc set — `atoi fclose fopen fprintf free
fwrite getenv malloc printf puts stderr strchr strlen strncpy` — because
`src/cbind.rs` deliberately binds the *same* C entry points rather than using
Rust equivalents (identical buffering, allocator identity and `atoi`/`getenv`
semantics). It additionally imports the Rust-runtime support set, all of which
is libc / libgcc-unwind and satisfied by the platform:

```
_Unwind_*            (libgcc_s — panic/unwind machinery)
__errno_location __tls_get_addr abort bcmp calloc close dl_iterate_phdr
fstat64 getcwd lseek64 memcpy memmove memset mmap64 munmap open64
posix_memalign pthread_key_create pthread_key_delete pthread_setspecific
read readlink realloc realpath stat64 syscall write writev
(weak) __cxa_thread_atexit_impl gettid statx
```

**0 missing / undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one configuration: the default (empty) feature set.
`cargo check --no-default-features` and `cargo check` are the same build.
`./check_features.sh` enumerates the feature list from `Cargo.toml` and checks
every combination; it finds the single empty combination.
