# Dynamic Symbol Surface

Derived from:

```text
nm -D --defined-only c_src/build/libdriver_c.so
nm -D --defined-only target/debug/libdriver.so
```

The C shared object was compiled from the default, option-free CMake
configuration with `-fPIC`. CMake defines no build options or preprocessor
configurations.

| C symbol | Type | C definition | Rust export | Status |
|----------|------|--------------|-------------|--------|
| `bad` | function | `c_src/src/main.c:43` | `src/lib.rs` | [x] |
| `good` | function | `c_src/src/main.c:96` | `src/lib.rs` | [x] |
| `main` | function | `c_src/src/main.c:102` | `src/lib.rs` | [x] |
| `printIntLine` | function | `c_src/src/main.c:36` | `src/lib.rs` | [x] |
| `printLine` | function | `c_src/src/main.c:28` | `src/lib.rs` | [x] |

Undefined C-library/runtime imports in the C object are `atof`, `fgets`,
`printf`, `puts`, `stdin`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`. They are
provided by libc or compiler runtime libraries and are not library API symbols.

Missing C API symbols in Rust: **0**.
