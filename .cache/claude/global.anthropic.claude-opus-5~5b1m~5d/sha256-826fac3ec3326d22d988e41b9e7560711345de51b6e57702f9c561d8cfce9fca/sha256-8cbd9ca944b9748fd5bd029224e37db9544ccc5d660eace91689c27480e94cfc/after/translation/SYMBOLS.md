# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-kC62fB.so | grep -v ' [wWuU] '
nm -D --defined-only translation/target/release/libgotomach_lib.so
```

## C source surface

`c_src/include/lib.h` declares exactly one symbol:

```c
int gotomach(int a, int b, int c, int d);
```

`c_src/src/lib.c` additionally gives external linkage to the three
`operation_fn` implementations (they are declared at file scope *without*
`static`), so they are exported too. Everything else in `lib.c`
(`is_valid_state`, `check_char_flag`, `init_processor`, `cleanup_processor`) is
`static` and therefore NOT part of the dynamic symbol table.

The macros `MAKE_FUNC_NAME`, `LOG_MSG`, `CREATE_LABEL` are defined but only
`LOG_MSG` is used; none of them generate additional exported symbols.

## Symbol table

| # | C symbol | C type | Rust `.so` exports it? | Notes |
|---|----------|--------|------------------------|-------|
| 1 | `double_value`  | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn double_value` |
| 2 | `gotomach`      | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn gotomach` |
| 3 | `process_value` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_value` |
| 4 | `triple_value`  | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn triple_value` |

### Static (non-exported) C functions — correctly absent from both `.so`s

| C symbol | Rust counterpart | Exported? |
|----------|------------------|-----------|
| `is_valid_state`     | `fn is_valid_state`     | no (private, matches C `static`) |
| `check_char_flag`    | `fn check_char_flag`    | no (private, matches C `static`) |
| `init_processor`     | `fn init_processor`     | no (private, matches C `static`) |
| `cleanup_processor`  | `fn cleanup_processor`  | no (private, matches C `static`) |

## Symbol diff

```
$ diff <(c symbols) <(rust symbols)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`. 0 extra non-libc symbols.**
No module of the C source was left untranslated; `lib.c` is the only
translation unit in `CMakeLists.txt`.

## Undefined (imported) symbols

The Rust `.so` imports `printf` from libc, exactly as the C `.so` does, so the
`LOG_MSG` output shares the C runtime's stdout buffer. All remaining undefined
symbols on either side are libc/`libgcc`/Rust-runtime internals, not library
API.

## Build / feature configuration

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default one (`--no-default-features` is
equivalent to the default). `crate-type = ["cdylib"]`; the project builds **no
binary executable**, so the "compare C and Rust stdout from a driver binary"
gate is not applicable — stdout is instead compared by capturing the
`printf` output of each `.so` in the differential tests.
