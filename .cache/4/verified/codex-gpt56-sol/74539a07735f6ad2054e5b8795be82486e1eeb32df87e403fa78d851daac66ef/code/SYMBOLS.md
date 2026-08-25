# Dynamic Symbol Surface

Generated from:

```text
nm -D c_src/build/libdriver_c.so
```

`c_src/CMakeLists.txt` defines an executable-only target. The shared object was
linked from CMake's PIC object file without changing the C source:

```text
cc -shared c_src/build/CMakeFiles/driver.dir/src/main.c.o \
  -o c_src/build/libdriver_c.so
```

## Defined Public Symbols

| C symbol | C source | Rust export | Status |
|----------|----------|-------------|--------|
| `driver` | `c_src/src/main.c:32` | `src/lib.rs` | [x] |
| `main` | `c_src/src/main.c:37` | `src/lib.rs` (`c_main` with export name `main`) | [x] |

## Complete C Dynamic Symbol Inventory

| Binding | Symbol | Rust `.so` parity |
|---------|--------|--------------------|
| weak undefined | `_ITM_deregisterTMCloneTable` | [x] |
| weak undefined | `_ITM_registerTMCloneTable` | [x] |
| weak undefined | `__cxa_finalize@GLIBC_2.2.5` | [x] |
| weak undefined | `__gmon_start__` | [x] |
| undefined libc | `__isoc99_scanf@GLIBC_2.7` | [x] |
| defined global | `driver` | [x] |
| defined global | `main` | [x] |
| undefined libc | `printf@GLIBC_2.2.5` | [x] |

The defined-symbol diff is empty:

```text
comm -23 \
  <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

