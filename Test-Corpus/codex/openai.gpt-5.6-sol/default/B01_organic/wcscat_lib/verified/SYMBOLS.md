# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-kx3K47.so`

Inventory command:

```sh
nm -D --defined-only --format=posix \
  ../c_src/build/libharvest-work-kx3K47.so
```

| C symbol | Type | C source | Rust export | Status |
|----------|------|----------|-------------|--------|
| `wcscat` | `T` | `src/lib.c:5` | `src/lib.rs:13` | [x] |

The C dynamic symbol table has no other defined public symbols.
The final C-minus-Rust symbol diff is empty, and `ldd -r` reports no unresolved
runtime symbols in the Rust shared library.
