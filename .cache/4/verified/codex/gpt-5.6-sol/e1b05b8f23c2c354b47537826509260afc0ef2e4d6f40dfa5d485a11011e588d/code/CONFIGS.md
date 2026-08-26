# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and the CMake file has no options or
preprocessor configuration branches. There is exactly one valid build
configuration:

| # | Cargo feature combination | C configuration | [ ] |
|---|---------------------------|-----------------|-----|
| 1 | `--no-default-features` (empty feature set) | default CMake configuration | [x] |

## Runtime and Input Configurations

The public API has no modes, options, flags, enums, formats, or byte-order
settings. Its only element type is the platform C `wchar_t` (32-bit on this
target). The rows below enumerate the empty/one/many loop shapes and the
capacity boundary that the C source treats differently. "Exact" means
`numElem == dst_length + src_length + 1`; "spare" means a larger `numElem`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `wcscat` | no options; empty destination, empty source, exact capacity `1` | [x] |
| 2 | `wcscat` | no options; empty destination, empty source, spare capacity | [x] |
| 3 | `wcscat` | no options; empty destination, one-element source, exact capacity | [x] |
| 4 | `wcscat` | no options; empty destination, many-element source, exact capacity | [x] |
| 5 | `wcscat` | no options; empty destination, nonempty source, spare capacity | [x] |
| 6 | `wcscat` | no options; one-element destination, empty source, exact capacity | [x] |
| 7 | `wcscat` | no options; many-element destination, empty source, exact capacity | [x] |
| 8 | `wcscat` | no options; nonempty destination, empty source, spare capacity | [x] |
| 9 | `wcscat` | no options; one-element destination, one-element source, exact capacity | [x] |
| 10 | `wcscat` | no options; one-element destination, many-element source, exact capacity | [x] |
| 11 | `wcscat` | no options; many-element destination, one-element source, exact capacity | [x] |
| 12 | `wcscat` | no options; many-element destination, many-element source, exact capacity | [x] |
| 13 | `wcscat` | no options; nonempty destination and source (one or many), spare capacity | [x] |

The sole low-level entry point and the full public API are both `wcscat`; no
convenience wrappers or composed pipelines exist.
