# Dynamic Symbol Surface

Generated from the default C shared library with:

```text
nm -D --defined-only --format=posix c_src/build/libdriver.so
```

The CMake project has one shared-library target (`driver`) and no build-time
options. `Cargo.toml` has no `[features]` table, so the Rust feature matrix has
one valid combination: `--no-default-features` with an empty feature list.

| C symbol | type | C source | Rust export | status |
|----------|------|----------|-------------|--------|
| `driver` | `T` | `c_src/src/goto.c:65` | `driver` | [x] |
| `forward_goto_example` | `T` | `c_src/src/goto.c:29` | `forward_goto_example` | [x] |
| `open_with_cleanup` | `T` | `c_src/src/goto.c:42` | `open_with_cleanup` | [x] |

The C library's remaining dynamic symbols are undefined libc/toolchain imports,
not public definitions: `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`, `fclose`,
`ferror`, `fgets`, `fopen`, `fprintf`, `fwrite`, `printf`, and `stderr`.

Completion gate: [x] the sorted C export set equals the sorted Rust export set,
with no missing or undefined non-libc implementation symbols.
