# Dynamic Symbol Surface

Ground truth:

```text
$ nm -D c_src/build/libtranslated_rust.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U floorf@GLIBC_2.2.5
0000000000001109 T hsv_to_rgb
```

## Defined public API

| symbol | C | Rust | status |
|--------|---|------|--------|
| `hsv_to_rgb` | `T` | `T` | present |

## Runtime imports

The weak `_ITM_*`, `__cxa_finalize`, and `__gmon_start__` entries are compiler
runtime references, not library exports. `floorf` is an imported libc/libm
symbol. They do not require Rust export wrappers.

The defined-symbol comparison is empty:

```text
$ comm -23 \
    <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only target/debug/libhsv_to_rgb_lib.so | awk '{print $3}' | sort -u)
```
