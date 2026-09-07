# Configuration-Surface Table

Mechanically derived from the public header and the scan/copy branches in
`c_src/src/lib.c`. There are no runtime options, modes, flags, compile-time
feature branches, alternate element types, or additional public entry points.
The element type is the platform `wchar_t` (signed 32-bit on this target).

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `wcscat` | empty destination; empty source; one-element minimum buffer, so the copied source NUL exactly fills the buffer | [x] |
| 2 | `wcscat` | empty destination; nonempty source; source NUL lands before the final destination element (spare capacity remains) | [x] |
| 3 | `wcscat` | empty destination; nonempty source; source NUL lands exactly in the final destination element | [x] |
| 4 | `wcscat` | nonempty terminated destination; empty source; exercise interior and final-slot destination terminators | [x] |
| 5 | `wcscat` | nonempty terminated destination; nonempty source; source NUL lands before the final destination element | [x] |
| 6 | `wcscat` | nonempty terminated destination; nonempty source; source NUL lands exactly in the final destination element | [x] |

Every row must be exercised through both shared-library ABIs with fixed-seed
randomized `wchar_t` values, including negative and high-bit-set values.
