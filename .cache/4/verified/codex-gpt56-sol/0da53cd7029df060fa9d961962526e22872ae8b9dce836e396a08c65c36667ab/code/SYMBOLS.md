# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

The C shared library has three globally defined public symbols. The Rust
comparison was generated with:

```text
nm -D --defined-only target/debug/libdriver.so
```

| symbol | C type | Rust export | status |
|--------|--------|-------------|--------|
| `get_os_arch` | `T` | `T` | present |
| `parse_uname_string` | `T` | `T` | present |
| `w_regexec` | `T` | `T` | present |

Missing C-defined symbols in Rust: **0**.

The remaining C `nm -D` entries are undefined or weak references supplied by
glibc/the ELF runtime (`fprintf`, `free`, `malloc`, `regcomp`, `regexec`,
`regfree`, `snprintf`, `stderr`, `strchr`, `strdup`, `strlen`, `strstr`,
`_ITM_*`, `__cxa_finalize`, and `__gmon_start__`); they are not library exports.
