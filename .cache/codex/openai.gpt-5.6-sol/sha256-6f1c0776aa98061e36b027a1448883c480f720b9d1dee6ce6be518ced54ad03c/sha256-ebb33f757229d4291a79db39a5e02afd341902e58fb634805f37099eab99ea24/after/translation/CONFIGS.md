# Configuration surface

The crate has no Cargo features. Phase D therefore exercises the default build
and the equivalent `--no-default-features` build.

The classes below are mechanically derived from the C `if`/`switch` branches.
For the arity family, these `param1` classes select both the bitmask branch and
the `compare_allocations` positive-value branch:

- `P0`: positive multiple of 4 (`param1 % 4 == 0`)
- `P1`, `P2`, `P3`: positive with remainder 1, 2, or 3
- `Z`: zero
- `N0`: negative multiple of 4 (`param1 % 4 == 0`)
- `ND`: negative non-multiple of 4 (negative remainder, `default` mask branch)

Each arity row also randomizes `param2`. An `NZ` axis randomizes nonzero
positive and negative values. For `arity`, tests use both the canonical length
and `int` values with the same low byte, because the compiled C definition
converts the first argument to `unsigned char`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C01 | `shift_array` | `size <= 0`; condition cannot become active | [x] |
| C02 | `shift_array` | `size == 1`, `positions <= 0`; no-op | [x] |
| C03 | `shift_array` | `size == 1`, `positions >= 1`; no-op | [x] |
| C04 | `shift_array` | `size >= 2`, `positions <= 0`; no-op | [x] |
| C05 | `shift_array` | `size >= 2`, `0 < positions < size`; overlapping right shift and zero prefix | [x] |
| C06 | `shift_array` | `size >= 2`, `positions >= size`; no-op | [x] |
| C07 | `process_string` | first byte is NUL (empty string) | [x] |
| C08 | `process_string` | first byte non-NUL; random NUL-terminated length/content | [x] |
| C09 | `apply_bitmask` | `operation == 0` | [x] |
| C10 | `apply_bitmask` | `operation == 1` | [x] |
| C11 | `apply_bitmask` | `operation == 2` | [x] |
| C12 | `apply_bitmask` | `operation == 3` | [x] |
| C13 | `apply_bitmask` | operation outside `0..=3`; `default` identity branch | [x] |
| C14 | `init_matrix` | writable `3 x 4` `int` matrix | [x] |
| C15 | `compare_allocations` | `val1 <= 0`; do not add 10 | [x] |
| C16 | `compare_allocations` | `val1 > 0`; add 10 | [x] |
| C17 | `arity2`; `arity` low byte 2 | `param1=P0`; fixed `param3=0`, `param4=0` | [x] |
| C18 | `arity2`; `arity` low byte 2 | `param1=P1`; fixed `param3=0`, `param4=0` | [x] |
| C19 | `arity2`; `arity` low byte 2 | `param1=P2`; fixed `param3=0`, `param4=0` | [x] |
| C20 | `arity2`; `arity` low byte 2 | `param1=P3`; fixed `param3=0`, `param4=0` | [x] |
| C21 | `arity2`; `arity` low byte 2 | `param1=Z`; fixed `param3=0`, `param4=0` | [x] |
| C22 | `arity2`; `arity` low byte 2 | `param1=N0`; fixed `param3=0`, `param4=0` | [x] |
| C23 | `arity2`; `arity` low byte 2 | `param1=ND`; fixed `param3=0`, `param4=0` | [x] |
| C24 | `arity3`; `arity` low byte 3 | `param1=P0`, `param3=0`; fixed `param4=0` | [x] |
| C25 | `arity3`; `arity` low byte 3 | `param1=P1`, `param3=0`; fixed `param4=0` | [x] |
| C26 | `arity3`; `arity` low byte 3 | `param1=P2`, `param3=0`; fixed `param4=0` | [x] |
| C27 | `arity3`; `arity` low byte 3 | `param1=P3`, `param3=0`; fixed `param4=0` | [x] |
| C28 | `arity3`; `arity` low byte 3 | `param1=Z`, `param3=0`; fixed `param4=0` | [x] |
| C29 | `arity3`; `arity` low byte 3 | `param1=N0`, `param3=0`; fixed `param4=0` | [x] |
| C30 | `arity3`; `arity` low byte 3 | `param1=ND`, `param3=0`; fixed `param4=0` | [x] |
| C31 | `arity3`; `arity` low byte 3 | `param1=P0`, `param3=NZ`; fixed `param4=0` | [x] |
| C32 | `arity3`; `arity` low byte 3 | `param1=P1`, `param3=NZ`; fixed `param4=0` | [x] |
| C33 | `arity3`; `arity` low byte 3 | `param1=P2`, `param3=NZ`; fixed `param4=0` | [x] |
| C34 | `arity3`; `arity` low byte 3 | `param1=P3`, `param3=NZ`; fixed `param4=0` | [x] |
| C35 | `arity3`; `arity` low byte 3 | `param1=Z`, `param3=NZ`; fixed `param4=0` | [x] |
| C36 | `arity3`; `arity` low byte 3 | `param1=N0`, `param3=NZ`; fixed `param4=0` | [x] |
| C37 | `arity3`; `arity` low byte 3 | `param1=ND`, `param3=NZ`; fixed `param4=0` | [x] |
| C38 | `arity4`; `arity` low byte `4..=255` | `param1=P0`, `param3=0`, `param4=0` | [x] |
| C39 | `arity4`; `arity` low byte `4..=255` | `param1=P1`, `param3=0`, `param4=0` | [x] |
| C40 | `arity4`; `arity` low byte `4..=255` | `param1=P2`, `param3=0`, `param4=0` | [x] |
| C41 | `arity4`; `arity` low byte `4..=255` | `param1=P3`, `param3=0`, `param4=0` | [x] |
| C42 | `arity4`; `arity` low byte `4..=255` | `param1=Z`, `param3=0`, `param4=0` | [x] |
| C43 | `arity4`; `arity` low byte `4..=255` | `param1=N0`, `param3=0`, `param4=0` | [x] |
| C44 | `arity4`; `arity` low byte `4..=255` | `param1=ND`, `param3=0`, `param4=0` | [x] |
| C45 | `arity4`; `arity` low byte `4..=255` | `param1=P0`, `param3=NZ`, `param4=0` | [x] |
| C46 | `arity4`; `arity` low byte `4..=255` | `param1=P1`, `param3=NZ`, `param4=0` | [x] |
| C47 | `arity4`; `arity` low byte `4..=255` | `param1=P2`, `param3=NZ`, `param4=0` | [x] |
| C48 | `arity4`; `arity` low byte `4..=255` | `param1=P3`, `param3=NZ`, `param4=0` | [x] |
| C49 | `arity4`; `arity` low byte `4..=255` | `param1=Z`, `param3=NZ`, `param4=0` | [x] |
| C50 | `arity4`; `arity` low byte `4..=255` | `param1=N0`, `param3=NZ`, `param4=0` | [x] |
| C51 | `arity4`; `arity` low byte `4..=255` | `param1=ND`, `param3=NZ`, `param4=0` | [x] |
| C52 | `arity4`; `arity` low byte `4..=255` | `param1=P0`, `param3=0`, `param4=NZ` | [x] |
| C53 | `arity4`; `arity` low byte `4..=255` | `param1=P1`, `param3=0`, `param4=NZ` | [x] |
| C54 | `arity4`; `arity` low byte `4..=255` | `param1=P2`, `param3=0`, `param4=NZ` | [x] |
| C55 | `arity4`; `arity` low byte `4..=255` | `param1=P3`, `param3=0`, `param4=NZ` | [x] |
| C56 | `arity4`; `arity` low byte `4..=255` | `param1=Z`, `param3=0`, `param4=NZ` | [x] |
| C57 | `arity4`; `arity` low byte `4..=255` | `param1=N0`, `param3=0`, `param4=NZ` | [x] |
| C58 | `arity4`; `arity` low byte `4..=255` | `param1=ND`, `param3=0`, `param4=NZ` | [x] |
| C59 | `arity4`; `arity` low byte `4..=255` | `param1=P0`, `param3=NZ`, `param4=NZ` | [x] |
| C60 | `arity4`; `arity` low byte `4..=255` | `param1=P1`, `param3=NZ`, `param4=NZ` | [x] |
| C61 | `arity4`; `arity` low byte `4..=255` | `param1=P2`, `param3=NZ`, `param4=NZ` | [x] |
| C62 | `arity4`; `arity` low byte `4..=255` | `param1=P3`, `param3=NZ`, `param4=NZ` | [x] |
| C63 | `arity4`; `arity` low byte `4..=255` | `param1=Z`, `param3=NZ`, `param4=NZ` | [x] |
| C64 | `arity4`; `arity` low byte `4..=255` | `param1=N0`, `param3=NZ`, `param4=NZ` | [x] |
| C65 | `arity4`; `arity` low byte `4..=255` | `param1=ND`, `param3=NZ`, `param4=NZ` | [x] |
