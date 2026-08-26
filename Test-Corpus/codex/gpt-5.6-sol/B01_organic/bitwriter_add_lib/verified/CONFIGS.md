# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
options, conditional definitions, or conditional sources. Therefore there is
exactly one valid feature combination:

| # | Rust feature set | C configuration | [ ] |
|---|------------------|-----------------|-----|
| F1 | Empty set (`--no-default-features`) | Default CMake configuration | [x] |

## Runtime configurations

The public header exposes one entry point and no option, mode, flag, enum,
format, or byte-order selector. The only source-level control-flow predicate
is `bw->bits + bits >= 64`; the loop is additionally capped at 100 iterations.
For defined C shifts, `bw->bits` is `0..=63` and `bits` is `1..=64`.

Every row randomizes the operand, accumulated value, `tot`, `pos`, `len`, and
buffer pointer value. The latter three fields are expected to remain unchanged.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `bitwriter_add` | Empty writer: `bw.bits == 0`, `bits in 1..=63`; sum below 64, loop not entered. | [x] |
| C2 | `bitwriter_add` | Partial writer: `bw.bits in 1..=62`, `bits in 1..=(63 - bw.bits)`; sum below 64, loop not entered. | [x] |
| C3 | `bitwriter_add` | Exact boundary: `bw.bits in 0..=63`, `bits == 64 - bw.bits`; sum equals 64 and the capped loop is entered. | [x] |
| C4 | `bitwriter_add` | Above boundary: `bw.bits in 1..=63`, `bits in (64 - bw.bits + 1)..=64`; sum exceeds 64 and the capped loop is entered. | [x] |
