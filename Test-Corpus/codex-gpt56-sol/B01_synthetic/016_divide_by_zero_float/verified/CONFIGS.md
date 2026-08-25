# Configuration Surface

## Build-Time Configurations

Cargo has no `[features]` table, so its implicit feature set is empty. CMake
defines no options, conditional targets, or preprocessor configurations.
Therefore the complete feature matrix has one member:

| # | Cargo invocation feature set | C configuration | |
|---|------------------------------|-----------------|--|
| 1 | `--no-default-features --features ''` | Default option-free CMake build | [x] |

## Runtime Configurations

There are no runtime option setters, modes, flags, enums, element types, or byte
order controls. The mechanically visible axes are the five exported entry
points, nullable versus non-null strings, signed integer boundaries, `fgets`
termination at its 19-byte payload boundary, `atof` input syntax, the
`fabs(data) > 0.000001` branch, and composition of the two reads in `main`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `printLine` | Non-null empty C string. | [x] |
| 2 | `printLine` | Non-null C string with ordinary bytes and with an embedded newline before its NUL terminator. | [x] |
| 3 | `printIntLine` | Signed decimal formatting across `INT_MIN`, negative, zero, positive, and `INT_MAX`. | [x] |
| 4 | `bad` | Newline-terminated numeric input shorter than 19 bytes; positive finite quotient in `int` range. | [x] |
| 5 | `bad` | Newline-terminated numeric input shorter than 19 bytes; negative finite quotient in `int` range. | [x] |
| 6 | `bad` | Numeric input terminated by EOF before 19 bytes, with no newline. | [x] |
| 7 | `bad` | Exactly 19 numeric bytes before EOF; `fgets` fills its payload capacity. | [x] |
| 8 | `bad` | More than 19 bytes; only the first 19-byte chunk is consumed. | [x] |
| 9 | `bad` | Leading whitespace/sign, decimal, exponent, and trailing-junk forms accepted by `atof`. | [x] |
| 10 | `bad` | Successful read parsing as zero (`0`, signed zero, empty line, or nonnumeric text). | [x] |
| 11 | `bad` | Successful read parsing as NaN or infinity. | [x] |
| 12 | `bad` | Finite nonzero input whose quotient is outside the C `int` range. | [x] |
| 13 | `good` | Newline-terminated positive `data` with `data > 0.000001`; division branch. | [x] |
| 14 | `good` | Newline-terminated negative `data` with `data < -0.000001`; division branch. | [x] |
| 15 | `good` | Positive and negative values immediately outside the epsilon boundary after conversion to `float`. | [x] |
| 16 | `good` | Numeric input terminated by EOF before 19 bytes, with no newline; division branch. | [x] |
| 17 | `good` | Exactly 19 bytes before EOF; `fgets` fills its payload capacity and takes the division branch. | [x] |
| 18 | `good` | More than 19 bytes; only the first 19-byte chunk is consumed. | [x] |
| 19 | `good` | `atof` whitespace/sign, decimal, exponent, and trailing-junk forms taking the division branch. | [x] |
| 20 | `good` | Infinity, for which `fabs(data) > 0.000001` is true. | [x] |
| 21 | `main` | `argc == 0`, `argv == NULL`, two short newline-terminated numeric records; both reads succeed. | [x] |
| 22 | `main` | Nonzero `argc` and non-null `argv`; arguments are ignored and both reads succeed. | [x] |
| 23 | `main` | First record takes the epsilon rejection branch; second short record is consumed by `bad`. | [x] |
| 24 | `main` | First logical line exceeds 19 bytes, so `good` and `bad` consume consecutive chunks of that same line. | [x] |
| 25 | `main` | EOF after the first successful record, so `bad` takes its `fgets == NULL` branch. | [x] |
| 26 | `main` | Immediate EOF, so both `goodB2G` and `bad` take their `fgets == NULL` branches. | [x] |

Rows 1-20 exercise the lowest-level exported entry points directly. Rows 21-26
exercise their composed call path through `main`.
