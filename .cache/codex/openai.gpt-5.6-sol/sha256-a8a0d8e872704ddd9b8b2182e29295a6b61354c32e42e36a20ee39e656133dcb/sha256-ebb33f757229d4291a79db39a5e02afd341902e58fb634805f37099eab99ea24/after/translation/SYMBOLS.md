# Dynamic symbol surface

Derived mechanically with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-QAS72x.so
```

Only globally defined project symbols are listed; undefined libc/loader symbols
are not implementations supplied by this library.

| C symbol | C type | Rust `.so` export | Status |
|----------|--------|-------------------|--------|
| `flac_validate` | `T` | `flac_validate` | [x] |
| `tflac_size_memory` | `T` | `tflac_size_memory` | [x] |

Completion requirement verified: the exact-name C-minus-Rust symbol diff is
empty, there are no extra Rust exports, and `ldd -r` reports no unresolved
relocations.
