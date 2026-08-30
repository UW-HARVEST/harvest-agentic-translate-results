# Configuration Surface

The public dynamic surface contains two entry points. The C source has no
runtime options, flags, modes, conditionals, switches, feature conditionals,
or variable-size inputs. The only input-shape axis is the full set of 256
possible `char` bit patterns.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printHexCharLine` | no options; one by-value `char`; all 256 bit patterns | [x] |
| 2 | `driver` | no options; one by-value `char`; all 256 bit patterns, including wrapping `0x7f + 1` | [x] |
