# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only ../c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | linkage in C source | status |
|---|--------|---------|------------|---------------------|--------|
| 1 | `driver` | `T` | `T` | `void driver(const char *in)` — declared in `include/driver.h`, external | MATCH |
| 2 | `run`    | `T` | `T` | `void run(int extra_bedrooms)` — NOT in the header but NOT `static`, so external linkage; part of the exported ABI | MATCH |

**Symbol diff (C defined − Rust defined): EMPTY.**
**Symbol diff (Rust defined − C defined): EMPTY** (`cdylib` + only two
`#[unsafe(no_mangle)]` items; no extra public surface).

## Internal-linkage items (correctly NOT exported by either side)

Every other function/object in `c_src/src/driver.c` is `static`, i.e. internal
linkage, and must NOT appear in `nm -D` for either library:

| C item | kind | Rust counterpart | exported? |
|--------|------|------------------|-----------|
| `house_t` | `typedef struct` | `struct House` (`#[repr(C)]`) | n/a (type) |
| `the_house` | `static house_t` | `static mut THE_HOUSE` | no (both) |
| `add_floor` | `static void` | `fn add_floor` | no (both) |
| `add_bedrooms` | `static void` | `fn add_bedrooms` | no (both) |
| `add_floor_to_the_house` | `static void` | `unsafe fn add_floor_to_the_house` | no (both) |
| `print_the_house` | `static void` | `unsafe fn print_the_house` | no (both) |
| `parse_val` | `static bool` | `unsafe fn parse_val` | no (both) |

Confirmed: `nm -D` on both libraries lists exactly two `T` symbols, so no
internal item leaked into the dynamic symbol table on either side.

## Undefined (imported) symbols

Requirement: 0 missing/undefined **non-libc** symbols in the Rust `.so`.

C `.so` imports: `printf`, `puts`, `strtol`, `__errno_location` (all glibc)
plus the usual weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__` stubs.

> Note: the C compiler rewrote `printf("An error occurred\n")` into
> `puts("An error occurred")`. That is a libc-level, byte-equivalent
> transformation of the same output, not a behavioural difference.

Rust `.so` imports: the same `printf`, `strtol`, `__errno_location`, `puts`,
plus glibc/`libgcc_s` runtime support used by the Rust `std`/panic machinery
(`malloc`, `free`, `memcpy`, `write`, `writev`, `_Unwind_*`, `pthread_key_*`,
…). `ldd` resolves the Rust `.so` against only `libgcc_s.so.1`, `libc.so.6`
and the loader:

```
libgcc_s.so.1 => /lib64/libgcc_s.so.1
libc.so.6     => /lib64/libc.so.6
```

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.**

## Verdict

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
- [x] No whole C module was left untranslated: `c_src` contains exactly one
      translation unit (`src/driver.c`, 86 lines) and one header
      (`include/driver.h`); every function in it has a Rust counterpart above.
