# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table. `c_src/CMakeLists.txt` has no options,
conditional definitions, or conditional sources. There is exactly one valid
combination:

| # | Cargo feature combination | CMake configuration | checked |
|---|---------------------------|---------------------|---------|
| 1 | Empty set (`--no-default-features`) | Default, with position-independent code enabled | [x] |

## Runtime and input configurations

The public API has one entry point, `main(int argc, char **argv)`, and no
runtime option, mode, flag, enum, element type, byte-order, or format
parameter. For valid calls `argc` is always `2`. The rows below cover the
input shapes distinguished by the `strtol` conversion result and by the
`val % 10 == 9` loop branch. Invalid conversion is tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `main` | Canonical nonnegative decimal whose narrowed `int` initially has remainder `9`; loop stops after its first print. | [x] |
| 2 | `main` | Canonical nonnegative decimal whose narrowed `int` initially does not have remainder `9`; loop increments one or more times before stopping. | [x] |
| 3 | `main` | Canonical small negative decimal; C's signed remainder is negative or zero until the count crosses zero and reaches positive remainder `9`. | [x] |
| 4 | `main` | Successful decimal with leading ASCII whitespace and an optional `+` or `-` sign, as accepted by `strtol`. | [x] |
| 5 | `main` | Successful numeric prefix followed by nonnumeric bytes; accepted because C checks only `end == argv[1]`, not `*end == '\0'`. | [x] |
| 6 | `main` | Decimal outside the C `long` range; `strtol` clamps to `LONG_MIN`/`LONG_MAX`, then C narrows to `int`. | [x] |
