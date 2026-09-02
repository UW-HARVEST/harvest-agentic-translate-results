# SYMBOLS.md — Symbol parity: C `.so` vs Rust `.so`

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (ground truth)

`c_src/` contains exactly one translation unit and one header:

| file | lines |
|------|-------|
| `c_src/src/driver.c` | 66 |
| `c_src/include/driver.h` | 29 |

There is no second module, so there is no un-translated C source. All five
functions defined in `driver.c` are accounted for in `translation/src/lib.rs`.

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | C linkage | status |
|---|--------|---------|------------|-----------|--------|
| 1 | `printLine` | `T` | `T` | extern (`void printLine(const char *)`) | MATCH |
| 2 | `bad`       | `T` | `T` | extern (`void bad(void)`)               | MATCH |
| 3 | `good`      | `T` | `T` | extern (`void good(void)`)              | MATCH |
| 4 | `driver`    | `T` | `T` | extern (`void driver(void)`), declared in `driver.h` | MATCH |

**Symbol diff (C exports not present in Rust): EMPTY.**

```
$ comm -23 c.syms rust.syms
(no output)
```

## Deliberately NOT exported (internal linkage in C)

These are `static` in `driver.c`, so they appear as local `t` symbols in the C
`.so` and are absent from `nm -D`. The Rust translation keeps them private
(plain `fn`, no `#[no_mangle]`), which reproduces the C linkage exactly.
Exporting them would be a parity *failure*, not a fix.

| symbol | C `nm` class | Rust |
|--------|--------------|------|
| `helperBad`  | `t` (local) | private `fn helperBad` (`#[allow(dead_code)]`) |
| `helperGood` | `t` (local) | private `fn helperGood` |

Note: `helperBad` is dead code in the C too — `bad()` never calls it. That is
reproduced faithfully; see `CONFIGS.md` row 12.

## Undefined symbols

The C `.so` imports only `puts` (GCC rewrites `printf("%s\n", s)` into
`puts(s)`), plus the standard weak ELF/glibc stubs.

The Rust `.so` imports `puts` as well — LLVM applies the identical
`printf("%s\n", s)` → `puts(s)` transformation to the `c_printf` call in
`printLine`. The remaining Rust imports are all libc / libgcc unwinder symbols
pulled in by the Rust standard library (`malloc`, `memcpy`, `write`,
`_Unwind_*`, `pthread_key_*`, …).

**Non-libc undefined symbols in the Rust `.so`: 0.**

Every undefined symbol resolves against `libc.so.6`, `libgcc_s.so.1`, or
`ld-linux-x86-64.so.2`, all of which are listed as `NEEDED`.

## Note on `SONAME`

The C `.so` carries `SONAME = libdriver.so`; the Rust `cdylib` carries no
`SONAME`. Both files are also *named* `libdriver.so`. Because the differential
tests `dlopen` both objects into a single process, the test suite asserts (in
`test_00_both_libraries_are_distinct_objects`) that the two handles resolve
`driver` to different addresses, guarding against the loader silently aliasing
the second `dlopen` to the first object — which would make every comparison
trivially self-consistent and worthless.
