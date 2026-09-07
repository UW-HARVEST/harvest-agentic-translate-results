# Dynamic Symbol Surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
nm -D --defined-only ../c_src/build/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust `.so` status |
|---|---:|---|
| `bad` | `T` | [x] exported as `bad` |
| `driver` | `T` | [x] exported as `driver` |
| `good` | `T` | [x] exported as `good` |
| `printLine` | `T` | [x] exported as `printLine` |

## C shared-object dependencies

These are dynamic imports/weak runtime hooks, not functions defined by this
library and therefore are not part of the Rust export requirement.

| symbol | C type |
|---|---:|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |
| `puts@GLIBC_2.2.5` | `U` |

## Current defined-symbol diff

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

Result: empty (zero missing C-defined symbols).

## Completion gate

- [x] Zero missing C-defined symbols in the Rust shared object.
- [x] All five valid-configuration rows pass with fixed-seed randomized inputs
  where inputs exist.
- [x] The sole error-surface row passes with the exact same no-output result.
- [x] Default and explicit `--no-default-features` release test runs pass.
- [x] No library-owned binary target exists in either build configuration, so
  executable stdout comparison is not applicable.
