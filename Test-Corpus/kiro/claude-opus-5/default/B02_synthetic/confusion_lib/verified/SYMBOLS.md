# SYMBOLS.md — Public symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`: `c_src/build/libharvest-work-0LOoHR.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Rust `.so`: `translation/target/release/libconfusion_lib.so` (`crate-type = ["cdylib"]`)

The C public header `c_src/include/lib.h` declares only `confusion`, but
`c_src/src/lib.c` defines six non-`static` functions, so all six have external
linkage and appear in the dynamic symbol table. All six must be exported by Rust.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `create_state`   | T | T | `ProcessState* create_state(int, int)` |
| 2 | `destroy_state`  | T | T | `void destroy_state(ProcessState*)` |
| 3 | `process_buffer` | T | T | `int process_buffer(ProcessState*, char)` |
| 4 | `update_flags`   | T | T | `void update_flags(ProcessState*, int)` |
| 5 | `confuse_types`  | T | T | `int confuse_types(ProcessState*, int)` |
| 6 | `confusion`      | T | T | `int confusion(int, int, int, int)` — the only header-declared entry point |

No macro-generated symbols exist: `STRINGIFY`, `DEBUG_VAR` and `LOG_OPERATION`
are expression/statement macros that expand to `printf` calls, not to
declarations, so they contribute no symbols.

## Diff result

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-0LOoHR.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libconfusion_lib.so | awk '{print $3}' | sort)
(empty)
```

**0 missing symbols. 0 extra symbols.**

## Undefined (imported) symbols

The Rust `.so` must not pull in non-libc undefined symbols. Both objects import
only libc:

| symbol | C | Rust | used for |
|--------|---|------|----------|
| `malloc`   | U | U | `create_state` allocations |
| `free`     | U | U | `destroy_state`, `create_state` failure path |
| `strlen`   | U | U | `process_buffer` |
| `memchr`   | U | (inlined in Rust) | `process_buffer` scan |
| `printf` / `puts` / `putchar` | U | U | all diagnostics (gcc rewrites some `printf` to `puts`/`putchar`) |
| `snprintf` / `__snprintf_chk` | U | U | `create_state` buffer formatting |

Rust additionally imports the usual `libgcc`/`pthread`/unwinder stubs that any
`cdylib` links; these are libc/toolchain symbols, not untranslated project code.

## Verification result

Re-checked by `verify_all.sh` for every configuration (see `CONFIGS.md` — the
crate declares no cargo features, so the default build is the only feature
configuration) and for both codegen profiles:

| configuration | symbol diff |
|---------------|-------------|
| default features, `--release` | empty (6/6) |
| default features, debug (`overflow-checks = on`) | empty (6/6) |

No symbol required a new `#[no_mangle]` wrapper and no C module was missing
from the translation: `c_src/src/lib.c` is the only C source file and all six of
its external functions were already implemented and exported.
