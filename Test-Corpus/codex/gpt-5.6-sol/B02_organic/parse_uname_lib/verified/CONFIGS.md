# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and therefore has no named or default
features. `c_src/CMakeLists.txt` has no options, conditional source lists,
compile definitions, or backend selectors.

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| B01 | `cargo ... --no-default-features` (no `--features` value) | default target with `CMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] `cargo check` + differential tests |

This is the complete set of valid feature combinations: **one (the empty
set)**.

## Runtime Configurations

Rows come from every branch in the three exported C functions. Randomized
cases vary prefixes, suffixes, digit lengths, and surrounding text while
preserving the listed branch condition.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|-------------------------------------------|--------|
| C01 | `get_os_arch` | one `x86_64` occurrence | [x] |
| C02 | `get_os_arch` | one `i386` occurrence | [x] |
| C03 | `get_os_arch` | one `i686` occurrence | [x] |
| C04 | `get_os_arch` | one `sparc` occurrence | [x] |
| C05 | `get_os_arch` | one `amd64` occurrence | [x] |
| C06 | `get_os_arch` | one `i86pc` occurrence | [x] |
| C07 | `get_os_arch` | one `ia64` occurrence | [x] |
| C08 | `get_os_arch` | one `AIX` occurrence (case-sensitive) | [x] |
| C09 | `get_os_arch` | one `armv6` occurrence | [x] |
| C10 | `get_os_arch` | one `armv7` occurrence | [x] |
| C11 | `get_os_arch` | one `aarch64` occurrence | [x] |
| C12 | `get_os_arch` | one `arm64` occurrence | [x] |
| C13 | `get_os_arch` | two or more supported strings; table order, not input order, selects the result | [x] |
| C14 | `w_regexec` | successful literal match, `nmatch == 0`, `pmatch == NULL` | [x] |
| C15 | `w_regexec` | successful match, `nmatch == 1`, full-match slot populated | [x] |
| C16 | `w_regexec` | successful match with one capture, `nmatch == 2` | [x] |
| C17 | `w_regexec` | more captures than supplied match slots | [x] |
| C18 | `w_regexec` | more supplied match slots than captures; unused slots become `{-1, -1}` | [x] |
| C19 | `w_regexec` | extended-regex alternation, grouping, character classes, and repetition | [x] |
| C20 | `parse_uname_string`, `w_regexec` | Windows marker ` [Ver: ` with a major-only numeric version | [x] |
| C21 | `parse_uname_string`, `w_regexec` | Windows marker with `major.minor` | [x] |
| C22 | `parse_uname_string`, `w_regexec` | Windows marker with `major.minor.build` | [x] |
| C23 | `parse_uname_string`, `w_regexec` | Windows marker with a multi-component build (`major.minor.build.more`) | [x] |
| C24 | `parse_uname_string`, `w_regexec` | Windows marker with a nonnumeric version | [x] |
| C25 | `parse_uname_string`, `w_regexec` | bracketed name/version (`name: major]`), no codename/platform/architecture | [x] |
| C26 | `parse_uname_string`, `w_regexec` | bracketed name/version (`name: major.minor]`), no codename | [x] |
| C27 | `parse_uname_string`, `w_regexec` | bracketed numeric version with ` (codename)` | [x] |
| C28 | `parse_uname_string`, `w_regexec` | bracketed nonnumeric version | [x] |
| C29 | `parse_uname_string`, `w_regexec` | bracketed `name|platform: version]`; pipe precedes colon and sets platform | [x] |
| C30 | `parse_uname_string`, `w_regexec` | bracketed `name: version|text]`; colon truncation hides pipe from platform parsing | [x] |
| C31 | `parse_uname_string` | bracketed name with no `: ` and no pipe; final byte is trimmed | [x] |
| C32 | `parse_uname_string` | bracketed `name|platform]` with no `: `; final byte is trimmed before pipe split | [x] |
| C33 | `parse_uname_string`, `get_os_arch` | bracketed parse plus supported architecture in the pre-bracket prefix | [x] |
| C34 | `parse_uname_string`, `get_os_arch` | no bracket and `x86_64` in the input | [x] |
| C35 | `parse_uname_string`, `get_os_arch` | no bracket and `i386` in the input | [x] |
| C36 | `parse_uname_string`, `get_os_arch` | no bracket and `i686` in the input | [x] |
| C37 | `parse_uname_string`, `get_os_arch` | no bracket and `sparc` in the input | [x] |
| C38 | `parse_uname_string`, `get_os_arch` | no bracket and `amd64` in the input | [x] |
| C39 | `parse_uname_string`, `get_os_arch` | no bracket and `i86pc` in the input | [x] |
| C40 | `parse_uname_string`, `get_os_arch` | no bracket and `ia64` in the input | [x] |
| C41 | `parse_uname_string`, `get_os_arch` | no bracket and `AIX` in the input | [x] |
| C42 | `parse_uname_string`, `get_os_arch` | no bracket and `armv6` in the input | [x] |
| C43 | `parse_uname_string`, `get_os_arch` | no bracket and `armv7` in the input | [x] |
| C44 | `parse_uname_string`, `get_os_arch` | no bracket and `aarch64` in the input | [x] |
| C45 | `parse_uname_string`, `get_os_arch` | no bracket and `arm64` in the input | [x] |
| C46 | `parse_uname_string`, `get_os_arch` | no bracket and no supported architecture (including empty input) | [x] |
