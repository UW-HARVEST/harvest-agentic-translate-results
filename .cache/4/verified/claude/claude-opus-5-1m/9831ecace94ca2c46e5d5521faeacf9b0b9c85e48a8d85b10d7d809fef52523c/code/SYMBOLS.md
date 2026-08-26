# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects. Nothing here is
inferred from what "looks important" — it is the raw dynamic symbol table.

## How this was produced

```sh
# C shared library
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D libdriver.so

# Rust shared library
cargo build --offline
nm -D target/debug/libdriver.so
```

Automated comparison: `./symbol_parity.sh` (see repo root).

## Raw `nm -D c_src/build/libdriver.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001173 T driver
                 U printf@GLIBC_2.2.5
                 U putchar@GLIBC_2.2.5
```

## C `.so` DEFINED (exported) symbols — the parity contract

`nm -D --defined-only c_src/build/libdriver.so`, excluding the toolchain-injected
weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__` glue (those are emitted by
crt/gcc into every shared object, not by the library's own source):

| # | C symbol | type | C declaration | present in Rust `.so`? | Rust definition |
|---|----------|------|---------------|------------------------|-----------------|
| 1 | `driver` | `T` (global text) | `void driver(float x);` — `c_src/include/driver.h:27`, defined `c_src/src/driver.c:35` | **YES** — `0000000000012250 T driver` | `src/lib.rs`, `#[unsafe(no_mangle)] pub extern "C" fn driver(x: f32)` |

**Missing symbols: 0.** The C library exports exactly one non-glue symbol and
the Rust `cdylib` exports it under the exact same name with the same
`extern "C"` signature. No wrapper had to be added and no C source file was
left untranslated.

## Completeness audit of the C source (is anything untranslated?)

`c_src` contains exactly two source files, both fully accounted for:

| C file | contents | translated? |
|--------|----------|-------------|
| `c_src/include/driver.h` | include guard + `void driver(float);` declaration | yes (the `extern "C"` signature of `driver`) |
| `c_src/src/driver.c` | `static void print_hex(unsigned char*, int)`, `void driver(float)` | yes — `print_hex` (private fn) and `driver` in `src/lib.rs` |

`print_hex` is `static` in C, therefore it is deliberately **not** in the C
`.so`'s dynamic symbol table, and it is correspondingly a private (non-exported)
Rust `fn`. Its absence from `nm -D` on the Rust `.so` is required parity, not a
gap. There is no third C file, so there is no skipped module to translate.

## Undefined (imported) symbols — must be libc/runtime only

C `.so` imports: `printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5` (gcc lowered the
source's `printf("\n")` into `putchar('\n')`; byte-for-byte the same output).

Rust `.so` imports: only glibc symbols (`printf`, `memcpy`, `malloc`, `free`,
`write`, `writev`, `open64`, `pthread_*`, `__errno_location`, …) and the
`_Unwind_*` family from `libgcc_s`. `printf` is imported by both, so the Rust
translation goes through the *same* libc `stdout` FILE object as the C code and
inherits identical buffering/flushing semantics.

**Undefined non-libc / non-runtime symbols in the Rust `.so`: 0.**

## Verdict

- [x] `nm -D` shows **0 missing** exported symbols in the Rust `.so`
      (C exports `{driver}`; Rust exports `{driver}`).
- [x] `nm -D` shows **0 undefined non-libc** symbols in the Rust `.so`.
- [x] No C source file/module was skipped by the translation; nothing is
      stubbed or `unimplemented!()`.
