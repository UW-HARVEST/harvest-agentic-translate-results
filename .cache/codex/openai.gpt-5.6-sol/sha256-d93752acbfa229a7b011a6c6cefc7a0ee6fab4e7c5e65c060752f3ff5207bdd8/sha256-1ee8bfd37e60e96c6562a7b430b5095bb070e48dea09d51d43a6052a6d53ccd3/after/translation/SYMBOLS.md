# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Command used:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

## Public Symbols

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `driver` | `T` | `T` | present |

The C shared object has no other defined dynamic symbols. Its undefined
dynamic symbols (`__ctype_b_loc`, `printf`, `setlocale`, `tolower`, and
`toupper`, plus weak ELF runtime hooks) are provided by libc or the toolchain;
there are no undefined project-local symbols.

Completion: [x] zero C public symbols are missing from the Rust shared object.
