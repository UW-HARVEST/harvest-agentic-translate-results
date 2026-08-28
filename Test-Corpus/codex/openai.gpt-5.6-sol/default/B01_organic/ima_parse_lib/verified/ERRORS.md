# Error Surface

Mechanically derived from every rejection return and conditional check in
`../c_src/src/lib.c`. The public API has no length argument, enum argument,
assertion, null check, range check, or min/max constant.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `ima_parse` | `ima_btoh32(header->type) != 0x63616666` (the first four input bytes are not `caff`) | `-1` | [x] |
| 2 | `ima_parse` | file type is valid and `ima_btoh16(header->version) != 1` | `-2` | [x] |
| 3 | `ima_parse` | header is valid, `desc` and `pakt` precede `data`, and `ima_btoh32(desc->format_id) != 0x696d6134` (the description format bytes are not `ima4`) | `-3` | [x] |

## Generic FFI Boundaries

`ima_parse` has no length or enum parameter, so zero/oversized lengths and
out-of-range enum values are not applicable. C performs no null checks:
null `data` and null `info` with otherwise valid data have process-level
undefined behavior and are compared in isolated subprocess probes.
