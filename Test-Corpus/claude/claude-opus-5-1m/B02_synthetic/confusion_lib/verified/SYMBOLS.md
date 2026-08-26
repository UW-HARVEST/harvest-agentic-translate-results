# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libconfusion_lib.so
```

## Build configurations

`Cargo.toml` has **no `[features]` section**, therefore the set of valid feature
combinations is exactly one: the empty set.

| # | cargo invocation | features enabled |
|---|------------------|------------------|
| 1 | `cargo check`                        | (none — no `[features]` table exists) |
| 2 | `cargo check --no-default-features`  | (none — identical to #1) |

`c_src/CMakeLists.txt` has no `option()`, no `add_compile_definitions`, no
`target_compile_definitions` and no `#ifdef`-driven configuration in
`src/lib.c` / `include/lib.h`, so the C side likewise has exactly one
configuration.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `create_state`    | T | T | `ProcessState* create_state(int, int)` |
| 2 | `destroy_state`   | T | T | `void destroy_state(ProcessState*)` |
| 3 | `process_buffer`  | T | T | `int process_buffer(ProcessState*, char)` |
| 4 | `update_flags`    | T | T | `void update_flags(ProcessState*, int)` |
| 5 | `confuse_types`   | T | T | `int confuse_types(ProcessState*, int)` |
| 6 | `confusion`       | T | T | `int confusion(int, int, int, int)` — the only symbol declared in `include/lib.h` |

**Missing from Rust `.so`: none.** `nm -D` diff of the defined symbol name sets
is empty (see `tests/symbols.rs`, which recomputes this diff at test time and
fails if it is ever non-empty).

No macro-generated exports exist in the C source: `STRINGIFY`, `DEBUG_VAR` and
`LOG_OPERATION` expand only to `printf` calls inside function bodies, they do
not declare symbols.

## Undefined (imported) symbols

The C `.so` imports only libc: `free`, `malloc`, `memchr`, `printf`, `puts`,
`snprintf`, `strlen` (plus the weak `_ITM_*` / `__cxa_finalize` /
`__gmon_start__` toolchain symbols).

The Rust `.so` imports the same libc functions it actually uses (`malloc`,
`free`, `memchr`, `printf`, `snprintf`, `strlen`) plus the Rust runtime's own
libc/libgcc requirements (`_Unwind_*`, `memcpy`, `mmap64`, `pthread_key_*`, …).

**0 missing / unresolvable non-libc symbols in the Rust `.so`** — every
undefined symbol is provided by `libc`/`libgcc_s`, which is confirmed by the
fact that `dlopen()` of the Rust `.so` succeeds in every test.

Note that the C `.so` imports `puts` while the Rust `.so` does not: gcc
strength-reduces the argument-less `printf("…\n")` calls in
`create_state` / `process_buffer` into `puts`. This is a *tail* call to a
different libc entry point that produces byte-identical stdout, so it is an
implementation detail, not an ABI difference (the Rust side calls `printf`
directly with the same format string).
