# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libString_Slice.so
```

The C shared object has one defined public dynamic symbol.

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `slice` | `T` (global text) | `slice` | [x] exported |

The complete unfiltered C `nm -D` output also contains only these imported or
weak runtime symbols: `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`, `printf`,
`puts`, and `strlen`. They are not C-library API definitions and therefore do
not require Rust exports.

## Completion check

- [x] Every C-defined public dynamic symbol is exported by the Rust `.so`.
- [x] Missing C-defined symbols: 0.
