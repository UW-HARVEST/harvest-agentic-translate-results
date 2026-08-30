# Configuration Surface

Mechanical scan scope: the public header and all C implementation branches.
There are no runtime options, modes, flags, state, preprocessor feature
branches, or input-shape branches. The sole public entry point accepts two
by-value C `int` values and always prints one decimal integer plus a newline.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|---|---|---|
| 1 | `driver(int x, int y)` | No options; full `(x, y)` C `int` domain. Exercise `INT_MIN`, `INT_MAX`, `-1`, `0`, `1`, mixed-sign pairs, and many fixed-seed randomized pairs. Compare captured stdout byte-for-byte. | [x] |

There are no lower-level, convenience, one-shot, or state-management entry
points beyond `driver`.
