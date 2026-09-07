# Configuration surface

Mechanically derived from the complete public header, the complete dynamic
symbol list, and every branch and data-shape special case in
`../c_src/src/driver.c`. The library has no Cargo/CMake feature flags, runtime
options, modes, enums, byte-order choices, or executable driver.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| CFG-001 | `printLine` | Empty NUL-terminated C string (`line[0] == '\0'`); writes one newline. | [x] |
| CFG-002 | `printLine` | Non-empty NUL-terminated C string; writes all bytes through the first NUL, then one newline. | [x] |
| CFG-003 | `driver` → `printLine` | `data == 0`; copy branch executes with zero bytes and writes one newline. | [x] |
| CFG-004 | `driver` → `printLine` | `1 <= data <= 98`; copy branch writes exactly `data` `A` bytes plus newline. | [x] |
| CFG-005 | `driver` → `printLine` | `data == 99`; maximum defined copy writes 99 `A` bytes plus newline. | [x] |
| CFG-006 | `driver` → `printLine` | `data == 100`; range-check boundary skips the copy and writes one newline. | [x] |
| CFG-007 | `driver` → `printLine` | `101 <= data <= INT_MAX`; copy remains skipped and one newline is written. | [x] |

Negative `driver` values are excluded from the valid configuration table
because the C ground truth has undefined behavior for them rather than a
defined output.
