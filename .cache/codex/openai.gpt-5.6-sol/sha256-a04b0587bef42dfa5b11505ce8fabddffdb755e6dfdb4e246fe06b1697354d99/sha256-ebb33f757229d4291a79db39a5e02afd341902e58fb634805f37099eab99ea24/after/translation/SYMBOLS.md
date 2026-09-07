# Dynamic Symbol Surface

Source library:
`../c_src/build/libharvest-work-qjBfdv.so`

Rust library:
`target/release/libdiv_euclid_lib.so`

The exported API was derived with:

```text
nm -D --defined-only ../c_src/build/libharvest-work-qjBfdv.so
```

| C symbol | C type | Rust symbol | Status |
|----------|--------|-------------|--------|
| `div_euclid` | `T` (global function) | `div_euclid` | present |

The unfiltered C `nm -D` output also contains only the usual undefined weak
runtime references (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, and
`__gmon_start__`). They are not definitions exported by this library.

Missing defined C symbols in Rust: **0**.

