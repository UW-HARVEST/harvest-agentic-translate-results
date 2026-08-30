# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C:    c_src/build/libdriver.so
Rust: translation/target/release/libdriver.so
```

## Public (defined, dynamic) symbols exported by the C `.so`

| # | symbol | type | declared in | exported by Rust `.so`? |
|---|--------|------|-------------|-------------------------|
| 1 | `driver` | `T` (text, global) | `include/driver.h:27` — `void driver(int x);` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(x: c_int)` |
| 2 | `run`    | `T` (text, global) | not in header; implicitly external in `src/driver.c:53` — `void run(int extra_bedrooms)` | YES — `#[unsafe(no_mangle)] pub unsafe extern "C" fn run(extra_bedrooms: c_int)` |

`run` is NOT declared in `driver.h`, but it is a non-`static` definition, so the C
compiler gives it external linkage and it appears in the dynamic symbol table.
It is therefore a genuine public entry point and is treated as such (it is the
LOW-LEVEL entry point; `driver` is the convenience wrapper that calls it twice).

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so    | awk '{print $3}' | sort -u) \
           <(nm -D --defined-only translation/.../libdriver.so | awk '{print $3}' | sort -u)
<empty>
```

**MISSING FROM RUST: none.** The diff is empty.

## `static` (internal-linkage) C items — must NOT be exported

These have internal linkage in C and correctly do not appear in either
dynamic symbol table. They are reproduced as private Rust items.

| C item | linkage | Rust counterpart | exported? |
|--------|---------|------------------|-----------|
| `house_t` (typedef struct) | type only | `struct house_t` (`#[repr(C)]`) | n/a |
| `the_house` | `static` object | `static mut THE_HOUSE` | no (correct) |
| `add_floor` | `static` fn | `unsafe fn add_floor` | no (correct) |
| `add_bedrooms` | `static` fn | `unsafe fn add_bedrooms` | no (correct) |
| `add_floor_to_the_house` | `static` fn | `unsafe fn add_floor_to_the_house` | no (correct) |
| `print_the_house` | `static` fn | `unsafe fn print_the_house` | no (correct) |

No C source file was left untranslated: the library consists of exactly one
translation unit (`src/driver.c`, 66 lines) and every item in it is accounted
for above. No stubs and no `unimplemented!()` were introduced.

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak libc symbol: `printf@GLIBC_2.2.5`.
The Rust `.so` also imports `printf@GLIBC_2.2.5` (the translation deliberately
calls C `printf` rather than `println!`, so formatting and stdio buffering are
byte-identical). Every other Rust import is libc/`libgcc` unwinder support
pulled in by the Rust runtime (`malloc`, `memcpy`, `_Unwind_*`, …) — all are
resolvable libc/compiler-runtime symbols, not missing library symbols.

**0 missing / 0 unresolvable non-libc symbols in the Rust `.so`.**
