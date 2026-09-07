# SYMBOLS.md — Exported symbol parity

C `.so`:    `c_src/build/libharvest-work-oNgdf3.so`
Rust `.so`: `translation/target/release/libcomplexmode_lib.so`

Generated with:

```sh
nm -D --defined-only <so> | awk '{print $3}' | sort
```

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `check_permissions`    | yes | yes | `int check_permissions(int, int)` |
| 2 | `compare_operations`   | yes | yes | `int compare_operations(const char*, const char*)` |
| 3 | `complexmode`          | yes | yes | the only symbol declared in `include/lib.h` |
| 4 | `copy_and_sum`         | yes | yes | `int copy_and_sum(int*, int)` |
| 5 | `create_result_string` | yes | yes | `char* create_result_string(const char*, int)` — returns `malloc`'d buffer |
| 6 | `multiply_with_log`    | yes | yes | `int multiply_with_log(int, int, char**)` |
| 7 | `safe_add`             | yes | yes | `int safe_add(int, int, int)` |

`c_src/src/lib.c` declares **no** `static` functions and there are no
macro-generated symbol names, so the seven functions above are the complete
public ABI. There is no additional C source file in `c_src/src/`, so no module
was skipped in translation.

**Missing from Rust `.so`: NONE.**

```
$ comm -23 c_syms.txt rust_syms.txt
(empty)
```

## Undefined (imported) symbols

The C `.so` imports only libc: `malloc`, `free`, `memcpy`, `printf`, `puts`
(the compiler rewrote the constant-string `printf` calls to `puts`), `snprintf`,
`strcmp`, plus the usual CRT/ITM stubs.

The Rust `.so` imports the same libc functions (`malloc`, `free`, `memcpy`,
`printf`, `puts`, `snprintf`, `strcmp`) plus Rust-runtime/libunwind symbols
(`_Unwind_*`, `__cxa_thread_atexit_impl`, allocator and `std` support calls such
as `calloc`, `realloc`, `mmap64`, `write`, `dl_iterate_phdr`, …). All of these
resolve out of glibc / libgcc_s.

**Undefined non-libc / unresolvable symbols in the Rust `.so`: NONE.**

Because both `.so`s are loaded into one process that already links glibc, they
share the *same* `stdout` `FILE*` and the same `malloc` arena — so `printf`
byte output and heap ownership (`create_result_string`'s buffer may be `free`d
by the caller) are interchangeable between the two libraries.

## Cargo feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so there is exactly
one build configuration (default = no features). Verified:

```sh
$ grep -c '^\[features\]' Cargo.toml
0
```

`cargo test --no-default-features` is therefore identical to the default build,
and both are run in the test matrix script.

There is no `[[bin]]` target and no `src/main.rs`, so there is no driver binary
to diff on stdout; `crate-type = ["cdylib"]` only.
