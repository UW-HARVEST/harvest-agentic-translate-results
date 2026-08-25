# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`, compiled from
`c_src/src/main.c` with `cc -shared -fPIC`.

Command:

```text
nm -D c_src/build/libdriver_c.so
```

## C-defined public symbols

| symbol | nm type | C definition | Rust parity |
|--------|---------|--------------|-------------|
| `driver` | `T` | `c_src/src/main.c:38` | [x] |
| `main` | `T` | `c_src/src/main.c:43` | [x] |
| `print_foo` | `T` | `c_src/src/main.c:34` | [x] |

The initial Rust program was binary-only and exported none of these through a
Rust shared object. The translated implementations now have real `extern "C"`
exports, and `comm` over the sorted C/Rust `nm -D --defined-only` symbol names
produces no missing symbols.

## C imported/runtime symbols

These entries also appear in the unfiltered `nm -D` output. They are runtime
imports, not functions defined by this library:

| symbol | nm type |
|--------|---------|
| `_ITM_deregisterTMCloneTable` | `w` |
| `_ITM_registerTMCloneTable` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` |
| `__gmon_start__` | `w` |
| `__isoc99_scanf@GLIBC_2.7` | `U` |
| `printf@GLIBC_2.2.5` | `U` |
