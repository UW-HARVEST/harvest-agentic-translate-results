# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory

The whole C library is a single translation unit: `c_src/src/driver.c`
(66 lines), with the public header `c_src/include/driver.h` declaring only
`void driver(void)`. The other four functions have external linkage (no
`static`) and are therefore exported from the `.so` as well; they are part of
the ABI surface and must be reproduced.

| C source file | translated to | status |
|---|---|---|
| `c_src/src/driver.c` | `translation/src/lib.rs` | fully translated (all 5 functions) |
| `c_src/include/driver.h` | (declarations only, no code) | n/a |

No C module was skipped; there is nothing left to translate.

## Exported (defined, `T`) symbol table

| # | symbol | C signature | in C `.so` | in Rust `.so` | notes |
|---|--------|-------------|-----------|---------------|-------|
| 1 | `printLine`    | `void printLine(const char *line)` | yes | yes | `#[no_mangle] pub unsafe extern "C" fn printLine` |
| 2 | `printIntLine` | `void printIntLine(int intNumber)`  | yes | yes | `#[no_mangle] pub unsafe extern "C" fn printIntLine` |
| 3 | `bad`          | `void bad(void)`                    | yes | yes | `#[no_mangle] pub unsafe extern "C" fn bad` |
| 4 | `good`         | `void good(void)`                   | yes | yes | `#[no_mangle] pub unsafe extern "C" fn good` |
| 5 | `driver`       | `void driver(void)`                 | yes | yes | `#[no_mangle] pub unsafe extern "C" fn driver` |

There are no macro-generated symbols, no exported data objects, no global
variables, no enums and no typedefs in the C source.

### Raw `nm -D --defined-only` output

C (`c_src/build/libdriver.so`):

```
0000000000001159 T printLine
000000000000117b T printIntLine
00000000000011a2 T bad
00000000000011d6 T good
0000000000001215 T driver
```

Rust (`translation/target/release/libdriver.so`):

```
0000000000011880 T bad
00000000000118b0 T driver
0000000000011930 T good
0000000000011970 T printIntLine
0000000000011990 T printLine
```

## Symbol diff

```
comm -23 <(c defined names | sort) <(rust defined names | sort)   -> (empty)
comm -13 <(c defined names | sort) <(rust defined names | sort)   -> (empty)
```

**Missing from Rust: 0. Extra in Rust: 0.**

## Undefined (imported) symbols

All undefined symbols on both sides resolve to the platform libc / unwinder;
none is a symbol that the library itself should have defined.

* C imports: `printf`, `puts`, plus the usual weak `_ITM_*`,
  `__cxa_finalize`, `__gmon_start__`.
  Note: GCC rewrites `printf("%s\n", line)` into `puts(line)`, which is why
  `puts` appears. The emitted byte stream is identical either way.
* Rust imports: `printf`, `puts`, `memcpy`, `malloc`, `free`, … and the
  `_Unwind_*` family — all from libc / libgcc, i.e. the Rust standard-library
  runtime. **0 non-libc undefined symbols.**

## Result

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
