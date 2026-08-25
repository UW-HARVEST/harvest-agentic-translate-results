# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and no optional dependencies.
`c_src/CMakeLists.txt` has no options or conditional compilation. There is
therefore exactly one valid feature combination:

| # | Cargo feature set | C configuration | Check |
|---|-------------------|-----------------|-------|
| 1 | empty (`--no-default-features`) | default | [x] |

## Runtime Configurations

There are no public headers, runtime options, modes, flags, pointers, lengths,
enums, byte-order controls, or variable data shapes. The fixed `house_t` layout
is two C `int` fields followed by one `double`; its complete object
representation is printed as 16 native-endian bytes on the build platform.

| # | entry point(s) | configuration (options set + input shape) | Check |
|---|----------------|--------------------------------------------|-------|
| 1 | `driver` | `floors` across the full C `int` domain; fixed `bedrooms = 3`, `bathrooms = 2.0` | [x] |
| 2 | `main` | `scanf("%d")` succeeds; randomized valid decimal C `int` input | [x] |
| 3 | `main` | `scanf("%d")` returns matching failure; initialized `x = 0` is retained | [x] |
| 4 | `main` | `scanf("%d")` reaches EOF before conversion; initialized `x = 0` is retained | [x] |
