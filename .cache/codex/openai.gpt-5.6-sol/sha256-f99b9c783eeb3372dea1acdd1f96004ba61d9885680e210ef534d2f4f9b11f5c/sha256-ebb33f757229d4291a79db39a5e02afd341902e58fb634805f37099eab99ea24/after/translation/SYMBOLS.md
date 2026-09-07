# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only ../c_src/build/libhello.so
nm -D --defined-only target/release/libhello.so
```

| C symbol | C type | Rust type | Rust export present |
|----------|--------|-----------|---------------------|
| `helloworld` | `T` | `T` | [x] |

The full C `nm -D` output also contains undefined/weak runtime entries
(`puts@GLIBC_2.2.5`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, and
`__gmon_start__`). They are libc/toolchain imports rather than library exports
and are therefore outside export-name parity.

Final sorted export diff: zero missing and zero extra symbols. `ldd -r` reports
no unresolved relocations for the Rust shared library.

Completion: [x] zero C-defined dynamic symbols are missing from Rust.
