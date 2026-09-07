# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libharvest-work-DWqeAw.so
```

| C symbol | Kind | Rust export | Status |
|----------|------|-------------|--------|
| `rev16` | `T` (global function) | `rev16` | Present; Phase D parity confirmed |

The C shared library exports exactly one public defined dynamic symbol. There
are no missing Rust implementations or wrappers. The final sorted symbol diff
is empty, and `ldd -r` reports no unresolved relocations for either library.
