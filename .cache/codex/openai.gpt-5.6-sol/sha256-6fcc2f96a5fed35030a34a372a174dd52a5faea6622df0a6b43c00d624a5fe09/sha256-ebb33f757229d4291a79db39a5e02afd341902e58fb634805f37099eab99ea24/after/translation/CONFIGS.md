# Configuration surface

Mechanically derived from the public header and the `if`/`else if` parser
branches in `../c_src/src/lib.c`. There is one public entry point and no runtime
feature, option, mode, or flag setter. Header `flags`, description format
flags, packet sizing fields, bit depth, packet count, priming frames, remainder
frames, edit count, and block bytes are accepted but not inspected.

Every row is exercised with many fixed-seed randomized inputs. Values named
"arbitrary" span their complete bit width.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `ima_parse` | canonical `desc -> pakt -> data` chunk order; arbitrary accepted metadata and 0/1/many block payloads | [x] |
| 2 | `ima_parse` | `pakt -> desc -> data` order, proving the two required metadata chunks are order-independent | [x] |
| 3 | `ima_parse` | zero-payload unknown chunk before the required chunks; parser advances by the chunk-header width | [x] |
| 4 | `ima_parse` | non-empty unknown chunk between required chunks; arbitrary payload bytes and positive skip size | [x] |
| 5 | `ima_parse` | duplicate `desc` chunks; the last description before `data` supplies format, sample rate, and channel count | [x] |
| 6 | `ima_parse` | duplicate `pakt` chunks; the last packet table before `data` supplies frame count | [x] |
| 7 | `ima_parse` | `data` chunk followed by arbitrary trailing chunk-shaped bytes; parsing stops at the first `data` chunk | [x] |
| 8 | `ima_parse` | `data` chunk size is zero, so `info.size` is zero while `info.blocks` still points after `edit_count` | [x] |
| 9 | `ima_parse` | `data` chunk size is negative (`-1` and randomized negative `i64` values); accepted and converted to `ima_u64_t` | [x] |
| 10 | `ima_parse` | signed-size boundaries for `data` (`i64::MIN`, `i64::MAX`) | [x] |
| 11 | `ima_parse` | sample-rate field spans arbitrary 64-bit patterns, including signed zero, infinities, subnormals, and NaNs | [x] |
| 12 | `ima_parse` | packet-table frame count spans negative/zero/positive and `i64` boundaries; result is stored in unsigned `ima_u64_t` | [x] |
| 13 | `ima_parse` | channel count spans zero, one, typical multichannel values, and the full `u32` boundary | [x] |
| 14 | `ima_parse` | caller input begins at byte offsets 0 through 7, exercising every alignment modulo eight used by the packed byte stream | [x] |

Build configuration surface: `Cargo.toml` declares no Cargo features, so the
only feature combination is the default/no-feature build.
