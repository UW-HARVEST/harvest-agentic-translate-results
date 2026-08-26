# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so its complete feature set is empty.
There is exactly one valid feature combination:

| # | enabled Cargo features | verification command |
|---|------------------------|----------------------|
| 1 | none (`{}`) | `cargo check/test --no-default-features` |

`c_src/CMakeLists.txt` declares no options, cache-controlled source selection,
compile definitions, or conditional branches. Its only configuration is the
default shared-library target containing `src/lib.c`.

## Runtime Configurations

The sole public entry point is `ima_parse`. It has no runtime option/mode
arguments. The rows below cover the cross-product portions that the C parser
actually distinguishes: each chunk-loop branch and position, replacement of
saved chunk pointers, ignored fields, and the output value/shape classes.
Every multi-byte input field is stored in CAF big-endian byte order.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `ima_parse` | Minimal valid sequence `desc -> pakt -> data`; randomized ordinary finite sample rate, nonnegative frame count, channel count, data size, and block bytes | [x] |
| 2 | `ima_parse` | Required metadata reversed: `pakt -> desc -> data`; randomized output fields | [x] |
| 3 | `ima_parse` | Unknown zero-payload chunk before both required metadata chunks | [x] |
| 4 | `ima_parse` | Unknown positive-payload chunk between `desc` and `pakt`; payload size is an aligned positive boundary/random value | [x] |
| 5 | `ima_parse` | Unknown chunk after both required metadata chunks and immediately before `data` | [x] |
| 6 | `ima_parse` | Repeated `desc` chunks before `data`; the last description supplies format, sample rate, and channel count | [x] |
| 7 | `ima_parse` | Repeated `pakt` chunks before `data`; the last packet table supplies frame count | [x] |
| 8 | `ima_parse` | Header flags and all non-read description, packet-table, and data fields vary across full-width random values; parsed fields remain valid | [x] |
| 9 | `ima_parse` | Data chunk size classes `0`, positive, `INT64_MAX`, negative, and `INT64_MIN`; C stores the signed value into `ima_u64_t` modulo 2^64 | [x] |
| 10 | `ima_parse` | Parsed numeric boundaries: all `f64` bit classes (signed zero, subnormal, finite, infinity, NaN), `u32` channel boundaries, and signed `i64` frame boundaries | [x] |
| 11 | `ima_parse` | Data payload shape varies from no complete block through one/many 34-byte `ima_block` records; returned block pointer offset and pointed-to bytes are compared | [x] |
