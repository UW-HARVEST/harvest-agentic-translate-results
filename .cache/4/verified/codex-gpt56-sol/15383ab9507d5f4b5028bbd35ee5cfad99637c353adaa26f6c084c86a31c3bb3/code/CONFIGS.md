# Configuration Surface

## Build-Time Matrix

`Cargo.toml` has no `[features]` section and CMake defines no options or
conditional compilation. There is exactly one valid feature combination:

| # | Cargo feature combination | CMake configuration | |
|---|---------------------------|---------------------|-|
| 1 | `--no-default-features` (empty set) | default | [x] |

## Runtime Matrix

The sole public and lowest-level entry point is `searchAndReplace`. The rows
below cover every branch axis in `c_src/src/lib.c`: first match absent/present;
prefix absent/present; later match absent/present; inter-match gap
absent/present; suffix absent/present; empty/nonempty replacement; and
non-overlapping treatment of an overlapping candidate. Relative replacement
lengths exercise the allocation arithmetic even though they do not introduce
an additional `if`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `searchAndReplace` | Empty `orig`, nonempty `search`: no match; empty and nonempty `value` variants | [x] |
| 2 | `searchAndReplace` | Nonempty `orig`, absent `search`: `strdup` path; replacement shorter/equal/longer than search | [x] |
| 3 | `searchAndReplace` | Exactly one match spanning all of `orig`: no prefix, gap, or suffix; empty replacement | [x] |
| 4 | `searchAndReplace` | Exactly one match spanning all of `orig`: nonempty replacement shorter/equal/longer than search | [x] |
| 5 | `searchAndReplace` | Exactly one match at the start with a nonempty suffix | [x] |
| 6 | `searchAndReplace` | Exactly one match in the middle with nonempty prefix and suffix | [x] |
| 7 | `searchAndReplace` | Exactly one match at the end with a nonempty prefix and no suffix | [x] |
| 8 | `searchAndReplace` | Multiple adjacent matches starting at byte 0, no inter-match gap, no suffix | [x] |
| 9 | `searchAndReplace` | Multiple adjacent matches after a nonempty prefix, no inter-match gap, no suffix | [x] |
| 10 | `searchAndReplace` | Multiple separated matches starting at byte 0, nonempty inter-match gap, no suffix | [x] |
| 11 | `searchAndReplace` | Multiple separated matches after a nonempty prefix, nonempty inter-match gap, no suffix | [x] |
| 12 | `searchAndReplace` | Multiple separated matches starting at byte 0 with a nonempty suffix | [x] |
| 13 | `searchAndReplace` | Multiple separated matches after a nonempty prefix with a nonempty suffix | [x] |
| 14 | `searchAndReplace` | Overlapping candidate occurrences: `strstr` resumes at `match + search_len`, so only non-overlapping matches are replaced | [x] |
| 15 | `searchAndReplace` | Empty `search` with empty or nonempty `orig`: `strstr` repeatedly returns the same position and the function does not terminate | [x] |

All strings are C strings, so embedded NUL bytes terminate the effective input.
Randomized tests include bytes after an early NUL to verify that effective
shape. The API exposes no runtime options, modes, flags, element types, integer
widths, formats, or byte-order controls.
