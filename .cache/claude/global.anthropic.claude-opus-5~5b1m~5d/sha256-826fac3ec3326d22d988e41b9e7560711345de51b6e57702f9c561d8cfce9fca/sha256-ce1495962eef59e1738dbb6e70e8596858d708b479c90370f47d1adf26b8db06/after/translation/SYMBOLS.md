# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-YuBSde.so   (name derives from parent dir via cmake_path)

cd translation && cargo build --release
# -> translation/target/release/libgotomach_lib.so
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libharvest-work-YuBSde.so`

| # | symbol        | type | source                          |
|---|---------------|------|---------------------------------|
| 1 | `process_value` | `T` | `c_src/src/lib.c:59`  |
| 2 | `double_value`  | `T` | `c_src/src/lib.c:65`  |
| 3 | `triple_value`  | `T` | `c_src/src/lib.c:71`  |
| 4 | `gotomach`      | `T` | `c_src/src/lib.c:106` |

### Not exported by the C `.so` (internal / `static`) — intentionally absent in Rust too

| symbol              | why not exported                                              |
|---------------------|---------------------------------------------------------------|
| `is_valid_state`    | `static bool` (`lib.c:48`) — internal linkage                  |
| `check_char_flag`   | `static bool` (`lib.c:55`) — internal linkage                  |
| `init_processor`    | `static ProcessorState*` (`lib.c:77`) — internal linkage       |
| `cleanup_processor` | `static void` (`lib.c:97`) — internal linkage                  |
| `MAKE_FUNC_NAME`    | macro, never expanded (`lib.c:30`) — generates **no** symbol   |
| `LOG_MSG`           | macro, expands to a `printf` call — generates **no** symbol    |
| `CREATE_LABEL`      | macro, never expanded (`lib.c:32`) — generates **no** symbol   |
| `operation_fn`      | typedef (`lib.c:34`) — generates **no** symbol                 |
| `ProcessorState`    | anonymous struct typedef (`lib.c:40`) — generates **no** symbol|

> The three function-like macros are the only macro machinery in the C source and
> **none of them is ever invoked to build an identifier**, so there are no
> macro-generated symbols to mirror. `LOG_MSG` is invoked, but it expands to a
> call expression, not a declaration.

## Rust `.so` exported (defined) dynamic symbols

`nm -D --defined-only translation/target/release/libgotomach_lib.so`

| # | symbol          | type | Rust item                                  |
|---|-----------------|------|--------------------------------------------|
| 1 | `double_value`  | `T`  | `#[unsafe(no_mangle)] pub unsafe extern "C" fn double_value`  |
| 2 | `gotomach`      | `T`  | `#[unsafe(no_mangle)] pub unsafe extern "C" fn gotomach`      |
| 3 | `process_value` | `T`  | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_value` |
| 4 | `triple_value`  | `T`  | `#[unsafe(no_mangle)] pub unsafe extern "C" fn triple_value`  |

## Symbol diff

```
comm -23 <(nm -D --defined-only <C.so>    | awk '{print $NF}' | sort) \
         <(nm -D --defined-only <RUST.so> | awk '{print $NF}' | sort)
```

**Result: EMPTY.** Every symbol the C `.so` exports is exported by the Rust
`.so` under the exact same name. No stubs were added; every export is a real
translation of the corresponding C function.

The Rust `.so` exports no *extra* `T` symbols beyond the four above (verified by
the reverse diff, which is also empty).

## Undefined (imported) symbols

The C `.so` imports `malloc`, `free`, `puts` (GCC lowers
`printf("literal\n")` → `puts("literal")`) plus the usual weak
`_ITM_*`/`__gmon_start__`/`__cxa_finalize` stubs.

The Rust `.so` imports the same `malloc`/`free`/`puts` (LLVM performs the
identical `printf` → `puts` lowering, which is why the log bytes match), plus
libc (`memcpy`, `memset`, `realloc`, `mmap64`, …), pthread TLS and
`_Unwind_*` (libgcc) symbols pulled in by the Rust standard library.

**0 missing / undefined non-libc symbols in the Rust `.so`** — every undefined
symbol resolves from `libc`/`libgcc_s`/`libpthread`, all of which are already
required by any Rust `cdylib`. Confirmed loadable end-to-end: the differential
tests `dlopen` the Rust `.so` and resolve all four exports at runtime.

### Completion checklist

- [x] `nm -D` shows 0 missing symbols in the Rust `.so` (C → Rust diff empty).
- [x] `nm -D` shows 0 undefined **non-libc** symbols in the Rust `.so`.
- [x] No symbol is a stub / `unimplemented!()`.
- [x] No C source file was left untranslated (`src/lib.c` is the only C file).
