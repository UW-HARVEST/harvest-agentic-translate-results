# Dynamic Symbol Surface

Source artifact: `c_src/build/libdriver_c.so`

Generated from:

```text
nm -D c_src/build/libdriver_c.so
```

| C address | type | symbol | classification | Rust parity |
|-----------|------|--------|----------------|-------------|
| | `w` | `_ITM_deregisterTMCloneTable` | Toolchain weak undefined hook | N/A |
| | `w` | `_ITM_registerTMCloneTable` | Toolchain weak undefined hook | N/A |
| | `w` | `__cxa_finalize@GLIBC_2.2.5` | glibc weak undefined import | N/A |
| | `w` | `__gmon_start__` | Toolchain weak undefined hook | N/A |
| | `U` | `__isoc99_scanf@GLIBC_2.7` | glibc undefined import | N/A |
| `0000000000001193` | `T` | `driver` | C-defined public function | Present |
| `00000000000011b8` | `T` | `main` | C-defined public function | Present |
| | `U` | `printf@GLIBC_2.2.5` | glibc undefined import | N/A |
| | `U` | `putchar@GLIBC_2.2.5` | glibc undefined import | N/A |

Defined-symbol comparison:

```text
$ comm -23 <(nm -D --defined-only c_src/build/libdriver_c.so | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
# no output
```

- [x] All 2 C-defined dynamic symbols are exported by the Rust shared object.
- [x] There are 0 missing or undefined non-libc C symbols.
