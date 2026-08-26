# Configuration Surface

`Cargo.toml` defines no optional features and CMake defines no options or
preprocessor configurations. The complete build-time configuration set is:

| # | Cargo feature combination | C configuration |
|---|---------------------------|-----------------|
| 1 | `--no-default-features` (empty set; also the default) | default CMake configuration |

The C public entry points are `run` (the low-level operation) and `main` (the
stdin-driven composition). There are no runtime option setters, flags, modes,
enums, lengths, element types, byte-order choices, or alternate formats.
Rows below enumerate the data shapes distinguished by arithmetic, `printf`,
`fgets`, and `strtol`.

| # | entry point(s) | configuration (options set + input shape) | covered |
|---|----------------|--------------------------------------------|---------|
| 1 | `run` | ordinary finite house; `extra_bedrooms == 0` | [x] |
| 2 | `run` | ordinary finite house; positive `extra_bedrooms`, no integer boundary crossing | [x] |
| 3 | `run` | ordinary finite house; negative `extra_bedrooms`, no integer boundary crossing | [x] |
| 4 | `run` | `floors == INT_MAX`, exercising the increment boundary | [x] |
| 5 | `run` | positive bedroom addition crosses `INT_MAX` | [x] |
| 6 | `run` | negative bedroom addition crosses `INT_MIN` | [x] |
| 7 | `run` | finite bathroom values around one-decimal formatting boundaries, including negative zero | [x] |
| 8 | `run` | non-finite bathroom values: positive/negative infinity and NaN | [x] |
| 9 | `main` -> `run` | plain unsigned decimal input, including zero, terminated by newline | [x] |
| 10 | `main` -> `run` | minus sign followed by decimal digits | [x] |
| 11 | `main` -> `run` | plus sign followed by decimal digits | [x] |
| 12 | `main` -> `run` | leading C-locale whitespace before a valid decimal | [x] |
| 13 | `main` -> `run` | valid decimal prefix followed by non-digit trailing bytes | [x] |
| 14 | `main` -> `run` | exact `INT_MAX` input (`2147483647`) | [x] |
| 15 | `main` -> `run` | exact `INT_MIN` input (`-2147483648`) | [x] |
| 16 | `main` -> `run` | valid decimal prefix followed by an embedded NUL and ignored bytes | [x] |
| 17 | `main` -> `run` | valid conversion at the 99-byte `fgets` payload limit | [x] |
| 18 | `main` -> `run` | valid decimal followed by EOF without a newline | [x] |
