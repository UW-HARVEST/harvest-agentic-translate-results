# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libSimpleList.so
```

## Public Defined Symbols

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `smallestValue` | `T` (global text) | `smallestValue` | [x] |

The defined-symbol diff is empty:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libSimpleList.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only translation/target/release/libSimpleList.so | awk '{print $3}' | sort -u)
```

## Undefined Runtime Symbols

The C shared object has only these weak toolchain/runtime imports; none are
library API symbols or non-libc unresolved dependencies:

| Symbol | Binding |
|--------|---------|
| `_ITM_deregisterTMCloneTable` | weak undefined |
| `_ITM_registerTMCloneTable` | weak undefined |
| `__cxa_finalize@GLIBC_2.2.5` | weak undefined |
| `__gmon_start__` | weak undefined |
