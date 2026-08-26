# SYMBOLS.md — symbol parity between the C `.so` and the Rust `.so`

Artifacts compared:

* C:    `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  i.e. no `CMAKE_BUILD_TYPE` ⇒ gcc at `-O0`)
* Rust: `target/debug/libenvy_lib.so` (`crate-type = ["cdylib"]`)

Commands used:

```sh
nm -D --defined-only  c_src/build/libtranslated_rust.so
nm -D --defined-only  target/debug/libenvy_lib.so
nm -D --undefined-only <same>
```

## Defined (exported) symbols

The C translation unit has **no `static` functions and no global variables**, so
every function in `c_src/src/lib.c` is an exported dynamic symbol. All five are
exported by the Rust `.so` under the exact same name.

| # | C symbol | C type | in C `.so` | in Rust `.so` | Rust item |
|---|----------|--------|------------|---------------|-----------|
| 1 | `parse_env_numeric`    | `T` (global text) | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_env_numeric` |
| 2 | `init_config_from_env` | `T` | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_config_from_env` |
| 3 | `perform_operation`    | `T` | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_operation` |
| 4 | `apply_bit_operations` | `T` | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_bit_operations` |
| 5 | `envy`                 | `T` | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn envy` |

`c_src/include/lib.h` declares only `int envy(int,int,int,int);`, but the other
four functions have external linkage in C and are therefore part of the ABI
surface; the differential tests call **all five** through `dlopen`/`dlsym`.

**Symbol diff (C defined − Rust defined): EMPTY.** No symbol needed to be added
and no C source file was left untranslated (`c_src/src/lib.c` is the only
translation unit; `CMakeLists.txt` lists exactly that one source file).

## Undefined (imported) symbols

Only libc / language-runtime imports; nothing from the library itself is
missing.

| symbol | in C `.so` | in Rust `.so` | note |
|--------|-----------|---------------|------|
| `getenv`, `atoi`, `strchr`, `printf`, `fprintf`, `snprintf`, `stderr` | yes | yes | the Rust port deliberately calls the *same* libc entry points so formatting/buffering is byte-identical |
| `puts` | yes | no | gcc rewrites `printf("literal\n")` into `puts("literal")`; both write the same bytes to the same `stdout` `FILE`, so this is not an observable difference (verified byte-for-byte in Phase B) |
| `memcpy` | no | yes | gcc inlines the two `memcpy(&a,&b,sizeof(struct ProcessState))` calls at `-O0`; the Rust port calls libc `memcpy` — same effect |
| `_Unwind_*`, `__errno_location`, `malloc`/`free`/`calloc`/`realloc`, `mmap64`, `open64`, `read`, `write`, `writev`, `close`, `getcwd`, `readlink`, `realpath`, `stat64`, `fstat64`, `lseek64`, `dl_iterate_phdr`, `pthread_key_*`, `syscall`, `abort`, `memmove`, `memset`, `bcmp`, `strlen`, `__tls_get_addr`, `posix_memalign` | no | yes | pulled in by the Rust `std` runtime (panic machinery / backtrace support), not by translated code |
| `_ITM_*`, `__cxa_finalize`, `__gmon_start__` (weak) | yes | yes | toolchain boilerplate |
| `__cxa_thread_atexit_impl`, `gettid`, `statx` (weak) | no | yes | Rust `std` boilerplate |

**0 missing / undefined non-libc symbols in the Rust `.so`.** ✅

## Automated verification

`tests/phase_d_symbols.rs` (`cargo test --test phase_d_symbols -- --nocapture`)
re-derives this table on every run:

1. `dlopen(…, RTLD_NOW | RTLD_LOCAL)` on **both** shared objects — eager binding
   of every relocation, which fails if any symbol is unresolved. Both load.
2. `nm -D --defined-only --format=posix` on both objects; the set difference
   `C \ Rust` must be empty.
3. Every C symbol is resolved through `dlsym` in both objects (this is how all
   the Phase B/C differential calls are made, so the `#[no_mangle]` wrappers are
   exercised, not the Rust functions directly).
4. The C library's exported set must still be exactly the five names above, so
   the test fails if the C surface ever grows and this file goes stale.

Last run:

```
RTLD_NOW dlopen: both shared objects resolve every relocation
C defined symbols   : 5
Rust defined symbols: 5
C symbols           : {"apply_bit_operations", "envy", "init_config_from_env",
                       "parse_env_numeric", "perform_operation"}
missing in Rust     : []
symbol parity: OK (0 missing)
```

## Feature / configuration matrix

`Cargo.toml` declares **no `[features]`** (and `c_src/CMakeLists.txt` has no
`option()` / `add_definitions()` / `#ifdef`-driven configuration either), so the
complete set of valid feature combinations is the single empty one.
`./check_all_features.sh` enumerates the `[features]` section mechanically,
builds the powerset and, for every combination, runs

```
cargo check --no-default-features [--features …]
cargo check --no-default-features [--features …] --tests
cargo build --no-default-features [--features …]           # dev cdylib
cargo test  --no-default-features [--features …]           # phases B, C, D
cargo build --release --no-default-features [--features …] # release cdylib
cargo test  … with ENVY_RUST_SO=target/release/libenvy_lib.so
```

plus the plain default configuration. Result: `ALL FEATURE COMBINATIONS PASS`.
