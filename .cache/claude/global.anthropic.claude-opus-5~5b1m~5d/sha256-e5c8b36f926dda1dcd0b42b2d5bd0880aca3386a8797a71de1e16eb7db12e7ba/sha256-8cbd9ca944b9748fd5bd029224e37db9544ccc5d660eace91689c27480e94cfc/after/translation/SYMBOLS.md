# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` defined dynamic symbols (`nm -D --defined-only c_src/build/libdriver.so`)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `driver` | `T` (text, global) | YES — `T driver` |

That is the complete list: `c_src/src/driver.c` defines exactly two functions,
`print_hex` (declared `static`, therefore **not** an exported symbol — it is a
local `t`/absent symbol only) and `driver` (exported, declared in
`include/driver.h`). `c_src/include/driver.h` declares exactly one function:
`void driver(float x);`. There are no exported globals, no macro-generated
symbols, no enums, and no additional translation units in `CMakeLists.txt`
(`add_library(driver SHARED src/driver.c)`).

## Rust `.so` defined dynamic symbols (`nm -D --defined-only translation/target/release/libdriver.so`)

| # | symbol | type | present in C `.so`? |
|---|--------|------|---------------------|
| 1 | `driver` | `T` (text, global) | YES |

No extra exports (the Rust `print_hex` is a private `unsafe fn`, matching the C
`static`).

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(empty)
```

**Result: EMPTY diff — full symbol parity.** 0 missing symbols in the Rust
`.so`, 0 extra symbols.

## Undefined (imported) symbols

Both libraries import `printf` and `putchar` from glibc — `putchar` appears
because both the C compiler and LLVM lower the trailing `printf("\n")` into
`putchar('\n')`. The Rust `.so` additionally imports the usual Rust runtime
support symbols (`_Unwind_*`, `malloc`, `memcpy`, `dl_iterate_phdr`, ...). All
of these are libc / libgcc symbols, not library-level API.

**0 missing / undefined non-libc symbols in the Rust `.so`.**
