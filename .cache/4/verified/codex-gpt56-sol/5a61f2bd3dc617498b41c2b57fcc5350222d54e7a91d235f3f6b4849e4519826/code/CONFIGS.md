# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and CMake declares no options or
preprocessor configuration. The full valid feature power set therefore has one
member:

| # | Cargo invocation configuration | CMake configuration |
|---|--------------------------------|----------------------|
| 1 | `--no-default-features --features ''` | default |

## Runtime Configurations

For `main`, each successful `fgets` shape is crossed with each nonnegative
`data` class that changes the copy count or the `data < 100` decision. Tests
within each row vary decimal spelling, leading whitespace/sign/zeroes, trailing
bytes, embedded NUL where applicable, and values with a fixed random seed.
EOF with no bytes and negative values are in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | valid pointer to an empty NUL-terminated byte string | [x] |
| 2 | `printLine` | valid pointer to a nonempty NUL-terminated byte string; randomized lengths and non-NUL byte values | [x] |
| 3 | `main` | `fgets` succeeds with newline among the first 13 bytes; `atoi` yields `0` | [x] |
| 4 | `main` | `fgets` succeeds with newline among the first 13 bytes; `atoi` yields `1..98` | [x] |
| 5 | `main` | `fgets` succeeds with newline among the first 13 bytes; `atoi` yields boundary value `99` | [x] |
| 6 | `main` | `fgets` succeeds with newline among the first 13 bytes; `atoi` yields `100..INT_MAX` | [x] |
| 7 | `main` | `fgets` succeeds at EOF after 1..13 bytes without newline; `atoi` yields `0` | [x] |
| 8 | `main` | `fgets` succeeds at EOF after 1..13 bytes without newline; `atoi` yields `1..98` | [x] |
| 9 | `main` | `fgets` succeeds at EOF after 1..13 bytes without newline; `atoi` yields boundary value `99` | [x] |
| 10 | `main` | `fgets` succeeds at EOF after 1..13 bytes without newline; `atoi` yields `100..INT_MAX` | [x] |
| 11 | `main` | at least 14 input bytes are available, so `fgets` stores only the first 13; that prefix makes `atoi` yield `0` | [x] |
| 12 | `main` | at least 14 input bytes are available, so `fgets` stores only the first 13; that prefix makes `atoi` yield `1..98` | [x] |
| 13 | `main` | at least 14 input bytes are available, so `fgets` stores only the first 13; that prefix makes `atoi` yield boundary value `99` | [x] |
| 14 | `main` | at least 14 input bytes are available, so `fgets` stores only the first 13; that prefix makes `atoi` yield `100..INT_MAX` | [x] |
