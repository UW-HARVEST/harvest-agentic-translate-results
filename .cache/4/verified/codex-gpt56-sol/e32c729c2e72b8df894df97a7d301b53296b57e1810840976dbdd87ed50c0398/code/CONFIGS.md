# Configuration Surface

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
options, conditional definitions, or conditional sources. Therefore there is
one valid feature configuration:

| # | Cargo arguments | CMake configuration | |
|---|-----------------|---------------------|-|
| F1 | `--no-default-features` (empty feature set; also the default) | default, with position-independent code enabled | [x] |

The runtime rows below are derived from both externally linked entry points,
the `strtol` acceptance condition at `driver.c:64`, the two-call composition at
`driver.c:76-77`, integer additions in `run`, and `%d` / `%.1f` output
formatting.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| C1 | `run` | Direct low-level call; ordinary in-range floor/bedroom/extra values and finite bathroom values; compare all four output lines and final struct bytes | [x] |
| C2 | `run` | Direct low-level call; floor or bedroom addition crosses an `int` representation boundary; compare C's observed wrapped output and final struct bytes | [x] |
| C3 | `run` | Direct low-level call; bathroom is signed zero, infinity, or NaN (including varied NaN payloads); compare formatted output and final struct bytes | [x] |
| C4 | `driver` | Canonical base-10 input with value in `INT_MIN..=INT_MAX`; randomized signs, magnitudes, and zero; exercise both composed `run` calls | [x] |
| C5 | `driver` | Accepted noncanonical input: leading C whitespace and/or explicit sign, with optional trailing nondigit bytes because only `endp != str` is required | [x] |
| C6 | `driver` | Accepted values at and near `INT_MIN` / `INT_MAX`; exercise representation-boundary additions across both composed `run` calls | [x] |

There are no runtime options, modes, flags, element counts, byte-order
settings, public formats, or alternate public element types.
