# Error Surface

Mechanically derived from every conditional return in `c_src/src/lib.c`.
There are no assertions, error macros, enums, null checks, explicit length
checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `ima_parse` | `ima_btoh32(header->type) != tag('f','f','a','c')` | returns `-1`; `info` is unchanged | [x] |
| 2 | `ima_parse` | file type is valid and `ima_btoh16(header->version) != 1` | returns `-2`; `info` is unchanged | [x] |
| 3 | `ima_parse` | header/version are valid, a data chunk is reached, and `ima_btoh32(desc->format_id) != tag('4','a','m','i')` | returns `-3`; `info` is unchanged | [x] |

## Boundary Audit

`ima_parse` has no length argument. A null `data` pointer is dereferenced at
line 87, and a null `info` pointer is dereferenced only on the success path at
line 123. Missing description or packet-table chunks likewise cause null
dereferences; missing data causes an unbounded scan. These are undefined
behavior in C, not defined error/rejection results, so they are not rows in the
error-return table. The differential suite probes null pointers in isolated
child processes so a fault cannot terminate the test runner.

Zero, oversized, and one-past-valid lengths are inapplicable because the public
API accepts no length. There are no public C enums, so out-of-range enum values
are also inapplicable.
