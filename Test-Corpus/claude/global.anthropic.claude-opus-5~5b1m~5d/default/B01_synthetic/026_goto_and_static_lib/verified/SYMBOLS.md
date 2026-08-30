# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only   c_src/build/libdriver.so
nm -D --undefined-only c_src/build/libdriver.so
nm -D --defined-only   translation/target/release/libdriver.so
nm -D --undefined-only translation/target/release/libdriver.so
```

## C source surface

The whole library is two files:

| C file | contents |
|--------|----------|
| `c_src/include/driver.h` | single public declaration `void driver(int x, int y, int z);` |
| `c_src/src/driver.c` | file-scope `static int y = 123;`, `static int multi_stage(int x, int z)`, `void driver(int x, int local_y, int z)` |

There are no other translation units, no macro-generated symbol families,
no `#ifdef`-guarded extra entry points. `multi_stage` and `y` are `static`
(internal linkage) and therefore MUST NOT be exported by either `.so`.

## Exported (dynamic, defined) symbols

| # | symbol | in C `.so` | in Rust `.so` | notes |
|---|--------|-----------|---------------|-------|
| 1 | `driver` | `T` (yes) | `T` (yes) | public entry point, `void driver(int,int,int)` |

### Symbols exported by C but MISSING from Rust

*(none — the diff is empty)*

### Internal-linkage C symbols that must NOT appear (negative check)

| symbol | in C `.so` | in Rust `.so` |
|--------|-----------|---------------|
| `multi_stage` | absent (static) | absent (private `fn`) |
| `y` | absent (static) | absent (private `static Y`) |

## Undefined (imported) symbols

These are libc / runtime imports and are NOT part of the API surface; they
are listed only to document that nothing non-libc is dangling.

C `.so` imports: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, plus the usual
weak CRT symbols (`__cxa_finalize`, `__gmon_start__`,
`_ITM_(de)registerTMCloneTable`).

> Note: the C compiler rewrote the constant-format `printf("...\n")` calls
> into `puts("...")`. That is an internal optimisation; the bytes written to
> `stdout` are identical, which is what the differential tests compare.

Rust `.so` imports: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, plus libc
allocator / IO / TLS / unwinder symbols pulled in by the Rust standard
library (`malloc`, `free`, `memcpy`, `write`, `_Unwind_*`, `pthread_key_*`,
…) and the same weak CRT symbols.

**All Rust undefined symbols are libc/`libgcc_s` runtime symbols. There are
0 missing or dangling non-libc symbols.**

## Result

- C exported symbols: 1 (`driver`).
- Rust exported symbols: 1 (`driver`).
- Missing from Rust: **0**.
- Extra non-libc undefined in Rust: **0**.

`SYMBOLS.md` gate: **PASS** (verified by `tests/symbols.rs`, which re-runs
`nm -D` on both objects and asserts the diff is empty).
