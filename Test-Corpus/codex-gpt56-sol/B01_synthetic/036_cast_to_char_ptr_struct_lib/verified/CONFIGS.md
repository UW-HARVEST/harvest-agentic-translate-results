# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so there are no named or default
features. `c_src/CMakeLists.txt` has no options or conditional source/definition
branches.

The complete valid feature combination set is:

| # | Cargo invocation feature set | C configuration |
|---|------------------------------|-----------------|
| 1 | `--no-default-features` (empty set) | Default CMake configuration |

## Runtime and Input Configurations

Mechanical branch analysis found no public runtime options, modes, flags,
formats, byte-order choices, element types, or caller-controlled counts. The
public header declares only `void driver(int x)`. The implementation always
constructs one zero-initialized `house_t`, assigns the caller's full-width C
`int` to `floors`, assigns constants `3` and `2.0`, and prints all
`sizeof(house_t)` bytes as lowercase hexadecimal followed by a newline.

The private `print_hex` helper is not a public entry point and its pointer and
length cannot be configured through the API.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; `floors` sampled across the full C `int` domain, including `INT_MIN`, negative, zero, positive, and `INT_MAX`; fixed 16-byte `house_t` output on the target ABI | [x] |
