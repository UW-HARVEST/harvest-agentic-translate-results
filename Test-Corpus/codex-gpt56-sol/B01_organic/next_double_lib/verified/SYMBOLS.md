# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libtranslated_rust.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `next_double` | `T` (global function) | `next_double` | [x] parity verified |

`cn_rnd_next` is `static` in `c_src/src/lib.c` and is not part of the C
dynamic symbol surface.
