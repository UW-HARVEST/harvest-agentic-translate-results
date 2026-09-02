# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-BjcvSn.so
nm -D --defined-only translation/target/release/libcleanup_lib.so
nm -D --undefined-only <each .so>
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```cmake
add_library(${project_name} SHARED
    src/lib.c)
```

`c_src/src/lib.c` is the only C source file in the tree (`find c_src -name '*.c'`
returns exactly that one path). It defines exactly three functions with external
linkage:

| C definition | line |
|---|---|
| `int cleanup(int a, int b, int c, int d)` | `src/lib.c:35` |
| `void print_result(const char *label, int result)` | `src/lib.c:79` |
| `void cleanup_resources(char *dynamic_str)` | `src/lib.c:83` |

There is **no untranslated module**: the Rust crate (`translation/src/lib.rs`)
covers the whole of `lib.c`. `STRINGIZE` / `TO_STRING` are object-like/function-like
macros and generate no symbols.

Note `c_src/include/lib.h` declares only `cleanup`; `print_result` and
`cleanup_resources` are declared in `lib.c` itself but are **not** `static`, so
they have external linkage and are part of the exported ABI. They are therefore
in scope for verification even though the public header omits them.

## Exported (defined, dynamic) symbol parity

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `cleanup`           | `T` | `T` | present in both |
| 2 | `cleanup_resources` | `T` | `T` | present in both |
| 3 | `print_result`      | `T` | `T` | present in both |

Missing from Rust: **none**. No `#[no_mangle]` wrapper had to be added, and no
C module had to be back-translated. Nothing is stubbed or `unimplemented!()`.

Verification command (must print nothing):

```sh
comm -23 \
  <(nm -D --defined-only c_src/build/libharvest-work-BjcvSn.so \
      | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only translation/target/release/libcleanup_lib.so \
      | awk '{print $3}' | sort -u)
```

The Rust `.so` additionally exports Rust-runtime symbols (`rust_eh_personality`,
`_ZN*` std mangled items, `__rust_*` allocator shims). Extra exports are allowed;
the gate is that no C export is *absent*.

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | note |
|---|---|---|---|
| `malloc`, `free`, `printf`, `snprintf`, `strlen`, `strncmp`, `puts` | U | U | libc; the Rust crate deliberately binds the *same* libc entry points so formatting, stdio buffering and heap ownership are identical |
| `_ITM_*`, `__cxa_finalize`, `__gmon_start__` | w | w | weak toolchain symbols |
| `_Unwind_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `syscall`, `write`, `writev`, `__cxa_thread_atexit_impl` | — | U/w | libc + libgcc, pulled in by the Rust std runtime |

`puts` appears undefined in the **C** `.so` because gcc rewrites
`printf("...\n")` / `printf("%s\n", p)` into `puts(...)`. That is a pure
optimisation: `puts(s)` emits `s` followed by `'\n'`, exactly what the two
`printf` forms emit, so it is not an observable difference. The Rust build keeps
the literal `printf` calls; byte-identical stdout is asserted in
`tests/differential.rs::stdout_*`.

**Non-libc undefined symbols in the Rust `.so`: 0.** Every `U`/`w` entry above
resolves out of `libc.so.6`, `libgcc_s.so.1` or the loader.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (empty) feature set. `scripts/check_features.sh`
enumerates features straight out of `Cargo.toml` and loops over them, so the
loop stays correct if features are ever added.

## Result

Symbol diff is **empty**, verified both by shell and as a test
(`tests/phase_d_symbols.rs` → 3 passed):

* `every_c_export_is_exported_by_rust` — set difference of `nm -D --defined-only`
  must be empty.
* `rust_so_has_no_unresolved_non_libc_symbols` — none of the three exports may
  appear in `nm -D --undefined-only` (which would mean it is imported, not
  implemented), and the `.so` must `dlopen` cleanly, which is itself proof that
  every remaining undefined symbol resolves from the loaded process image.
* `all_c_exports_resolve_via_dlsym_on_rust_so` — each name is fetched with
  `dlsym` on the Rust `.so`, so the tests only ever reach the Rust code through
  its `#[no_mangle] extern "C"` wrappers, never by direct Rust linkage.
