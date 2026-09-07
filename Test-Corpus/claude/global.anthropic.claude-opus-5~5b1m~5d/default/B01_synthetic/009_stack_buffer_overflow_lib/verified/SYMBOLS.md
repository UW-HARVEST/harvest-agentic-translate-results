# SYMBOLS.md — public symbol surface

Source of truth: `nm -D --defined-only` on `c_src/build/libdriver.so`
(built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`, default/empty
`CMAKE_BUILD_TYPE`, i.e. `-O0`).

## C `.so` exported (global, defined) symbols

Command:

```
nm -D --defined-only c_src/build/libdriver.so | grep -v ' [wWvV] '
```

```
0000000000001159 T printLine
000000000000117b T printIntLine
00000000000011a2 T bad
0000000000001324 T good
0000000000001346 T driver
```

`goodG2B` and `goodB2G` are `static` in `c_src/src/driver.c` and therefore are
`t` (local), *not* exported. They must NOT be exported from Rust either
(otherwise the Rust `.so` would have a larger surface than the C one).

```
$ nm c_src/build/libdriver.so | grep -i 'good'
... t goodB2G
... t goodG2B
... T good
```

## Rust `.so` exported symbols

Command:

```
nm -D --defined-only translation/target/release/libdriver.so \
  | grep -v '_ZN\|rust\|__rust'
```

```
0000000000011920 T bad
0000000000011a00 T driver
0000000000011a50 T good
0000000000011bb0 T printIntLine
0000000000011bd0 T printLine
```

## Parity table

| # | C symbol | C binding | exported by Rust `.so`? | note |
|---|----------|-----------|-------------------------|------|
| 1 | `printLine`    | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` |
| 2 | `printIntLine` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printIntLine` |
| 3 | `bad`          | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn bad` |
| 4 | `good`         | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn good` |
| 5 | `driver`       | `T` (global text) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |
| — | `goodG2B`      | `t` (local)       | n/a — must stay private | private `unsafe fn goodG2B` in Rust: correct |
| — | `goodB2G`      | `t` (local)       | n/a — must stay private | private `unsafe fn goodB2G` in Rust: correct |

**Missing from Rust: NONE.** No translation gaps: `c_src` contains exactly one
translation unit (`src/driver.c`, 114 lines) plus one header
(`include/driver.h`), and every function in it (including the two `static`
helpers) has a corresponding Rust implementation in `translation/src/lib.rs`.

## Undefined (imported) symbols

C `.so` imports: `printf`, plus the usual glibc/ld.so bookkeeping
(`__stack_chk_fail` is absent at `-O0` here, `_ITM_*`/`__gmon_start__`/
`__cxa_finalize` are weak).

```
$ nm -D --undefined-only c_src/build/libdriver.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U printf@GLIBC_2.2.5
                 U puts@GLIBC_2.2.5
```

Note the `puts` import: the compiler rewrites `printf("%s\n", line)` into
`puts(line)`. The Rust `.so` imports **both `printf` and `puts` as well**
(LLVM performs the identical rewrite on `printf(b"%s\n\0", line)`), so the
`%`-in-string behaviour is the same in both: the argument is *not* treated as
a format string.

Rust `.so` imports the same `printf`/`puts` (the translation deliberately
routes all output through the C runtime rather than Rust's `std::io::stdout`,
so buffering/flushing/interleaving is identical), plus the standard glibc /
libgcc-unwind symbols pulled in by `std` (`memcpy`, `malloc`, `pthread_key_*`,
`_Unwind_*`, …). All are libc/libgcc symbols; **0 missing/undefined non-libc
symbols**.

## Verified

- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name.
- [x] The Rust `.so` exports no *extra* `driver.c`-derived symbol (the two
      `static` helpers stay private).
- [x] 0 missing/undefined non-libc symbols in the Rust `.so`.
