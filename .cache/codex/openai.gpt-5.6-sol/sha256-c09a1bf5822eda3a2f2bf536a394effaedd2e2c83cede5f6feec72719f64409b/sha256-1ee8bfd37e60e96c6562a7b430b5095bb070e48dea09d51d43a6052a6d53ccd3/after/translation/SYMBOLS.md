# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-WUNOKH.so`

Rust library:
`target/release/libldexp_q2_lib.so`

The API inventory is derived from:

```sh
nm -D --defined-only ../c_src/build/libharvest-work-WUNOKH.so
nm -D --defined-only target/release/libldexp_q2_lib.so
```

## Defined Public Symbols

| C symbol | C type | Rust type | Rust status |
|----------|--------|-----------|-------------|
| `ldexp_q2` | `T` | `T` | present |

The C library has no other defined dynamic symbols.

## C Toolchain Imports

Plain `nm -D` also reports these undefined weak runtime symbols. They are
toolchain imports rather than library exports:

| Symbol | C binding | Rust dynamic table |
|--------|-----------|--------------------|
| `_ITM_deregisterTMCloneTable` | weak, undefined | present |
| `_ITM_registerTMCloneTable` | weak, undefined | present |
| `__cxa_finalize@GLIBC_2.2.5` | weak, undefined | present |
| `__gmon_start__` | weak, undefined | present |

## Parity

- [x] Every API symbol defined by the C shared object is defined by the Rust
      shared object with the exact same name.
- [x] Missing C API symbols: 0.

