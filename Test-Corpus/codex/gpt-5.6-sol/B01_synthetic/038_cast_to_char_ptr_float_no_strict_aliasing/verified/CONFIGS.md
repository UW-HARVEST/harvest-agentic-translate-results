# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
conditional source, definition, or option. There is exactly one valid feature
combination:

| # | Cargo feature set | C configuration | [ ] |
|---|-------------------|-----------------|-----|
| 1 | empty (`--no-default-features`) | default CMake configuration | [x] |

## Runtime Configurations

The public surface is the complete set from the C shared object. `driver`
always copies exactly `sizeof(float)` bytes and emits them in native byte
order. `main` initializes positive zero, asks libc to parse one `%f`, ignores
the conversion count, then calls `driver`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | Direct FFI call; randomized raw `float` bit patterns spanning positive/negative zero, finite normal/subnormal values, infinities, and quiet/signaling NaNs; exactly 4 native-order bytes. | [x] |
| 2 | `main` -> `driver` | Successful `%f` conversion from randomized finite decimal text, including signs, integer/fraction forms, and decimal exponents. | [x] |
| 3 | `main` -> `driver` | Successful `%f` conversion from randomized hexadecimal floating text with binary exponents. | [x] |
| 4 | `main` -> `driver` | Successful `%f` conversion of case variants of positive/negative infinity. | [x] |
| 5 | `main` -> `driver` | Successful `%f` conversion of case variants of NaN. | [x] |
| 6 | `main` -> `driver` | Successful `%f` conversion with leading whitespace and trailing unread bytes; output is determined by the converted prefix. | [x] |

Failed conversions are listed in `ERRORS.md`.
