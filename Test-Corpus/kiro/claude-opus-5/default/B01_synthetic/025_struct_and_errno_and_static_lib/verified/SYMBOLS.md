# SYMBOLS.md — dynamic symbol surface (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D c_src/build/libdriver.so
nm -D translation/target/release/libdriver.so
```

## C `.so` — defined (exported) symbols

`nm -D --defined-only c_src/build/libdriver.so`

| symbol | type | C source | present in Rust `.so`? |
|--------|------|----------|------------------------|
| `driver` | `T` (global text) | `c_src/src/driver.c:78` — `void driver(const char *in)`, declared in `include/driver.h` | YES (`T driver`) |
| `run`    | `T` (global text) | `c_src/src/driver.c:57` — `void run(int extra_bedrooms)`; **not** declared in `driver.h` but not `static` either, so it has external linkage and is part of the exported ABI | YES (`T run`) |

That is the complete set: the C translation unit defines exactly two symbols
with external linkage. Everything else in `driver.c` is `static` (internal
linkage) and therefore deliberately absent from both `.so` files:

`the_house`, `add_floor`, `add_bedrooms`, `add_floor_to_the_house`,
`print_the_house`, `parse_val`.

No macro-generated symbols exist in this translation unit (no symbol-defining
macros are used).

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libdriver.so      | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Result: **empty**. 0 symbols exported by the C `.so` are missing from the Rust
`.so`. No wrappers had to be added and no C module was left untranslated —
`c_src/src/driver.c` is the only C source file in the project
(`c_src/CMakeLists.txt` lists exactly `src/driver.c`) and it is fully
translated in `translation/src/lib.rs`.

The Rust `.so` exports no *extra* non-libc symbols either (`nm -D
--defined-only` yields exactly `driver` and `run`).

## Undefined (imported) symbols

Not required to match — these are the libc/runtime imports each toolchain
happens to need — but recorded for completeness.

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|-----------|------|
| `printf@GLIBC_2.2.5`   | U | U | both call the platform `printf` |
| `puts@GLIBC_2.2.5`     | U | U | in C this is gcc's `printf("...\n")` → `puts` optimisation for the two constant-string calls; the emitted bytes are identical either way |
| `strtol@GLIBC_2.2.5`   | U | U | Rust delegates parsing to the platform `strtol` |
| `__errno_location@GLIBC_2.2.5` | U | U | Rust reads/writes the same thread-local `errno` storage `strtol` writes |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*` | w | w | standard weak startup/teardown hooks |
| `_Unwind_*`, `malloc`, `free`, `memcpy`, `dl_iterate_phdr`, `pthread_key_*`, … | — | U | Rust `std`/panic-runtime imports pulled in by the `std` prelude; they are not part of the library's own API surface |

There are **0 missing/undefined non-libc symbols** in the Rust `.so`: every
undefined symbol it lists resolves against glibc / libgcc_s, which are already
loaded in any process that loads the library (verified with
`ldd translation/target/release/libdriver.so` reporting no "not found").

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
configuration is the default one (`--no-default-features` and the default build
produce the identical crate). The symbol table above therefore holds for every
feature combination; `scripts/check_features.sh` enumerates the feature set
mechanically and confirms the list is empty.
