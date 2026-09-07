# Dynamic symbol surface

Derived from:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

The default C shared object was built from `mdcore.c` with `OP=add` and
`REPEAT=5`.

| symbol | C type | Rust parity |
|---|---|---|
| `G_OP` | global function pointer data | [x] |
| `G_OP_NAME` | global C-string pointer data | [x] |
| `helper_call` | function | [x] |
| `helper_ptr` | function | [x] |
| `op_add` | function | [x] |
| `op_mul` | function | [x] |
| `op_sub` | function | [x] |
| `use_generated` | function | [x] |

Undefined non-libc symbols required by C: none. The only strong undefined
symbol is `printf@GLIBC_2.2.5`; the other undefined entries are weak
toolchain/runtime hooks.

## Feature configurations

The C preprocessor has two independent axes:

- `OP`: `add`, `sub`, or `mul`; omitted defaults to `add`.
- `REPEAT`: `0` through `7`; omitted defaults to `5`.

Cargo permits every subset of the 11 declared features, and the Rust source
explicitly defines precedence for multi-selector subsets. Therefore all
`2^11 = 2048` feature combinations are valid:

```text
features = {add, sub, mul, 0, 1, 2, 3, 4, 5, 6, 7}
combination(mask) = every features[i] for which mask & (1 << i) != 0
mask = 0, 1, 2, ..., 2047
```

This is an exact mechanical enumeration of every subset, with mask zero being
`<none>`. Each subset maps to one of the 24 C configurations as follows:

- `OP=mul` if `mul` is present, else `OP=sub` if `sub` is present, else
  `OP=add`.
- `REPEAT` is the greatest present numeric feature, or `5` when no numeric
  feature is present.

## Verification result

All 2048 feature combinations passed `cargo check`, release build, dynamic-FFI
differential tests, executable stdout/stderr/exit-status comparison, and
defined-symbol parity against their mapped C configuration.
