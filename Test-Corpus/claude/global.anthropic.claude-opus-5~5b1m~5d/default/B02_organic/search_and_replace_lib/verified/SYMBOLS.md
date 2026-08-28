# SYMBOLS.md — Phase A: public symbol surface

Mechanically derived from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The whole C library is two files; there is no untranslated module.

| C file | contents | translated in |
|--------|----------|---------------|
| `c_src/include/lib.h` | 1 declaration: `char *searchAndReplace(const char*, const char*, const char*)` | `translation/src/lib.rs` |
| `c_src/src/lib.c` | 1 definition: `searchAndReplace` (90 lines) | `translation/src/lib.rs` (`pub unsafe extern "C" fn searchAndReplace`) |

`grep -nE '^[A-Za-z_].*\(' c_src/src/lib.c` finds exactly one function definition,
and there are no `#define`d/macro-generated symbol names, no `static` helpers, no
global data, and no `__attribute__((constructor))` in the C source. So the
expected exported surface is exactly one symbol.

## `nm -D --defined-only` (exported, dynamic)

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `searchAndReplace` | `T` (0x1159) | `T` (0x11ce0) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn`, C ABI, name not mangled |

Symbol diff (`comm -3` of the two sorted defined-symbol name lists): **empty**.

Neither `.so` exports any other global text/data symbol (both are built without
`--export-dynamic`; the Rust `cdylib` exports only `#[no_mangle]` items).

## `nm -D --undefined-only` (imports)

* C `.so`: `malloc`, `realloc`, `strdup`, `strlen`, `strncpy`, `strstr`
  (all `GLIBC_2.2.5`) + weak ITM/gmon/`__cxa_finalize` stubs.
* Rust `.so`: `malloc`, `realloc`, `strdup`, `strlen` (the three `extern "C"`
  imports declared in `src/lib.rs`, plus `strlen` used by `CStr::from_ptr`),
  plus the Rust standard-library's own libc/libgcc imports
  (`memcpy`, `memmove`, `memset`, `bcmp`, `free`, `calloc`, `posix_memalign`,
  `abort`, `mmap64`/`munmap`, `open64`/`read`/`write`/`close`, `dl_iterate_phdr`,
  `pthread_key_*`, `_Unwind_*`, `__errno_location`, `__tls_get_addr`, …).

`ldd` on the Rust `.so` resolves to `libgcc_s.so.1`, `libc.so.6`,
`ld-linux-x86-64.so.2` only.

**0 missing symbols, 0 undefined non-libc / non-libgcc symbols in the Rust `.so`.**
(`strstr`/`strncpy` are open-coded in Rust — `c_strstr`/`c_strncpy` — which is a
private implementation detail, not part of the exported surface; their observable
behaviour is what Phases B/C verify.)

## Feature configurations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
possible feature configurations are:

| combo | cargo flags |
|-------|-------------|
| default (empty) | `cargo test` |
| no-default-features (identical, empty) | `cargo test --no-default-features` |
| all-features (identical, empty) | `cargo test --all-features` |

All three resolve to the same code; `scripts/verify_all.sh` runs the suite under
each of them anyway, and additionally against the **debug** build of the Rust
`.so` (which enables `debug_assertions` and integer-overflow checks) via
`RUST_DRIVER_SO`.
