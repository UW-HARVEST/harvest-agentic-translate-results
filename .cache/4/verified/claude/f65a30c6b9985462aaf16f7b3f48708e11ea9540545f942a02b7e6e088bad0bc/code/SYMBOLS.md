# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

* C  : `c_src/build/libdriver.so`   (cmake, default configuration)
* Rust: `target/debug/libdriver.so` (`cargo build --no-default-features`)

## Raw `nm -D` output

### C `.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001173 T driver
                 U printf@GLIBC_2.2.5
                 U putchar@GLIBC_2.2.5
```

### Rust `.so` (defined-only)

```
0000000000012620 T driver
```

## Exported (defined) symbol parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T` (0x1173) | `T` (0x12620) | **present in both** |

`nm -D --defined-only` on the C `.so` yields exactly one line (`driver`); the
Rust `.so` yields exactly one line (`driver`).

**Symbol diff (C defined − Rust defined): EMPTY.** No symbol needs a new
`#[no_mangle]` wrapper and no C source file was left untranslated: the library
consists of a single translation unit (`c_src/src/driver.c`) whose only
non-`static` definition is `driver`.

## Non-exported C symbols (intentionally not in the ABI)

| C symbol | linkage | present in Rust? | rationale |
|----------|---------|------------------|-----------|
| `print_hex` | `static void` (internal) | yes, as a private `fn print_hex` | `static` in C ⇒ not in `nm -D`; must NOT be exported by Rust either. Verified: absent from both `.so` dynamic tables. |
| `house_t` | file-local `typedef` | yes, as private `#[repr(C)] struct HouseT` | type, no symbol |

## Undefined (imported) symbols

| symbol | C `.so` | Rust `.so` | notes |
|--------|---------|------------|-------|
| `printf@GLIBC_2.2.5` | `U` | `U` | both route formatting through the *same* glibc `printf`, so `%02x` conversion and `stdout` buffering are shared, not reimplemented |
| `putchar@GLIBC_2.2.5` | `U` | (not required) | gcc's own optimisation of `printf("\n")` → `putchar('\n')`; a libc-internal detail with identical observable output. Not an ABI obligation. |
| `_ITM_*`, `__cxa_finalize`, `__gmon_start__` | `w` (weak) | n/a | crt/toolchain boilerplate, not part of the library API |

**Result: 0 missing and 0 undefined non-libc symbols in the Rust `.so`.**
