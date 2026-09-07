# Dynamic symbol surface

Source command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust `.so` export | Status |
|----------|--------|-------------------|--------|
| `get_os_arch` | `T` | `get_os_arch` | present |
| `parse_uname_string` | `T` | `parse_uname_string` | present |
| `w_regexec` | `T` | `w_regexec` | present |

The mechanically extracted defined-symbol diff is empty:

```text
comm -23 \
  <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort -u)
```

The C library's remaining dynamic entries are libc/glibc imports and toolchain
weak symbols, not library API exports. The Rust library has no undefined
non-system `driver` symbols.

- [x] Final `nm -D --defined-only` API-symbol diff is empty.
