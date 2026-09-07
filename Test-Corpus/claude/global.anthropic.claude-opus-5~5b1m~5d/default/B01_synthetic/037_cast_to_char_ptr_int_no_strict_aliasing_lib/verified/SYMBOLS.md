# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:
- C:    `c_src/build/libdriver.so`
- Rust: `translation/target/release/libdriver.so`

## Exported (defined) dynamic symbols

`nm -D --defined-only`

| # | symbol | C `.so` | Rust `.so` | source of definition | status |
|---|--------|---------|------------|----------------------|--------|
| 1 | `driver` | `T driver` | `T driver` | `c_src/src/driver.c:36` → `translation/src/lib.rs` `#[no_mangle] pub extern "C" fn driver` | OK |

### Non-exported C symbols (intentionally not in the table above)

| C symbol | linkage | Rust counterpart | note |
|----------|---------|------------------|------|
| `print_hex` | `static` (file-local, `t`) | private `fn print_hex` in `lib.rs` | `static` in C ⇒ not part of the dynamic symbol surface, so the Rust `.so` must **not** export it either. Parity holds. |

## Missing-symbol analysis

Symbols exported by the C `.so` but **not** by the Rust `.so`: **NONE**.

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(empty)
```

No whole C module was skipped: the library consists of exactly one translation
unit (`c_src/src/driver.c`, 40 lines) with exactly one public header
(`c_src/include/driver.h`) declaring exactly one function. Both are translated.
No stubs and no `unimplemented!()` exist in the Rust crate.

## Undefined (imported) symbols

Undefined symbols are *not* required to match — they are the libc/runtime
imports each toolchain chooses. Recorded here only to prove none of them is a
missing *project* symbol.

C `.so` undefined:

```
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
U printf@GLIBC_2.2.5
U putchar@GLIBC_2.2.5
```

Rust `.so` undefined: 53 symbols, all of them libc (`printf`, `putchar`,
`memcpy`, `malloc`, `write`, …), glibc pthread/TLS helpers, or the
`_Unwind_*` personality routines pulled in by the Rust `std` panic runtime.

**0 missing / undefined non-libc symbols in the Rust `.so`.** ✅

Note: both libraries import `putchar` because both compilers (GCC and LLVM)
apply the same `printf("\n")` → `putchar('\n')` peephole optimisation. This is
an implementation detail of the same observable byte output.
