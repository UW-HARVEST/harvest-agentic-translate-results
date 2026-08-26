# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no CMake
options, conditional sources, compile definitions, or preprocessor toggles.
There is exactly one valid build-time combination:

| # | Cargo arguments | CMake configuration | [ ] |
|---|-----------------|---------------------|-----|
| B01 | `--no-default-features` (empty feature set) | default, with `CMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] |

## Runtime Matrix

The only public shared-library entry point is `process_strings`. All helper
functions in `lib.c` are `static`. Rows below are the pruned cross-product of
operation dispatch, tested flag bits, length branches, and data-shape branches
that produce distinct C control flow or results. Bits other than `0x01` for
operation 2 and `0x02` for operation 4 are ignored and are varied in every
randomized row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| V01 | `process_strings` op 0 | input exactly equals arbitrary reference; empty, one-byte, and many-byte C strings; lengths and flags ignored | [x] |
| V02 | `process_strings` op 0 | input is special fallback token `"VALID"` or `"OK"` and differs from reference | [x] |
| V03 | `process_strings` op 1 | each of `START`, `STOP`, `PAUSE`, `RESUME`, `RESET`; `input_len >= command length`; NUL follows command | [x] |
| V04 | `process_strings` op 1 | each standard command followed by a space and arbitrary suffix; `input_len >= command length` | [x] |
| V05 | `process_strings` op 1 | exact standard command with `input_len < command length`, including zero; fallback `strcmp` path | [x] |
| V06 | `process_strings` op 1 | exact special command `"ADMIN"` | [x] |
| V07 | `process_strings` op 2 | exact flag clear; arbitrary reference is a prefix of equal or longer input, including empty reference | [x] |
| V08 | `process_strings` op 2 | exact flag set; input exactly equals arbitrary reference | [x] |
| V09 | `process_strings` op 2 | exact flag set; input equals reference plus each suffix `_v1`, `_v2`, `_old`, `_new`, `_tmp` | [x] |
| V10 | `process_strings` op 2 | exact flag set; reference length reaches or exceeds 63, exercising `expected[64]` truncation and zero suffix capacity | [x] |
| V11 | `process_strings` op 3 | explicit delimiter (`reference != NULL && ref_len > 0`) found at position zero, interior, or final searched byte | [x] |
| V12 | `process_strings` op 3 | default `':'` delimiter selected by `reference == NULL` | [x] |
| V13 | `process_strings` op 3 | default `':'` delimiter selected by `reference != NULL && ref_len == 0`; reference byte ignored | [x] |
| V14 | `process_strings` op 3 | embedded NUL precedes a later delimiter inside `input_len`; search stops at NUL | [x] |
| V15 | `process_strings` op 3 | arbitrary binary bytes with explicit nonzero `input_len`, including one and oversized-but-allocated lengths | [x] |
| V16 | `process_strings` op 4 | case-sensitive flag clear; exact arbitrary match, including empty string | [x] |
| V17 | `process_strings` op 4 | case-sensitive flag clear; unequal lengths and input begins with the complete reference, returning prefix result | [x] |
| V18 | `process_strings` op 4 | case-sensitive flag clear; equal lengths differ only by ASCII letter case | [x] |
| V19 | `process_strings` op 4 | case-sensitive flag set; exact arbitrary match | [x] |
| V20 | `process_strings` op 4 | case-sensitive flag set; input equals each generated wildcard form `*p*`, `p*`, `*p` | [x] |
| V21 | `process_strings` op 4 | case-sensitive flag set; reference occurs at position zero, interior, or end of a longer input | [x] |
| V22 | `process_strings` op 4 | case-sensitive flag set; reference lengths around `snprintf`'s 64-byte destination boundary exercise wildcard truncation | [x] |
| V23 | `process_strings` all ops | embedded NUL terminates C-string comparisons; supplied lengths vary independently where the operation ignores them | [x] |
| V24 | `process_strings` all ops | irrelevant flag bits clear, individually set, and all set; only operation 2 bit `0x01` and operation 4 bit `0x02` alter behavior | [x] |

Error-result branches, zero length, null pointers, unknown operation values, and
non-match shapes are enumerated separately in `ERRORS.md`.
