# Dynamic Symbol Surface

Derived from:

```text
nm -D c_src/build/libdriver.so
```

## Public API Definitions

| C symbol | C type | Rust definition | Status |
|----------|--------|-----------------|--------|
| `driver` | `T` | `T` | [x] |

## Complete C Dynamic Symbol Table

The remaining entries are undefined imports supplied by libc or the ELF
toolchain. They are recorded here so the artifact accounts for every line of
`nm -D`, but they are not library API definitions that Rust must export.

| Symbol | C type | Classification |
|--------|--------|----------------|
| `_ITM_deregisterTMCloneTable` | `w` | ELF toolchain weak import |
| `_ITM_registerTMCloneTable` | `w` | ELF toolchain weak import |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | libc weak import |
| `__gmon_start__` | `w` | ELF toolchain weak import |
| `driver` | `T` | library public definition |
| `printf@GLIBC_2.2.5` | `U` | libc import |

## Configuration

`Cargo.toml` has no `[features]` table and CMake defines no build option or
conditional source. The complete valid feature set is therefore:

| # | Cargo invocation | C configuration |
|---|------------------|-----------------|
| 1 | `--no-default-features` (no named features) | default |

