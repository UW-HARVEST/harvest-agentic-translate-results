# Configuration Surface

The header exposes one entry point and no runtime options, modes, flags,
element types, byte-order settings, feature macros, or lower-level public
functions. The only input axis is the shape of the two NUL-terminated byte
strings consumed by `strcspn`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `s1` logically empty; randomized empty/nonempty rejection sets | [x] |
| 2 | `driver` | nonempty `s1`; logically empty `s2`, so the full `s1` length is printed | [x] |
| 3 | `driver` | nonempty strings; the first byte of `s1` occurs in `s2` | [x] |
| 4 | `driver` | nonempty strings; the first rejected byte occurs after a nonempty prefix | [x] |
| 5 | `driver` | nonempty strings; no byte of `s1` occurs in `s2` | [x] |
| 6 | `driver` | backing arrays contain bytes after the first NUL; trailing bytes are ignored | [x] |
| 7 | `driver` | strings contain non-ASCII bytes in the range `0x80..=0xff` | [x] |
| 8 | `driver` | long strings and rejection sets exercise size-dependent libc paths | [x] |

Every row is exercised with many deterministic randomized cases through both
shared-library FFI boundaries.
