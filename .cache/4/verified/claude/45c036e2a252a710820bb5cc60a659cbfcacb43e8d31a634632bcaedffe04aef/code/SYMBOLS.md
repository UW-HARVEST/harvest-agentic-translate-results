# SYMBOLS.md — C ↔ Rust exported-symbol parity

Generated mechanically from `nm -D --defined-only` on both shared objects.

```
C   .so: c_src/build/libtranslated_rust.so   (cmake, src/lib.c — the only C TU)
Rust.so: target/debug/libcharinbuf_lib.so    (crate-type = ["cdylib"])
```

The C header `include/lib.h` declares only `charinbuf`; the other nine symbols
are non-`static` definitions in `src/lib.c` and therefore part of the exported
ABI too. `static int counter` and the `operation_func` typedef are not symbols.
There are no namespace-renaming macros, so each `#[unsafe(no_mangle)]` name is
the final linker symbol.

## Parity table

| # | symbol | C .so | Rust .so | Rust definition site |
|---|--------|-------|----------|----------------------|
| 1 | `apply_operation` | T | T | `src/helpers.rs` |
| 2 | `charinbuf` | T | T | `src/charinbuf.rs` |
| 3 | `create_buffer` | T | T | `src/helpers.rs` |
| 4 | `decrement_counter` | T | T | `src/counter.rs` |
| 5 | `find_char_in_buffer` | T | T | `src/helpers.rs` |
| 6 | `increment_counter` | T | T | `src/counter.rs` |
| 7 | `is_string_empty` | T | T | `src/helpers.rs` |
| 8 | `multiply_counter` | T | T | `src/counter.rs` |
| 9 | `reset_counter` | T | T | `src/counter.rs` |
| 10 | `validate_uint16_range` | T | T | `src/helpers.rs` |

## Diff result

```
$ comm -23 <(nm -D --defined-only C.so | awk '{print $3}' | sort) \
           <(nm -D --defined-only RUST.so | awk '{print $3}' | sort)
(empty — 0 symbols missing from the Rust .so)
```

C exports 10 symbols, Rust exports the same 10. **0 missing.** No stubs were
used: every symbol is a real translation of the corresponding C function.

## Undefined (imported) symbols in the Rust .so

All undefined symbols are libc / libgcc-unwind / Rust-runtime imports — there is
no undefined *library* symbol, i.e. nothing in the C source was left untranslated
and then linked against. The C-stdlib routines the C TU itself used are called
straight through (`src/cstd.rs`) so behaviour stays byte-identical:

| import | used by |
|--------|---------|
| `printf` | `charinbuf` (all modes) — same `stdout` FILE, same buffering |
| `malloc` / `free` | `create_buffer`, `charinbuf` modes 2 & 4 |
| `memchr` | `find_char_in_buffer` |
| `strlen` / `strcpy` | `create_buffer`, `charinbuf` modes 2 & 4 |

Remaining imports (`_Unwind_*`, `__cxa_*`, `pthread_key_*`, `mmap64`, `statx`, …)
come from the Rust standard library that the `cdylib` links in, not from the
translated code.

## Feature combinations covered

`Cargo.toml` has no `[features]` section and `c_src` has no `#ifdef` /
`option()` / `target_compile_definitions`, so there is exactly ONE valid
configuration: the empty feature set. `--no-default-features` and the default
build are the same compilation, and both were verified to `cargo check` clean.
