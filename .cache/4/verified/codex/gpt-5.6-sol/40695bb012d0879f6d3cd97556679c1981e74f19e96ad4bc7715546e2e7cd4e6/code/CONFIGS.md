# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no CMake
options or conditional source selection. The complete valid feature set is:

| # | Cargo invocation feature selection | C configuration |
|---|------------------------------------|-----------------|
| 1 | `--no-default-features` (empty feature set) | default/unconditional |

## Runtime Configurations

The public header exposes one entry point, no options or modes, and one fixed
input/output shape. The C implementation has no data-dependent branches.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `md5_digest` | no options; one aligned `tflac_md5` containing four independent `uint32_t` values; writable `uint8_t[16]` output | [x] |
