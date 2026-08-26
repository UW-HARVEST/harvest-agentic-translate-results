# Dynamic Symbol Surface

Reference library:

```text
cc -shared -fPIC -O0 -o target/c-reference/libdriver_c.so c_src/src/main.c
nm -D --defined-only --format=posix target/c-reference/libdriver_c.so
```

The C source has no public header. Therefore, the mechanically derived public
surface is the complete set of globally defined dynamic symbols in the shared
object:

| symbol | C type | Rust status | source |
|--------|--------|-------------|--------|
| `main` | `T` | [x] exported and matched | `c_src/src/main.c:61` |
| `run` | `T` | [x] exported and matched | `c_src/src/main.c:51` |

The C object's undefined entries are runtime imports (`scanf`, `printf`, and
ELF toolchain hooks), not library exports, and are excluded from symbol parity.

Verified with an empty set difference:

```text
comm -23 \
  <(nm -D --defined-only target/c-reference/libdriver_c.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```
