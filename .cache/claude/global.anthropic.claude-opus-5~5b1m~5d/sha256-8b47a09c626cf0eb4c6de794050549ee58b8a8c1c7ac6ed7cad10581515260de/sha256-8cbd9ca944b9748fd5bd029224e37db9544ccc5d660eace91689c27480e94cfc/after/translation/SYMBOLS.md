# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-qma4tq.so`, compared against the Rust cdylib
`translation/target/release/libcleanup_lib.so`.

## C `.so` exported (global, defined) symbols

```
T cleanup
T cleanup_resources
T print_result
```

(`nm -D` additionally lists only compiler/linker-generated local or weak
entries — `_init`, `_fini`, `__bss_start`, `_edata`, `_end` — which are not part
of the library's API surface and are emitted automatically by the linker for any
shared object. They are not required of the Rust cdylib.)

## Parity table

| # | symbol | C signature (`include/lib.h` + `src/lib.c`) | in C `.so` | in Rust `.so` | status |
|---|--------|--------------------------------------------|-----------|---------------|--------|
| 1 | `cleanup`           | `int cleanup(int a, int b, int c, int d)`   | T | T | ✅ present |
| 2 | `print_result`      | `void print_result(const char *label, int result)` | T | T | ✅ present |
| 3 | `cleanup_resources` | `void cleanup_resources(char *dynamic_str)` | T | T | ✅ present |

Only `cleanup` is declared in the public header `include/lib.h`;
`print_result` and `cleanup_resources` are declared and defined with external
linkage in `src/lib.c`, so they are exported from the `.so` too and are part of
the ABI surface that must be matched.

## Missing symbols

None. The symbol diff (C exports minus Rust exports) is **empty**.

```
$ comm -23 c_syms.txt rust_syms.txt
(no output)
```

## Undefined (imported) symbols

The Rust `.so` imports only libc entry points that the C `.so` also imports —
`printf`, `snprintf`, `malloc`, `free`, `strncmp`, `strlen` (plus the Rust
runtime's `memcpy`/unwind-free `panic = "abort"` bits). There are **0 missing or
undefined non-libc symbols**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
build configuration is the default one. `cargo check --no-default-features`
and `cargo check` produce the same crate; there are no additional feature
combinations to sweep.
