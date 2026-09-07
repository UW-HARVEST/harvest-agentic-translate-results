# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C:    `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## C source inventory (completeness check)

`c_src/` contains exactly one translation unit and one public header:

| C file | public functions defined |
|--------|--------------------------|
| `c_src/src/lib.c`      | `custom_strdup` |
| `c_src/include/lib.h`  | (declaration only) `char *custom_strdup(const char *str);` |

No other `.c` files exist, so no module was skipped by the translation.

## Public symbol table

| # | symbol | C `.so` | Rust `.so` | type | status |
|---|--------|---------|------------|------|--------|
| 1 | `custom_strdup` | `T` (0x1129) | `T` | `char *(const char *)` | ✅ present in both |

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
(empty)
```

**0 symbols missing from the Rust `.so`.**

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, matching the C `.so`'s imports
(`malloc`, `memcpy`, `strlen` — the C compiler may inline `strlen`/`memcpy`,
which is a codegen detail, not an ABI difference):

| symbol | C `.so` imports | Rust `.so` imports | libc? |
|--------|-----------------|--------------------|-------|
| `malloc`  | yes | yes | yes |
| `memcpy`  | yes | yes | yes |
| `strlen`  | yes (may be inlined) | yes | yes |

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Allocator compatibility note

The C `custom_strdup` returns a `malloc`-allocated buffer that the caller frees
with `free`. The Rust translation calls the platform `malloc` directly (not the
Rust global allocator), so a pointer returned from the Rust `.so` is safely
`free`-able by an external C caller — same ownership contract as the C version.
This is verified by `tests/differential.rs::phase_b_row_*`, which `free()`s every
returned pointer via libc.
