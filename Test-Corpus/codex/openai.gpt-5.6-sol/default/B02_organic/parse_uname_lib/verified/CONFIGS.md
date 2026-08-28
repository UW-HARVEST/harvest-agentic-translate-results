# Configuration Surface

The rows below are derived from the three symbols exported by the C shared
library, the `ARCHS` loop, and every `if` branch in `src/lib.c`. There are no
compile-time options, runtime flags, byte-order modes, element types, or Cargo
features.

Parser combination notation:

- `A0` / `A1`: no recognized architecture / recognized architecture in the
  mutable prefix before `" ["`.
- `P0` / `P1`: OS name has no `|platform` / has `|platform`.
- `C0` / `C1`: version has no `" (codename)"` / has it.
- `VN`, `VM`, `VMM`: non-numeric version / major only / major and minor.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C01 | `get_os_arch` | Header contains `x86_64` | [x] |
| C02 | `get_os_arch` | Header contains `i386` | [x] |
| C03 | `get_os_arch` | Header contains `i686` | [x] |
| C04 | `get_os_arch` | Header contains `sparc` | [x] |
| C05 | `get_os_arch` | Header contains `amd64` | [x] |
| C06 | `get_os_arch` | Header contains `i86pc` | [x] |
| C07 | `get_os_arch` | Header contains `ia64` | [x] |
| C08 | `get_os_arch` | Header contains `AIX` | [x] |
| C09 | `get_os_arch` | Header contains `armv6` | [x] |
| C10 | `get_os_arch` | Header contains `armv7` | [x] |
| C11 | `get_os_arch` | Header contains `aarch64` | [x] |
| C12 | `get_os_arch` | Header contains `arm64` | [x] |
| C13 | `get_os_arch` | Header contains multiple recognized strings; earliest `ARCHS` entry wins, independent of text order | [x] |
| C14 | `get_os_arch` | Empty/nonempty header contains no recognized string | [x] |
| C15 | `w_regexec` | Empty pattern and empty string, `nmatch=0`, `pmatch=NULL` | [x] |
| C16 | `w_regexec` | Valid literal pattern matches a nonempty string, `nmatch=0`, `pmatch=NULL` | [x] |
| C17 | `w_regexec` | Valid literal pattern matches, `nmatch=1`, whole-match offsets requested | [x] |
| C18 | `w_regexec` | Valid pattern with one capture matches, `nmatch=2`, whole and capture offsets requested | [x] |
| C19 | `w_regexec` | Valid pattern with fewer captures than slots matches, `nmatch>2`; unmatched slots are reported | [x] |
| C20 | `w_regexec` | Valid pattern does not match an empty/nonempty string | [x] |
| C21 | `parse_uname_string`, `w_regexec` | Windows marker; version is non-numeric, so major/minor/build do not match | [x] |
| C22 | `parse_uname_string`, `w_regexec` | Windows marker; version has major only | [x] |
| C23 | `parse_uname_string`, `w_regexec` | Windows marker; version has major.minor only | [x] |
| C24 | `parse_uname_string`, `w_regexec` | Windows marker; version has major.minor.single-build | [x] |
| C25 | `parse_uname_string`, `w_regexec` | Windows marker; version has major.minor.multi-part-build | [x] |
| C26 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `x86_64` | [x] |
| C27 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `i386` | [x] |
| C28 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `i686` | [x] |
| C29 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `sparc` | [x] |
| C30 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `amd64` | [x] |
| C31 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `i86pc` | [x] |
| C32 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `ia64` | [x] |
| C33 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `AIX` | [x] |
| C34 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `armv6` | [x] |
| C35 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `armv7` | [x] |
| C36 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `aarch64` | [x] |
| C37 | `parse_uname_string`, `get_os_arch` | No `" ["` marker; prefix contains `arm64` | [x] |
| C38 | `parse_uname_string`, `get_os_arch` | No `" ["` marker and no recognized architecture | [x] |
| C39 | `parse_uname_string`, `get_os_arch` | Non-Windows marker without `": "`: `P0 A0` | [x] |
| C40 | `parse_uname_string`, `get_os_arch` | Non-Windows marker without `": "`: `P0 A1` | [x] |
| C41 | `parse_uname_string`, `get_os_arch` | Non-Windows marker without `": "`: `P1 A0` | [x] |
| C42 | `parse_uname_string`, `get_os_arch` | Non-Windows marker without `": "`: `P1 A1` | [x] |
| C43 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C0 P0 A0` | [x] |
| C44 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C0 P0 A1` | [x] |
| C45 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C0 P1 A0` | [x] |
| C46 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C0 P1 A1` | [x] |
| C47 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C1 P0 A0` | [x] |
| C48 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C1 P0 A1` | [x] |
| C49 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C1 P1 A0` | [x] |
| C50 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VN C1 P1 A1` | [x] |
| C51 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C0 P0 A0` | [x] |
| C52 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C0 P0 A1` | [x] |
| C53 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C0 P1 A0` | [x] |
| C54 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C0 P1 A1` | [x] |
| C55 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C1 P0 A0` | [x] |
| C56 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C1 P0 A1` | [x] |
| C57 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C1 P1 A0` | [x] |
| C58 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VM C1 P1 A1` | [x] |
| C59 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C0 P0 A0` | [x] |
| C60 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C0 P0 A1` | [x] |
| C61 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C0 P1 A0` | [x] |
| C62 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C0 P1 A1` | [x] |
| C63 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C1 P0 A0` | [x] |
| C64 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C1 P0 A1` | [x] |
| C65 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C1 P1 A0` | [x] |
| C66 | `parse_uname_string`, `w_regexec`, `get_os_arch` | Non-Windows marker with `": "`: `VMM C1 P1 A1` | [x] |
