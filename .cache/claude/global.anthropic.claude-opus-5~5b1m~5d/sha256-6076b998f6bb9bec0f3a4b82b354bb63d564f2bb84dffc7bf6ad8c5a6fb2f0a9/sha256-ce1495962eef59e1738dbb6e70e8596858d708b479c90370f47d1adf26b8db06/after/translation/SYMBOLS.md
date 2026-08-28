# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-BDAOsw.so` (name comes from the parent
  directory name via `CMakeLists.txt`; tests locate it by globbing `c_src/build/*.so`)
- Rust `.so`: `translation/target/{debug,release}/libmathop_lib.so`

## Commands used

```sh
nm -D --defined-only c_src/build/libharvest-work-BDAOsw.so        | awk '{print $NF}' | sort
nm -D --defined-only translation/target/release/libmathop_lib.so  | awk '{print $NF}' | sort
comm -23 <(c symbols) <(rust symbols)     # C symbols missing from Rust  -> MUST be empty
```

## Symbol table (12 public symbols in the C `.so`)

| # | symbol | C type / signature | in C `.so` | in Rust `.so` | notes |
|---|--------|--------------------|-----------|---------------|-------|
| 1 | `is_valid_operation` | `bool (char)` | T | T | returns `_Bool` (1 byte, 0/1) |
| 2 | `get_operation_priority` | `int (Operation)` | T | T | enum arg passed as plain `int` |
| 3 | `add_operation` | `int (int,int,int)` | T | T | 3rd arg unused |
| 4 | `multiply_operation` | `int (int,int,int)` | T | T | 3rd arg unused |
| 5 | `subtract_operation` | `int (int,int,int)` | T | T | 3rd arg unused |
| 6 | `divide_operation` | `int (int,int,int)` | T | T | guards `b == 0` |
| 7 | `modulo_operation` | `int (int,int,int)` | T | T | guards `b == 0` |
| 8 | `select_operation` | `MathOperation (Operation)` | T | T | returns function pointer |
| 9 | `get_computation_timestamp` | `time_t (void)` | T | T | `time_t` = `long` (8 bytes, LP64) |
| 10 | `allocate_results` | `ComputationResult* (int)` | T | T | wraps `calloc` |
| 11 | `perform_computation_with_history` | `int (int,int,Operation,ComputationResult**,int*)` | T | T | mutates caller state |
| 12 | `mathop` | `int (int,int,int,int)` | T | T | the only symbol in `include/lib.h` |

### Result

```
$ comm -23 <(C syms) <(Rust syms)
<empty>
```

**0 symbols missing from the Rust `.so`.** No symbol required a new
`#[no_mangle]` wrapper and no C source file was left untranslated: `src/lib.c`
is the only C translation unit and all 12 of its external definitions are
present in `src/lib.rs`. Nothing is stubbed or `unimplemented!()`.

The Rust `.so` exports exactly these 12 and no extra `T` symbols, so the
surface is identical in both directions.

## Undefined (imported) symbols

The Rust translation deliberately calls the platform C library for `calloc`,
`time` and `printf` so that allocation and stdio buffering behaviour are
byte-identical to the original.

| symbol | C `.so` | Rust `.so` | non-libc? |
|--------|---------|-----------|-----------|
| `calloc` | U | U | no (libc) |
| `time` | U | U | no (libc) |
| `printf` | U | U | no (libc) |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` | w | w | no (toolchain weak syms) |

The Rust `.so` additionally imports the usual Rust runtime/libc set
(`memcpy`, `__libc_start_main`-adjacent glue, unwinder hooks). There are
**0 undefined non-libc symbols**, i.e. nothing is left dangling.

## ABI cross-check (`ComputationResult`)

Verified with a compiled C probe (`sizeof`/`_Alignof`/`offsetof`) against the
`#[repr(C)]` Rust struct:

| property | C | Rust |
|----------|---|------|
| `sizeof` | 24 | 24 |
| `_Alignof` | 8 | 8 |
| `offsetof(value)` | 0 | 0 |
| `offsetof(timestamp)` | 8 | 8 |
| `offsetof(status)` | 16 | 16 |
| `sizeof(time_t)` | 8 | 8 |
| `sizeof(_Bool)` | 1 | 1 |
| `char` signed? | yes | yes (`c_char` = `i8`) |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one (`--no-default-features` and the default
build are the same compilation). Phase D's "every feature combination" therefore
collapses to a single combination, which is verified explicitly by
`ci/verify_all.sh`.
