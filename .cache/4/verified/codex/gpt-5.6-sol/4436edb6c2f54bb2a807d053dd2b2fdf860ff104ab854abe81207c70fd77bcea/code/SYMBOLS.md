# Dynamic Symbol Surface

Source: `nm -D --defined-only c_src/build/libtranslated_rust.so`.

The C shared object is ELF64 for x86-64. Toolchain/runtime entries and undefined
libc references are not library API symbols.

| C symbol | Type | Rust export |
|----------|------|-------------|
| `siphash` | `T` | [x] |
| `stbds_hash_bytes` | `T` | [x] |

## Feature Combinations

`Cargo.toml` has no `[features]` table and therefore has one valid
configuration:

| # | Default features | Explicit features | `cargo` arguments | [ ] |
|---|------------------|-------------------|-------------------|-----|
| 1 | disabled | none | `--no-default-features --features ""` | [x] |
