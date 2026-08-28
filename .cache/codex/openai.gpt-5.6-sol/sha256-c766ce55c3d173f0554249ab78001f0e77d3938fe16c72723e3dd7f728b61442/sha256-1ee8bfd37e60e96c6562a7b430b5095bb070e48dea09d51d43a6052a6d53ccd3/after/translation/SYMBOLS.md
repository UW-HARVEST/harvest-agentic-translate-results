# Dynamic Symbol Surface

Source library: `../c_src/build/libdriver.so`

Command:

```text
nm -D --defined-only ../c_src/build/libdriver.so
```

| C symbol | C type | Rust export | Status |
|----------|--------|-------------|--------|
| `get_os_arch` | `T` | `get_os_arch` | [x] |
| `parse_uname_string` | `T` | `parse_uname_string` | [x] |
| `w_regexec` | `T` | `w_regexec` | [x] |

The C library's full `nm -D` output also contains only undefined GLIBC
functions/data and the usual weak ELF runtime hooks. The Rust library's
undefined references are versioned GLIBC or GCC unwind runtime symbols plus
weak ELF runtime hooks. Its `DT_NEEDED` entries resolve to `libc.so.6`,
`libgcc_s.so.1`, and `ld-linux-x86-64.so.2`; it has no unresolved application
or C-library API symbol.

## Feature Matrix

`Cargo.toml` declares no features, so the complete matrix is:

| configuration | release `.so` rebuilt | full differential suite |
|---------------|-----------------------|-------------------------|
| Default | [x] | [x] 8 passed |
| `--no-default-features` | [x] | [x] 8 passed |

Completion check:

- [x] Every C-defined dynamic symbol is present under the exact name in
  `target/release/libdriver.so`.
- [x] Missing C-defined symbols: 0.
- [x] Undefined non-runtime application symbols: 0.
