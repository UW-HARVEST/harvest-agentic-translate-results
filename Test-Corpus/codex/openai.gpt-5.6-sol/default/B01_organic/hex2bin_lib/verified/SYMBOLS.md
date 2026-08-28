# Dynamic Symbol Surface

Reference library:
`../c_src/build/libharvest-work-PcBVYE.so`

Rust library:
`target/release/libhex2bin_lib.so`

The public API list is the mechanically extracted set from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-PcBVYE.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `hex2bin` | `T` | `hex2bin` | [x] |

The C library also has one strong dynamic import, `strchr@GLIBC_2.2.5`, plus
the standard weak toolchain imports. It has no other defined dynamic symbols.
The Rust library's additional undefined symbols are libc, libgcc unwinding,
pthread, and Rust runtime dependencies; there are no undefined project
symbols.

Completion criterion: [x] zero C-defined dynamic symbols missing from Rust.
