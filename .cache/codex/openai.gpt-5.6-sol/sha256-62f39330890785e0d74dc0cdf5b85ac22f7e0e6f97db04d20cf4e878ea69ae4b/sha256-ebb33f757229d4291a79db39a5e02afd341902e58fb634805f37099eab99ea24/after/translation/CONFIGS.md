# Configuration surface

The public API has one entry point, no options, flags, modes, element types,
length parameters, compile-time feature branches, or lower-level public
functions. Its only operation is `strcspn(s1, s2)`, so the meaningful valid
input shapes are determined by empty/nonempty strings and the location of the
first byte from `s2` in `s1`.

| # | entry point(s) | configuration (options set + input shape) | passed |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | `s1` empty; `s2` empty or nonempty | [x] |
| 2 | `driver` | `s1` nonempty; `s2` empty | [x] |
| 3 | `driver` | both nonempty; first byte of `s1` is in `s2` | [x] |
| 4 | `driver` | both nonempty; first matching byte is in the interior of `s1` | [x] |
| 5 | `driver` | both nonempty; no byte of `s1` is in `s2` | [x] |
| 6 | `driver` | both nonempty; matching/reject sets contain repeated bytes | [x] |

Every row is exercised with deterministic randomized byte strings through both
shared-library FFI exports.
