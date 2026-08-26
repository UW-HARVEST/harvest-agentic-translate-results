# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

The default C build has one globally defined dynamic symbol. Undefined entries
from plain `nm -D` are runtime/libc imports, not library exports.

| symbol | C type | Rust type | status |
|--------|--------|-----------|--------|
| `hex2bin` | `T` | `T` | [x] |

Rust comparison command:

```text
nm -D --defined-only target/debug/libhex2bin_lib.so
```

Missing C exports in Rust: **0**

The C library imports `strchr@GLIBC_2.2.5`; all other undefined C dynamic
symbols are weak toolchain/runtime symbols. There are no undefined non-libc
library symbols.
