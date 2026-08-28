# Exported Symbol Surface

Source library:
`../c_src/build/libharvest-work-MGK5vE.so`

Inventory command:

```sh
nm -D --defined-only --format=posix \
  ../c_src/build/libharvest-work-MGK5vE.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `merge_sort` | `T` | `merge_sort` | Present |

The C shared library exports one defined public symbol. The Rust shared library
exports the same symbol with the exact name. There are no missing symbols.
`ldd -r target/release/libmerge_sort_lib.so` also reports no unresolved
symbols; its dynamic imports resolve through `libc` and `libgcc_s`.
