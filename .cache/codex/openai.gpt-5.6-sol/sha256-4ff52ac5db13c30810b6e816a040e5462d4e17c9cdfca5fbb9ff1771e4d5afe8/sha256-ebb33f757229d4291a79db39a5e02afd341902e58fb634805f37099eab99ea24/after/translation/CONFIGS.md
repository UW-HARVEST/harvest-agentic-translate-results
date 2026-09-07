# Configuration Surface

Mechanical branch inventory:

- Public implemented/exported entry points: `call_predict`.
- Runtime selector: integer `pfcn`.
- Valid selector values: each of `0` through `11` has a distinct `switch`
  branch and compares the selected function pointer with a different static
  predictor.
- Input shapes, buffers, element types, byte order, flags, and mutable state:
  none cross the public FFI boundary.
- Cargo features: none.
- Binary targets: none.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `call_predict` | `pfcn = 0`; no other input | [x] |
| 2 | `call_predict` | `pfcn = 1`; no other input | [x] |
| 3 | `call_predict` | `pfcn = 2`; no other input | [x] |
| 4 | `call_predict` | `pfcn = 3`; no other input | [x] |
| 5 | `call_predict` | `pfcn = 4`; no other input | [x] |
| 6 | `call_predict` | `pfcn = 5`; no other input | [x] |
| 7 | `call_predict` | `pfcn = 6`; no other input | [x] |
| 8 | `call_predict` | `pfcn = 7`; no other input | [x] |
| 9 | `call_predict` | `pfcn = 8`; no other input | [x] |
| 10 | `call_predict` | `pfcn = 9`; no other input | [x] |
| 11 | `call_predict` | `pfcn = 10`; no other input | [x] |
| 12 | `call_predict` | `pfcn = 11`; no other input | [x] |

The invalid/default selector branch is tracked in `ERRORS.md`, not duplicated
as a valid configuration.
