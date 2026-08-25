# Configuration Surface

## Build-Time Configurations

`Cargo.toml` declares no `[features]`, and `c_src/CMakeLists.txt` declares no
options or conditional definitions. There is therefore one valid combination:

| # | Rust features | CMake options | [ ] |
|---|---------------|---------------|-----|
| B1 | empty set (`--no-default-features`) | defaults (none declared) | [x] |

## Runtime Configurations

The C source has two public entry points and no runtime option flags. Rows D1-D7
partition the outcomes of `strcspn` used by `driver`. Rows M1-M9 cover the
distinct fixed-buffer and string shapes reaching `main` through its two
`fgets` calls. The 100-byte arrays allow at most 99 input bytes per call.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| D1 | `driver` | empty `s1`; empty reject string | [x] |
| D2 | `driver` | empty `s1`; nonempty reject string | [x] |
| D3 | `driver` | nonempty `s1`; empty reject string | [x] |
| D4 | `driver` | nonempty strings; rejected byte at index 0 | [x] |
| D5 | `driver` | nonempty strings; first rejected byte at an interior index | [x] |
| D6 | `driver` | nonempty strings; first rejected byte at the final index | [x] |
| D7 | `driver` | nonempty strings; no byte is rejected | [x] |
| M1 | `main`, `driver` | two empty newline-terminated lines | [x] |
| M2 | `main`, `driver` | two short newline-terminated lines; no rejected byte | [x] |
| M3 | `main`, `driver` | two short newline-terminated lines; rejection at index 0 | [x] |
| M4 | `main`, `driver` | two short newline-terminated lines; rejection at an interior/final index | [x] |
| M5 | `main`, `driver` | first line has exactly 98 data bytes plus newline (fits first `fgets`) | [x] |
| M6 | `main`, `driver` | first line has 99 data bytes plus newline (first `fgets` truncates; second consumes newline) | [x] |
| M7 | `main`, `driver` | first logical line exceeds 99 bytes (both `fgets` calls consume chunks of that line) | [x] |
| M8 | `main`, `driver` | second line reaches EOF without newline (its final data byte is removed) | [x] |
| M9 | `main`, `driver` | input contains an embedded NUL before newline (`strlen` truncates at NUL) | [x] |
