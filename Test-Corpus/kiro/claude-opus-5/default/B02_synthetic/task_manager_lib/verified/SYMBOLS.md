# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so        | awk '$2~/^[TBD]$/{print $3}' | sort
nm -D --defined-only translation/target/release/libdriver.so | awk '$2~/^[TBD]$/{print $3}' | sort
```

## C source files and their public symbols

| C file | symbols | Rust module |
|--------|---------|-------------|
| `c_src/src/logger.c` | `initialize_logger`, `log_info`, `log_warning`, `log_error`, `finalize_logger` | `src/logger.rs` |
| `c_src/src/task_manager.c` | `create_task_manager`, `add_task`, `print_tasks`, `destroy_task_manager` | `src/task_manager.rs` |
| `c_src/src/driver.c` | `driver` | `src/driver.rs` |

No macro-generated symbols exist in this library (no symbol-emitting macros in
the C sources). No `#ifdef`-gated public symbols. There is no `[features]`
section in `translation/Cargo.toml`, so exactly ONE feature combination exists
(the default / empty one).

## Symbol table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `add_task`             | T | T | OK |
| 2 | `create_task_manager`  | T | T | OK |
| 3 | `destroy_task_manager` | T | T | OK |
| 4 | `driver`               | T | T | OK |
| 5 | `finalize_logger`      | T | T | OK |
| 6 | `initialize_logger`    | T | T | OK |
| 7 | `log_error`            | T | T | OK |
| 8 | `log_info`             | T | T | OK |
| 9 | `log_warning`          | T | T | OK |
| 10 | `print_tasks`         | T | T | OK |

## Diff result

```
comm -23 c_syms rust_syms   # in C, missing from Rust
<empty>
comm -13 c_syms rust_syms   # extra in Rust
<empty>
```

**0 missing symbols. 0 extra symbols.**

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
libdl entries (`malloc`, `free`, `fopen`, `fclose`, `fprintf`, `printf`,
`getenv`, `atoi`, `strlen`, `strchr`, `strncpy`, `stderr`, `memcpy`, `abort`,
`_Unwind_*`, `__cxa_*`, `pthread_key_*`, …).

**0 missing/undefined non-libc symbols.**
