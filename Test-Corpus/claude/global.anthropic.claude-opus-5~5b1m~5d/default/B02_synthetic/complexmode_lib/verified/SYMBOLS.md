# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-lvNhbn.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `translation/target/release/libcomplexmode_lib.so`
  (built with `cargo build --release`)

Regenerate / re-verify with:

```sh
./check_symbols.sh
```

## C source inventory

`c_src/src/lib.c` is the only translation unit. It declares **no** `static`
functions, so all 7 of its function definitions have external linkage and land
in the dynamic symbol table. `c_src/include/lib.h` only declares `complexmode`,
but the other six are still part of the `.so` ABI and are therefore part of the
verification surface.

| C source line | C definition | linkage |
|---|---|---|
| `lib.c:38` | `char* create_result_string(const char* op, int val)` | external |
| `lib.c:47` | `int check_permissions(int perms, int required)` | external |
| `lib.c:51` | `int safe_add(int a, int b, int perms)` | external |
| `lib.c:59` | `int multiply_with_log(int a, int b, char** log_msg)` | external |
| `lib.c:67` | `int copy_and_sum(int* src, int count)` | external |
| `lib.c:90` | `int compare_operations(const char* op1, const char* op2)` | external |
| `lib.c:99` | `int complexmode(int mode, int value1, int value2, int value3)` | external |

There are no macro-generated symbols, no namespace/prefix macros, no `#ifdef`
gated definitions, and no global data objects in the C translation unit.

## Exported-symbol parity table

`nm -D --defined-only` on each `.so`, function symbols only:

| # | symbol | in C `.so` | in Rust `.so` | Rust definition | status |
|---|--------|-----------|---------------|-----------------|--------|
| 1 | `create_result_string` | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C"` | OK |
| 2 | `check_permissions`    | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` | OK |
| 3 | `safe_add`             | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` | OK |
| 4 | `multiply_with_log`    | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C"` | OK |
| 5 | `copy_and_sum`         | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C"` | OK |
| 6 | `compare_operations`   | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C"` | OK |
| 7 | `complexmode`          | `T` | `T` | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` | OK |

**Missing from Rust: 0. Extra in Rust: 0.**

```
$ comm -23 c_syms.txt rust_syms.txt   # in C but not Rust
(empty)
$ comm -13 c_syms.txt rust_syms.txt   # in Rust but not C
(empty)
```

No symbol required an added wrapper and no C module was found untranslated:
`lib.c` is the whole library and all 7 functions have real Rust bodies (no
stubs, no `unimplemented!()`).

## Undefined (imported) symbols

The C `.so` imports only libc: `malloc`, `free`, `memcpy`, `printf`, `puts`,
`snprintf`, `strcmp` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
`puts` appears because GCC rewrites `printf("literal\n")` into
`puts("literal")`; the bytes written to `stdout` are identical.

The Rust `.so` imports the same libc entry points that the translation calls
directly (`malloc`, `free`, `memcpy`, `printf`, `snprintf`, `strcmp`) plus the
Rust standard-library runtime's own libc/libgcc imports (allocator, `_Unwind_*`
from `libgcc_s`, `dl_iterate_phdr`, `pthread_key_*`, file/`mmap` syscall
wrappers used by the panic-backtrace machinery, ...).

**0 undefined non-libc / non-compiler-runtime symbols in the Rust `.so`.**
`NEEDED` entries are `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`.

Because the translation forwards to the *same* libc `malloc`/`free`/`printf`/
`snprintf`/`strcmp` as the C build, heap ownership is interchangeable across the
FFI boundary (a caller may `free()` a pointer returned by either `.so`) and the
`stdout` byte stream is produced by identical formatting code.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration. `cargo test`, `cargo test --no-default-features` and
`cargo test --all-features` all resolve to the same unit. See
`check_features.sh`, which enumerates the feature list from `Cargo.toml` and
runs the full test suite for every combination it finds.
