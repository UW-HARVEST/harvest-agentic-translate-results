# Dynamic symbol surface

Source library: `../c_src/build/libharvest-work-QACph0.so`

Derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-QACph0.so
```

| C symbol | Rust export | Status |
|----------|-------------|--------|
| `apply_bitmask` | `apply_bitmask` | present |
| `arity` | `arity` | present |
| `arity2` | `arity2` | present |
| `arity3` | `arity3` | present |
| `arity4` | `arity4` | present |
| `compare_allocations` | `compare_allocations` | present |
| `init_matrix` | `init_matrix` | present |
| `process_string` | `process_string` | present |
| `shift_array` | `shift_array` | present |

The C library has no other defined dynamic symbols. Its undefined dynamic
symbols are libc/toolchain dependencies (`free`, `malloc`, `memmove`, `strlen`,
and weak runtime bookkeeping symbols), not library API.

Completion check: [x] re-run the final `nm -D` diff in Phase D.
