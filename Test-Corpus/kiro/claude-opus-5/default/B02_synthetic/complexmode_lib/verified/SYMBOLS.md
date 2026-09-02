# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-tzqyqR.so` (name follows the parent
  directory name, so tests glob `c_src/build/lib*.so`)
* Rust `.so`: `translation/target/release/libcomplexmode_lib.so`

Commands used:

```sh
nm -D --defined-only c_src/build/lib*.so
nm -D --defined-only translation/target/release/libcomplexmode_lib.so
```

## Exported (defined) symbols

| # | C symbol (`nm -D` on C `.so`) | C type | present in Rust `.so` | Rust definition |
|---|-------------------------------|--------|-----------------------|-----------------|
| 1 | `create_result_string` | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn create_result_string` |
| 2 | `check_permissions`    | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn check_permissions` |
| 3 | `safe_add`             | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub extern "C" fn safe_add` |
| 4 | `multiply_with_log`    | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn multiply_with_log` |
| 5 | `copy_and_sum`         | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn copy_and_sum` |
| 6 | `compare_operations`   | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn compare_operations` |
| 7 | `complexmode`          | `T` | YES (`T`) | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn complexmode` |

Only `complexmode` is declared in the public header `c_src/include/lib.h`; the
other six have external linkage in `c_src/src/lib.c` (no `static`), so they are
part of the `.so`'s ABI surface and are tested as public entry points.

**Symbol diff (C defined − Rust defined): EMPTY.** No macro-generated symbols
exist in this library (no symbol-emitting macros in the C source).

There are no `D`/`B`/`R` (data) symbols in the C `.so`: the C file declares no
non-`static` globals. Nothing to mirror.

## Undefined (imported) symbols

The C `.so` imports, from libc:
`free`, `malloc`, `memcpy`, `printf`, `puts`, `snprintf`, `strcmp`
(plus the weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` glibc boilerplate).

Note `strcpy` does **not** appear: GCC expands `strcpy` with a literal source
into inline stores. Likewise several `printf("...\n")` calls are rewritten by
GCC into `puts`. Both are byte-for-byte equivalent on stdout, so the Rust
translation's use of `printf`/`strcpy` from libc is ABI- and output-compatible.

The Rust `.so` imports the same libc functions plus the Rust standard
library's own libc/unwind dependencies (`_Unwind_*`, `pthread_key_*`, `mmap64`,
`memset`, `calloc`, `realloc`, …). All are ordinary libc/libgcc symbols
resolved by the loader.

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

Verified with `ldd -r` (no unresolved symbols) — see
`tests/differential.rs::symbol_parity` which re-checks the diff at test time.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. Phase D's "every feature combination"
therefore reduces to a single combo; `cargo test --no-default-features` is
still exercised to prove it.
