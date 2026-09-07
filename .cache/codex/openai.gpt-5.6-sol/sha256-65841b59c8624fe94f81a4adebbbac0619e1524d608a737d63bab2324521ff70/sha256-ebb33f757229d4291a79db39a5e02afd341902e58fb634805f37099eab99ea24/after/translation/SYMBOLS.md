# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-eBuD7L.so`

Rust library:
`target/release/libmd5_digest_lib.so`

Inventory command:

```sh
nm -D ../c_src/build/libharvest-work-eBuD7L.so
```

Every entry emitted by `nm -D` for the C shared object is listed below.
`T` is a symbol defined and exported by the library; lowercase `w` entries are
weak runtime imports rather than public C API definitions.

| C type | symbol | C-defined public API | present in Rust `nm -D` | status |
|--------|--------|----------------------|-------------------------|--------|
| `w` | `_ITM_deregisterTMCloneTable` | no | yes | runtime import |
| `w` | `_ITM_registerTMCloneTable` | no | yes | runtime import |
| `w` | `__cxa_finalize@GLIBC_2.2.5` | no | yes | runtime import |
| `w` | `__gmon_start__` | no | yes | runtime import |
| `T` | `md5_digest` | yes | yes, exact name | exported |

Defined-public-symbol comparison:

```sh
comm -23 \
  <(nm -D --defined-only ../c_src/build/libharvest-work-eBuD7L.so |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' | sort -u) \
  <(nm -D --defined-only target/release/libmd5_digest_lib.so |
    awk '$2 ~ /^[TDBR]$/ { print $3 }' | sort -u)
```

Result: empty (zero missing C-defined public symbols).

