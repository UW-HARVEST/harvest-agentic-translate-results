# Dynamic symbol surface

Generated from:

```text
nm -D ../c_src/build/libdriver.so
nm -D target/release/libdriver.so
```

## C-defined public symbols

| symbol | C type | Rust status |
|---|---:|---|
| `bad` | `T` | exported with exact name |
| `driver` | `T` | exported with exact name |
| `good` | `T` | exported with exact name |
| `printHexCharLine` | `T` | exported with exact name |
| `printLine` | `T` | exported with exact name |

Defined-symbol comparison:

```text
comm -23 <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort) \
         <(nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort)
```

Result: empty (0 C-defined symbols missing from Rust).

## Other entries in the C dynamic symbol table

These are runtime/libc dependencies rather than symbols defined by the C
library. Each also occurs in the Rust library's dynamic symbol table.

| symbol | C type | Rust dynamic table |
|---|---:|---|
| `_ITM_deregisterTMCloneTable` | `w` | present |
| `_ITM_registerTMCloneTable` | `w` | present |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | present |
| `__gmon_start__` | `w` | present |
| `printf@GLIBC_2.2.5` | `U` | present |
| `puts@GLIBC_2.2.5` | `U` | present |

## Feature configurations

`Cargo.toml` has no `[features]` table. The only semantic configuration is the
feature-empty build. Verification runs it both normally and with
`--no-default-features`.

- [x] default invocation
- [x] `--no-default-features`
