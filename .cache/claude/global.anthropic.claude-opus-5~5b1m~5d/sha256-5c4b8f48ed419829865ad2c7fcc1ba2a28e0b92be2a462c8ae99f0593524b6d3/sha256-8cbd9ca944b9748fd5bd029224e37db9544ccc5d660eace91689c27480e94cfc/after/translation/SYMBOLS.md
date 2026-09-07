# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `driver` | T (present) | T (present) | `void driver(const char *in)` — declared in `include/driver.h`. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |
| 2 | `run`    | T (present) | T (present) | `void run(house_t *the_house, int extra_bedrooms)` — NOT in the public header, but non-`static` in `src/driver.c`, therefore an exported symbol and a real low-level entry point. Rust: `#[unsafe(no_mangle)] pub unsafe extern "C" fn run` |

**Symbol diff (C-defined minus Rust-defined): EMPTY.** ✔

## `static` C functions (deliberately NOT exported by either library)

These have internal linkage in C, so they must NOT appear in `nm -D` for either
side. They are private helpers in the Rust translation as well.

| C function | linkage | Rust counterpart |
|---|---|---|
| `static void add_floor(house_t *)` | internal | `fn add_floor` (private) |
| `static void add_bedrooms(house_t *, int)` | internal | `fn add_bedrooms` (private) |
| `static void print_house(house_t *)` | internal | `fn print_house` (private) |
| `static bool parse_val(const char *, int *)` | internal | `fn parse_val` (private) |

## Undefined (imported) symbols

Both libraries import only libc symbols; the Rust `.so` additionally imports the
Rust runtime's libc/unwind set. No non-libc undefined symbols on either side.

C imports: `__errno_location`, `printf`, `puts`, `strtol` (+ weak
`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Note: GCC rewrites the C `printf("An error occurred\n")` call into `puts`,
which is why `puts` appears in the C imports. This is a pure
compiler optimization — the bytes written to stdout are identical, so the Rust
side may legitimately keep using `printf`. (`puts` also appears in the Rust
imports via the Rust std runtime.)

Rust extra imports are all glibc/`libgcc_s` (`_Unwind_*`, `malloc`, `memcpy`,
`mmap64`, `pthread_key_*`, `write`, …) pulled in by `std`. None are
project symbols.

## Verification checklist

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with the
      exact same name.
- [x] No extra project symbols exported by the Rust `.so`.
- [x] 0 missing / 0 undefined non-libc symbols in the Rust `.so`.
