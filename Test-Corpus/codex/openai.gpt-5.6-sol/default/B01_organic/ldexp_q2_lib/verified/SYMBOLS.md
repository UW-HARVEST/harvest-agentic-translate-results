# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-54f7Vc.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `ldexp_q2` | `T` | `ldexp_q2` | [x] |

The C shared library has no other defined dynamic symbols. The weak and
undefined entries printed by `nm -D` are toolchain/runtime imports, not public
library API definitions.

Final `comm` comparison of sorted defined dynamic symbol names: empty (zero
symbols missing from Rust).
