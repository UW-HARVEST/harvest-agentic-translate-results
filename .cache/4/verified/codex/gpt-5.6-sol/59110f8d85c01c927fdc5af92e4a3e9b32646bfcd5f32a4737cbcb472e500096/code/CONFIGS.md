# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so there are no named features and no
default features. `c_src/CMakeLists.txt` declares no options or conditional
sources. The single valid build configuration is:

```text
cargo ... --no-default-features --features ''
```

## Runtime Configurations

The C source has no runtime mode, option, flag, format, type, byte-order, or
variable-size branches. Its loops have fixed dimensions:

- `array`: exactly `256 * 1024` C `int` elements
- inner arithmetic loop: exactly 100 iterations per element
- end-to-end operation: exactly 2000 full-array passes
- `seed`: one `unsigned int`; all bit patterns follow the same path

The rows below are the full branch-distinct cross-product. Randomized data for
row 1 must include zero, positive, negative, and `int` boundary values.
Randomized seeds for row 2 must include the `unsigned int` boundaries.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `array`, `perform_expensive_operations` | Direct low-level call over the fixed 262144-element exported `int` array; no options | [x] |
| 2 | `long_exec` (composes `srand`, fixed-array initialization, 2000 calls to `perform_expensive_operations`, XOR reduction, and decimal stdout) | Any 32-bit unsigned seed; fixed 262144-element output array and one newline-terminated decimal result | [x] |

Evidence: `tests/ffi_differential.rs` passes 12 randomized full arrays for row
1 and eight isolated seeds (four boundaries plus four fixed-seed random values)
for row 2. Both shared libraries are called exclusively through `libloading`.

Feature-combination status: [x] complete for the sole empty feature set.
