# Configuration Surface

The public API has one entry point and no runtime modes, flags, build features,
or element-type choices. The C loops distinguish destination and source
strings of length zero, one, or many. For every pair, the successful capacity
boundary is either exact (`dst_len + src_len + 1`) or has spare elements.
Nonzero `wchar_t` values are randomized across their signed 32-bit range.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `wcscat` | empty destination + empty source; exact capacity `1` | [x] |
| 2 | `wcscat` | empty destination + empty source; spare capacity | [x] |
| 3 | `wcscat` | empty destination + one-element source; exact capacity `2` | [x] |
| 4 | `wcscat` | empty destination + one-element source; spare capacity | [x] |
| 5 | `wcscat` | empty destination + many-element source; exact capacity | [x] |
| 6 | `wcscat` | empty destination + many-element source; spare capacity | [x] |
| 7 | `wcscat` | one-element destination + empty source; exact capacity `2` | [x] |
| 8 | `wcscat` | one-element destination + empty source; spare capacity | [x] |
| 9 | `wcscat` | one-element destination + one-element source; exact capacity `3` | [x] |
| 10 | `wcscat` | one-element destination + one-element source; spare capacity | [x] |
| 11 | `wcscat` | one-element destination + many-element source; exact capacity | [x] |
| 12 | `wcscat` | one-element destination + many-element source; spare capacity | [x] |
| 13 | `wcscat` | many-element destination + empty source; exact capacity | [x] |
| 14 | `wcscat` | many-element destination + empty source; spare capacity | [x] |
| 15 | `wcscat` | many-element destination + one-element source; exact capacity | [x] |
| 16 | `wcscat` | many-element destination + one-element source; spare capacity | [x] |
| 17 | `wcscat` | many-element destination + many-element source; exact capacity | [x] |
| 18 | `wcscat` | many-element destination + many-element source; spare capacity | [x] |

The insufficient-capacity and unterminated-destination shapes end in rejection
and are therefore enumerated in `ERRORS.md`, not duplicated here.
