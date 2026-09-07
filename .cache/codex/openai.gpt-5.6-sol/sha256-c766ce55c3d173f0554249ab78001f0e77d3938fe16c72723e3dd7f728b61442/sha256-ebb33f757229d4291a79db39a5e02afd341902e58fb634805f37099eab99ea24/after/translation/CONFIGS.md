# Configuration surface

The rows below come from the public entry points found by `nm -D`, the `ARCHS`
table, and each `if`/regex branch in `../c_src/src/lib.c`. The API has no
compile-time Cargo features and no runtime option object; its configuration
axes are regex match shape, architecture-token precedence, and uname format.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|-----|
| 1 | `get_os_arch` | input contains `x86_64` | [x] |
| 2 | `get_os_arch` | input contains `i386` | [x] |
| 3 | `get_os_arch` | input contains `i686` | [x] |
| 4 | `get_os_arch` | input contains `sparc` | [x] |
| 5 | `get_os_arch` | input contains `amd64` | [x] |
| 6 | `get_os_arch` | input contains `i86pc` | [x] |
| 7 | `get_os_arch` | input contains `ia64` | [x] |
| 8 | `get_os_arch` | input contains `AIX` | [x] |
| 9 | `get_os_arch` | input contains `armv6` | [x] |
| 10 | `get_os_arch` | input contains `armv7` | [x] |
| 11 | `get_os_arch` | input contains `aarch64` | [x] |
| 12 | `get_os_arch` | input contains `arm64` | [x] |
| 13 | `get_os_arch` | input contains multiple supported tokens; first `ARCHS` table entry wins, independent of textual position | [x] |
| 14 | `w_regexec` | valid regex matches; `nmatch == 0`, `pmatch == NULL` | [x] |
| 15 | `w_regexec` | valid regex matches; `nmatch == 1`, full-match offsets requested | [x] |
| 16 | `w_regexec` | valid regex with a capture matches; `nmatch == 2`, subgroup offsets requested | [x] |
| 17 | `w_regexec` | valid regex does not match | [x] |
| 18 | `w_regexec` | empty valid regex and empty input string | [x] |
| 19 | `w_regexec` | valid regex matches with a large safely backed `nmatch` array | [x] |
| 20 | `parse_uname_string`, `w_regexec` | Windows ` [Ver: ...]` format with major-only numeric version | [x] |
| 21 | `parse_uname_string`, `w_regexec` | Windows format with major.minor version | [x] |
| 22 | `parse_uname_string`, `w_regexec` | Windows format with major.minor.build version | [x] |
| 23 | `parse_uname_string`, `w_regexec` | Windows format with dotted multi-component build | [x] |
| 24 | `parse_uname_string`, `w_regexec` | Windows format with nonnumeric/empty version; no numeric captures | [x] |
| 25 | `parse_uname_string` | generic ` [...]` format, no `: `, no `|`; closing byte is removed from `os_name` | [x] |
| 26 | `parse_uname_string` | generic format, no `: `, with `name|platform`; name/platform split after closing-byte removal | [x] |
| 27 | `parse_uname_string`, `w_regexec` | generic `name: version` with major-only numeric version, no codename, no platform | [x] |
| 28 | `parse_uname_string`, `w_regexec` | generic `name: major.minor` version, no codename, no platform | [x] |
| 29 | `parse_uname_string`, `w_regexec` | generic numeric version with ` (codename)` | [x] |
| 30 | `parse_uname_string`, `w_regexec` | generic nonnumeric version with ` (codename)`; codename set but numeric captures absent | [x] |
| 31 | `parse_uname_string`, `w_regexec` | generic `name|platform: version`; platform split combines with version parsing | [x] |
| 32 | `parse_uname_string`, `get_os_arch` | no bracket marker and one supported architecture token; only `os_arch` is set | [x] |
| 33 | `parse_uname_string`, `get_os_arch` | no bracket marker and no supported architecture token; all fields remain null | [x] |
| 34 | `parse_uname_string`, `get_os_arch` | generic marker with supported architecture in the prefix before ` [`; architecture survives input truncation | [x] |
| 35 | `parse_uname_string`, `get_os_arch` | generic marker with supported architecture only after ` [`; architecture is not found after input truncation | [x] |
| 36 | `parse_uname_string` | Windows marker takes precedence over generic marker handling and skips architecture extraction | [x] |
| 37 | `parse_uname_string` | empty input string (zero-length shape), no marker, no architecture | [x] |
| 38 | `parse_uname_string` | one-byte input string, no marker, no architecture | [x] |
