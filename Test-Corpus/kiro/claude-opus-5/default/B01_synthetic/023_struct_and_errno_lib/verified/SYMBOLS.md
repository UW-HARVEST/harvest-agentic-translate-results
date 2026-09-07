# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Defined (exported) symbols

| # | C symbol | C type | Rust `.so` exports it? | Notes |
|---|----------|--------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES (`T driver`) | `void driver(const char *in)` — declared in `include/driver.h`. |
| 2 | `run`    | `T` (global text) | YES (`T run`)    | `void run(house_t *the_house, int extra_bedrooms)` — **not** declared in the public header, but has external linkage in `src/driver.c`, so it is part of the ABI. |

### Symbol diff

```
comm -23 <(nm -D --defined-only C.so    | awk '{print $NF}' | sort) \
         <(nm -D --defined-only RUST.so | awk '{print $NF}' | sort)
=> (empty)
```

**0 symbols missing from the Rust `.so`.**

## Symbols intentionally NOT exported

These are `static` in `src/driver.c` (internal linkage) and appear in neither
`.so`'s dynamic symbol table. They are translated as private Rust `fn`s:

| C symbol | signature |
|----------|-----------|
| `add_floor`   | `static void add_floor(house_t *)` |
| `add_bedrooms`| `static void add_bedrooms(house_t *, int)` |
| `print_house` | `static void print_house(house_t *)` |
| `parse_val`   | `static bool parse_val(const char *, int *)` |

`house_t` is a `typedef struct` local to `src/driver.c`; it is a *type*, not a
symbol. Its layout (`int floors; int bedrooms; double bathrooms;` → 16 bytes,
8-byte aligned on x86-64 SysV) is part of the `run` ABI and is reproduced by
`#[repr(C)] pub struct house_t`.

## Undefined (imported) symbols

| symbol | in C `.so` | in Rust `.so` | classification |
|--------|-----------|---------------|----------------|
| `printf@GLIBC` | U | U | libc |
| `puts@GLIBC` | U | U | libc (gcc rewrote `printf("An error occurred\n")` → `puts`) |
| `strtol@GLIBC` | U | U | libc |
| `__errno_location@GLIBC` | U | U | libc |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*` | w | w | toolchain weak refs |
| `_Unwind_*`, `abort`, `malloc`, `free`, `memcpy`, `mmap64`, `dl_iterate_phdr`, … | — | U | Rust `std` runtime (panic machinery, allocator, backtrace). Non-libc-visible extras are all satisfied by libc/libgcc; none are unresolved. |

`ldd` on the Rust `.so` resolves every entry — there are **0 missing/undefined
non-libc symbols**.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, therefore the only build configuration is the default one.
`cargo check --no-default-features` and `cargo check --all-features` are
equivalent to `cargo check`. Phase D's "every feature combination" reduces to a
single combination, verified explicitly by `scripts/verify_all.sh`, which enumerates the `[features]` section mechanically.
