# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Artifacts compared:

- C:    `c_src/build/libharvest-work-H33xBf.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
- Rust: `translation/target/release/libhdr_compare_lib.so`
  (built with `cd translation && cargo build --release`)

## Complete C source surface

The entire C library is two files:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | one declaration: `int hdr_compare(const uint8_t *h1, const uint8_t *h2);` |
| `c_src/src/lib.c` | `static int hdr_valid(const uint8_t *h)` (file-local, **not** exported) and `int hdr_compare(...)` |

`CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`) into the
shared library. There is no second module, no macro-generated symbol family, and
no conditional compilation (`#ifdef`) anywhere in the C sources. Therefore the
public ABI surface is a single function, and **no C source was left
untranslated** — `translation/src/lib.rs` contains both `hdr_valid` (as a private
`unsafe fn`, matching the C `static`) and `hdr_compare` (as
`#[unsafe(no_mangle)] pub unsafe extern "C" fn`).

## `nm -D --defined-only` — exported symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|-----------|--------|
| 1 | `hdr_compare` | `T` (0x1190) | `T` (0x11690) | **present in both** |

Raw output:

```
$ nm -D --defined-only c_src/build/libharvest-work-H33xBf.so
0000000000001190 T hdr_compare

$ nm -D --defined-only translation/target/release/libhdr_compare_lib.so
0000000000011690 T hdr_compare
```

### Symbol diff

```
C-exported symbols missing from Rust .so :  (none)
```

The diff is **empty**. `hdr_valid` is `static` in C, so it is deliberately absent
from both `.so` files (confirmed: it does not appear in either `nm -D` listing);
keeping it private in Rust is the correct parity choice, not a gap.

## `nm -D -u` — undefined (imported) symbols

| side | non-libc / non-runtime undefined symbols |
|------|------------------------------------------|
| C    | none (only the weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__` glibc/ITM stubs) |
| Rust | none — every entry is glibc (`malloc`, `memcpy`, `open64`, `pthread_key_create`, …) or the C++/Rust unwinder (`_Unwind_*` from `libgcc`), both pulled in by Rust `std`'s panic/backtrace machinery, not by untranslated code |

**0 missing / undefined non-libc symbols in the Rust `.so`.** ✅

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent to the
default here). Phase D's "every feature combination" therefore collapses to the
single combination, which is verified explicitly by
`tests/feature_matrix.sh` / the commands recorded in `VERIFICATION.md`.
