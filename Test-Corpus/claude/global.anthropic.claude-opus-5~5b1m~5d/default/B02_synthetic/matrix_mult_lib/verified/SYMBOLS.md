# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## Exported (defined, dynamic) symbols

`nm -D --defined-only <so> | awk '{print $3}' | sort`

| # | symbol | C source | declared in header | C `.so` | Rust `.so` | Rust impl |
|---|--------|----------|--------------------|---------|------------|-----------|
| 1 | `allocate_matrix`               | `src/matrix.c:33`  | no (non-`static`, so still exported) | ✅ | ✅ | `src/matrix.rs` |
| 2 | `free_matrix`                   | `src/matrix.c:66`  | `include/matrix.h:33` | ✅ | ✅ | `src/matrix.rs` |
| 3 | `initialize_matrix_from_string` | `src/matrix.c:78`  | `include/matrix.h:32` | ✅ | ✅ | `src/matrix.rs` |
| 4 | `multiply_matrices`             | `src/matrix.c:118` | `include/matrix.h:34` | ✅ | ✅ | `src/matrix.rs` |
| 5 | `matrix_to_string`              | `src/matrix.c:137` | `include/matrix.h:35` | ✅ | ✅ | `src/matrix.rs` |
| 6 | `write_to_file`                 | `src/write.c:32`   | `include/write.h:26`  | ✅ | ✅ | `src/write.rs`  |
| 7 | `driver`                        | `src/driver.c:35`  | no (non-`static`)     | ✅ | ✅ | `src/driver.rs` |

There are no namespace/renaming macros, no `__attribute__((alias))`, no
macro-generated symbol families and no versioned symbols anywhere in `c_src/`,
so the linker names are exactly the source-level names.

### Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`, 0 extra. ✅

No stubs / `unimplemented!()` / `todo!()` exist in the crate; every export is a
full translation of the corresponding C function.

## Undefined (imported) symbols

The C `.so` imports only libc: `__errno_location atoi fclose fopen fprintf free
fwrite malloc perror snprintf stderr strcat strdup strerror strlen strtok_r`
(plus the weak CRT/ITM/gmon markers).

The Rust `.so` imports the same libc set plus the symbols the Rust runtime
itself needs (`libc`: `memcpy memmove memset calloc realloc posix_memalign
abort open64 read write writev close mmap64 munmap lseek64 stat64 fstat64
statx getcwd getenv realpath readlink syscall dl_iterate_phdr bcmp
pthread_key_*`, `__tls_get_addr`, `__cxa_thread_atexit_impl`, `gettid`; and
`libgcc_s`: the `_Unwind_*` family for the panic/backtrace machinery).

**0 undefined non-libc / non-runtime symbols.** `ldd` resolves fully against
`libc.so.6` + `libgcc_s.so.1`. ✅

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, therefore the complete set of feature combinations is:

| # | combination | cargo invocation |
|---|-------------|------------------|
| 1 | (default = empty) | `cargo test` |
| 2 | (explicitly no defaults, identical to #1) | `cargo test --no-default-features` |
| 3 | (all features = empty set) | `cargo test --all-features` |

All three resolve to the same, single code path. `scripts/check_features.sh`
enumerates them from `Cargo.toml` and runs the whole differential suite for
each, so the "every feature combination" gate is satisfied by construction.
