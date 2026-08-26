# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` has no
options or conditional compilation. There is exactly one valid feature
combination:

| # | Cargo invocation | C configuration |
|---|------------------|-----------------|
| F01 | `--no-default-features` (empty feature set) | default CMake configuration |

## Runtime Configurations

These rows come from the public/exported entry points and every distinct branch
in `valid_1`, `valid_2`, `valid_3`, `valid_4`, `w_utf8_drop`, and
`w_utf8_filter`. Inputs are NUL-terminated byte strings and therefore cannot
contain an embedded NUL.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| C01 | `w_utf8_drop` | empty input; loop is skipped and the terminator pointer is returned | [x] |
| C02 | `w_utf8_drop` | one/many 1-byte characters, including ASCII boundary `0x7f` | [x] |
| C03 | `w_utf8_drop` | one/many 2-byte characters, including `c2 80` and `df bf` boundaries | [x] |
| C04 | `w_utf8_drop` | ordinary 3-byte characters with starts `e1..ec` or `ee` | [x] |
| C05 | `w_utf8_drop` | `e0` 3-byte characters at the second-byte lower boundary `a0` and upper boundary `bf` | [x] |
| C06 | `w_utf8_drop` | `ed` 3-byte characters at the valid second-byte boundaries `80` and `9f` | [x] |
| C07 | `w_utf8_drop` | `ef` 3-byte characters, exercising its explicit second-byte `<= bf` condition | [x] |
| C08 | `w_utf8_drop` | ordinary 4-byte characters with starts `f1..f3` | [x] |
| C09 | `w_utf8_drop` | `f0` 4-byte characters at the second-byte lower boundary `90` and upper boundary `bf` | [x] |
| C10 | `w_utf8_drop` | `f4` 4-byte characters at the valid second-byte boundaries `80` and `8f` | [x] |
| C11 | `w_utf8_drop` | invalid lead at offset zero: continuation byte, overlong lead `c0/c1`, or out-of-range lead `f5..ff` | [x] |
| C12 | `w_utf8_drop` | invalid byte after a nonempty mixed valid prefix; returns the exact first-invalid offset | [x] |
| C13 | `w_utf8_drop` | malformed 2/3/4-byte candidate: bad continuation, overlong 3/4-byte form, surrogate form, or code point above `U+10ffff` | [x] |
| C14 | `w_utf8_filter` | wholly valid empty/one/many mixed-width input; both replacement flag values take the `strdup` path | [x] |
| C15 | `w_utf8_filter` | `replacement == false`; one/many invalid bytes at start/middle/end are deleted | [x] |
| C16 | `w_utf8_filter` | `replacement == false`; valid 1/2/3/4-byte sequences after the first invalid byte exercise all copy-loop widths | [x] |
| C17 | `w_utf8_filter` | `replacement == true`; one invalid byte at start/middle/end becomes one `ef bf bd` sequence | [x] |
| C18 | `w_utf8_filter` | `replacement == true`; valid 1/2/3/4-byte sequences after the first invalid byte exercise all copy-loop widths | [x] |
| C19 | `w_utf8_filter` | `replacement == true`; consecutive and interleaved invalid bytes, from one through 1365 replacements, use the first 4096-byte growth reserve | [x] |
| C20 | `w_utf8_filter` | `replacement == true`; at least 1366 invalid bytes exhaust the reserve and take a later `realloc` branch | [x] |
