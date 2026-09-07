# Configuration Surface

The axes below come directly from the public function signatures and branches
in `src/lib.c`:

- operation entry point: add 10, double, or triple;
- ignored operation arguments: arbitrary `unused_param`, and null/non-null
  opaque `unused_context`;
- `gotomach` mode: `0`, `1`, `2`, or any other integer (warning + mode `0`);
- iteration shape: empty, one, many, or `UINT16_MAX`;
- threshold branch: no generated values selected, some selected, or all
  selected;
- valid seed shape: minimum (`0`), interior values, and maximum
  (`UINT16_MAX`);
- count warning: selecting all `UINT16_MAX` generated values reaches the
  `state->count >= UINT16_MAX` break.

Rows use randomized valid seeds and thresholds where applicable, always adding
the valid seed boundaries. “Invalid/default mode” includes negative values,
`3`, and extreme `int` values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `process_value` | randomized non-overflowing `int` values; arbitrary ignored integer; null and non-null context | [x] |
| C02 | `double_value` | randomized values in `INT_MIN/2..INT_MAX/2`; arbitrary ignored integer; null and non-null context | [x] |
| C03 | `triple_value` | randomized values in `ceil(INT_MIN/3)..floor(INT_MAX/3)`; arbitrary ignored integer; null and non-null context | [x] |
| C04 | `gotomach` | mode 0; 0 iterations; valid seed min/interior/max; threshold arbitrary | [x] |
| C05 | `gotomach` | mode 1; 0 iterations; valid seed min/interior/max; threshold arbitrary | [x] |
| C06 | `gotomach` | mode 2; 0 iterations; valid seed min/interior/max; threshold arbitrary | [x] |
| C07 | `gotomach` | invalid/default mode; 0 iterations; valid seed min/interior/max; threshold arbitrary; warning path | [x] |
| C08 | `gotomach` | mode 0; 1 iteration; threshold selects none; valid seed min/interior/max | [x] |
| C09 | `gotomach` | mode 0; 1 iteration; threshold selects all; valid seed min/interior/max | [x] |
| C10 | `gotomach` | mode 1; 1 iteration; threshold selects none; valid seed min/interior/max | [x] |
| C11 | `gotomach` | mode 1; 1 iteration; threshold selects all; valid seed min/interior/max | [x] |
| C12 | `gotomach` | mode 2; 1 iteration; threshold selects none; valid seed min/interior/max | [x] |
| C13 | `gotomach` | mode 2; 1 iteration; threshold selects all; valid seed min/interior/max | [x] |
| C14 | `gotomach` | invalid/default mode; 1 iteration; threshold selects none; valid seed min/interior/max; warning path | [x] |
| C15 | `gotomach` | invalid/default mode; 1 iteration; threshold selects all; valid seed min/interior/max; warning path | [x] |
| C16 | `gotomach` | mode 0; many iterations; threshold selects none | [x] |
| C17 | `gotomach` | mode 0; many iterations; threshold selects some | [x] |
| C18 | `gotomach` | mode 0; many iterations; threshold selects all | [x] |
| C19 | `gotomach` | mode 1; many iterations; threshold selects none | [x] |
| C20 | `gotomach` | mode 1; many iterations; threshold selects some | [x] |
| C21 | `gotomach` | mode 1; many iterations; threshold selects all | [x] |
| C22 | `gotomach` | mode 2; many iterations; threshold selects none | [x] |
| C23 | `gotomach` | mode 2; many iterations; threshold selects some | [x] |
| C24 | `gotomach` | mode 2; many iterations; threshold selects all | [x] |
| C25 | `gotomach` | invalid/default mode; many iterations; threshold selects none; warning path | [x] |
| C26 | `gotomach` | invalid/default mode; many iterations; threshold selects some; warning path | [x] |
| C27 | `gotomach` | invalid/default mode; many iterations; threshold selects all; warning path | [x] |
| C28 | `gotomach` | mode 0; `UINT16_MAX` iterations; all values selected; maximum-count warning/break | [x] |
| C29 | `gotomach` | mode 1; `UINT16_MAX` iterations; all values selected; maximum-count warning/break | [x] |
| C30 | `gotomach` | mode 2; `UINT16_MAX` iterations; all values selected; maximum-count warning/break | [x] |
| C31 | `gotomach` | invalid/default mode; `UINT16_MAX` iterations; all values selected; invalid-mode and maximum-count warnings | [x] |

Cargo feature surface: `Cargo.toml` declares no features, so the complete
feature set is the empty set. Verification is run both normally and with
`--no-default-features`; both select that same empty feature combination.
