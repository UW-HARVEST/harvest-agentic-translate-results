# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically:

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so
```

## C source inventory (whole library — nothing is excluded)

The CMake target `driver` compiles exactly one translation unit:

| C file | lines | functions with external linkage |
|--------|-------|---------------------------------|
| `c_src/src/driver.c`     | 40 | `foo`, `driver` |
| `c_src/include/driver.h` | 28 | (declares `driver` only; header guard `DRIVER_H_` is the only preprocessor construct) |

There are **no** namespace/renaming macros in the header, so the linker names
are exactly the source-level names. `foo` is *not* `static`, so it has external
linkage and is exported even though it is absent from the public header.

## Defined dynamic symbols

| # | symbol | C `.so` | Rust `.so` | C prototype | Rust definition |
|---|--------|---------|------------|-------------|-----------------|
| 1 | `driver` | `T` (0x1176) | `T` | `void driver(const char *in)`     | `src/driver.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |
| 2 | `foo`    | `T` (0x1129) | `T` | `int  foo(const char *in, char c)` | `src/driver.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn foo` |

**Symbol diff (C exports − Rust exports) = EMPTY.** Verified automatically by
the test `phase_d_symbol_parity` in `tests/differential.rs`, which reruns `nm -D`
on both shared objects and asserts set equality, so the gate cannot silently rot.

Nothing was missing, so no `#[no_mangle]` wrapper had to be added and no
untranslated C module had to be back-filled: `driver.c` is the entire library
and both of its external-linkage functions were already translated.

## Undefined (imported) symbols

The C `.so` imports `printf@GLIBC_2.2.5` and `strchr@GLIBC_2.2.5` (plus the
usual weak `_ITM_*` / `__gmon_start__` / `__cxa_finalize`).

The Rust `.so` imports **the same two**: `printf@GLIBC_2.2.5` and
`strchr@GLIBC_2.2.5`. Both are deliberately taken from libc rather than
reimplemented, so that the emitted bytes, the stdout buffering behaviour and the
faulting behaviour on out-of-bounds/NULL pointers are identical to the C
library's (see the `strchr` note in `src/driver.rs`; the original translation
hand-rolled `strchr` and that caused a real divergence — see "Divergence found
and fixed" below). Its remaining imports (`_Unwind_*`, `malloc`, `memcpy`,
`dl_iterate_phdr`, …) are libc / libgcc_s runtime support pulled in by Rust
`std`.

## Divergence found and fixed

The hand-written `strchr` in the original translation dereferenced the raw
pointer with `*p`. In a `dev` build rustc's debug assertions insert a
null-pointer-dereference check there, which **panics**; the panic then crosses
the `extern "C"` boundary of `foo`, which is a non-unwinding boundary, so the
process **aborts**. Result:

| input | C `.so` | Rust `.so` (before) | Rust `.so` (after) |
|-------|---------|---------------------|--------------------|
| `foo(NULL, 'A')` | killed by `SIGSEGV` (11) | killed by `SIGABRT` (6) | killed by `SIGSEGV` (11) |
| `foo(NULL, '\0')` | `SIGSEGV` | `SIGABRT` | `SIGSEGV` |
| `driver(NULL)`   | `SIGSEGV` | `SIGABRT` | `SIGSEGV` |

Fixed by calling libc `strchr` — the very function `driver.c` calls — with the
`char`→`int` integer promotion spelled out at the call site (`c as c_int`).

## Test-harness hazard worth recording

`cargo test` does **not** rebuild a `cdylib`-only lib target, because
integration tests never link against it. The suite therefore loaded whatever
`target/<profile>/libdriver.so` an earlier `cargo build` had left behind, and
source edits appeared to have no effect. `tests/differential.rs` now asserts
that the `.so` is newer than everything in `src/` and than `Cargo.toml`, and
`run_tests.sh` always runs `cargo build` before `cargo test`.

**0 missing symbols and 0 undefined non-libc symbols in the Rust `.so`.**
Verified with:

```
ldd -r target/debug/libdriver.so     # no "undefined symbol" lines
```
