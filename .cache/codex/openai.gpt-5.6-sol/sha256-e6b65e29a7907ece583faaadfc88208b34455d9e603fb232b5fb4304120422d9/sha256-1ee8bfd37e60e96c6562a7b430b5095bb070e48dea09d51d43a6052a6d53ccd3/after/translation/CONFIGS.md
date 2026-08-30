# Configuration Surface

Mechanical sources:

- `../c_src/include/driver.h` declares the full public API:
  `void driver(int x)`.
- `../c_src/src/driver.c` defines no runtime options, modes, flags, switches,
  conditional branches, or compile-time feature branches.
- `driver` always constructs one zero-initialized `house_t` with C layout
  `{ int floors; int bedrooms; double bathrooms; }`, fixes `bedrooms` to `3`
  and `bathrooms` to `2.0`, copies all `sizeof(house_t)` bytes, and prints each
  byte as two lowercase hexadecimal digits followed by a newline.
- The only input axis is the complete by-value C `int` domain. Tests must
  include zero, both signs, `INT_MIN`, `INT_MAX`, and many fixed-seed randomized
  values.
- The internal `print_hex` function is `static`, so it is not a public
  low-level entry point. There are no convenience wrappers or feature
  combinations in `Cargo.toml`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; one arbitrary 32-bit C `int`; fixed 16-byte `house_t` representation printed as 32 lowercase hex digits plus newline | [x] |

## Feature Matrix

`Cargo.toml` declares no features, so the empty feature set is the only
combination. It was verified through both equivalent Cargo invocations.

| invocation | check | release build | differential tests |
|------------|-------|---------------|--------------------|
| default | [x] | [x] | [x] |
| `--no-default-features` | [x] | [x] | [x] |
