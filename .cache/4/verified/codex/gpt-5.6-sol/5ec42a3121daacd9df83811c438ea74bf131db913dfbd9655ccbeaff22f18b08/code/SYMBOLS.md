# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libtranslated_rust.so
```

The C shared object has one defined public API symbol. The remaining four
entries are weak, undefined toolchain hooks rather than library exports.

| symbol | C `nm -D` type | role | Rust `.so` status |
|---|---:|---|---|
| `_ITM_deregisterTMCloneTable` | `w` | weak undefined toolchain hook | present (`w`) |
| `_ITM_registerTMCloneTable` | `w` | weak undefined toolchain hook | present (`w`) |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | weak undefined libc hook | present (`w`) |
| `__gmon_start__` | `w` | weak undefined profiling hook | present (`w`) |
| `hdr_bitrate` | `T` | defined public API | present (`T`) |

Defined-export parity command:

```text
comm -23 \
  <(nm -D --defined-only --extern-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u) \
  <(nm -D --defined-only --extern-only target/debug/libhdr_bitrate_lib.so | awk '{print $3}' | sort -u)
```

- [x] No C-defined public symbol is missing from the Rust shared object.
- [x] No C undefined non-libc dependency requires a Rust implementation.
