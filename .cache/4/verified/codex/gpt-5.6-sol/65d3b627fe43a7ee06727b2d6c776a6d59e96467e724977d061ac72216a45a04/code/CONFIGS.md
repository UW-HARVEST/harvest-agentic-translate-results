# Configuration Surface

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no options
or conditional definitions. Therefore the complete build-time matrix contains
one combination:

| # | Cargo feature combination | CMake configuration | checked |
|---|---------------------------|---------------------|---------|
| B01 | `--no-default-features --features ""` (empty set; also the default) | default | [x] |

The runtime table is derived from the `argc` branches, `strtol` calls,
`strlen`-derived boundaries, and `%.*s` output in `c_src/src/main.c`. `main` is
the only public or low-level entry point.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| C01 | `main` | `argc == 2`; empty string (`len == 0`) | [x] |
| C02 | `main` | `argc == 2`; one-byte non-NUL string (`len == 1`) | [x] |
| C03 | `main` | `argc == 2`; multi-byte non-NUL string (`len > 1`), including non-UTF-8 bytes | [x] |
| C04 | `main` | `argc == 3`; parsed `start == 0`, for empty/one/many-byte strings | [x] |
| C05 | `main` | `argc == 3`; interior `0 < start < len` | [x] |
| C06 | `main` | `argc == 3`; boundary `start == len` (empty suffix) | [x] |
| C07 | `main` | `argc == 3`; valid start with `strtol` lexical variants: leading whitespace, optional sign, or trailing nondigits | [x] |
| C08 | `main` | `argc == 3`; decimal magnitude at `long` overflow/cast boundaries whose resulting `int` is in `[0, len]` | [x] |
| C09 | `main` | `argc == 4`; one-byte slice (`start == 0`, `stop == 1`) | [x] |
| C10 | `main` | `argc == 4`; prefix slice (`start == 0`, `0 < stop < len`) | [x] |
| C11 | `main` | `argc == 4`; interior slice (`0 < start < stop < len`) | [x] |
| C12 | `main` | `argc == 4`; suffix/full-boundary slice (`0 <= start < stop == len`) | [x] |
| C13 | `main` | `argc == 4`; valid distinct numeric argument pointers with leading whitespace, optional signs, and trailing nondigits | [x] |
