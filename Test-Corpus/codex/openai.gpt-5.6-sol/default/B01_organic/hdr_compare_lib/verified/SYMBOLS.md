# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-XreACI.so`

Rust library:
`target/release/libhdr_compare_lib.so`

The public API inventory is the mechanically extracted set from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-XreACI.so
```

| symbol | C `nm` type | Rust export | status |
|---|---:|---:|:---:|
| `hdr_compare` | `T` | `T` | [x] |

`hdr_valid` is `static` in `src/lib.c` and does not appear in the C dynamic
symbol table. The C library's only undefined dynamic symbols are weak ELF/glibc
runtime hooks (`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`); they are not
library API symbols.

- [x] No C-defined dynamic symbol is missing from the Rust shared library.
- [x] The Rust shared library has no undefined non-runtime library API symbol.
