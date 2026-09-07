# Dynamic Symbol Surface

Derived from:

```text
nm -D ../c_src/build/libharvest-work-apqsdJ.so
nm -D target/release/libhex2bin_lib.so
```

## Public C library exports

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `hex2bin` | `T` (defined global function) | `hex2bin` | [x] present |

The C shared object has no other defined global/weak public symbols.

## C shared-object runtime imports

These entries also appear in `nm -D`, but are undefined toolchain/libc
dependencies rather than public symbols implemented by this library:

| Symbol | `nm` type | Classification |
|--------|-----------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | optional toolchain runtime |
| `_ITM_registerTMCloneTable` | `w` | optional toolchain runtime |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc/toolchain runtime |
| `__gmon_start__` | `w` | optional toolchain runtime |
| `strchr@GLIBC_2.2.5` | `U` | libc dependency |

The Rust shared object may have a different set of undefined runtime imports;
those are not library API exports. The exact defined-export diff is empty.
