# Error surface

The rows below are mechanically derived from every `assert`, explicit error
branch, and public-boundary rejection in `c_src/src/lib.c`. The C source has no
enum type, explicit null check, or documented public min/max range.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `cp_ptr` via `cp_inflate` | `s->bits_left & 7 != 0` when a stored block requests its byte pointer | assertion failure (`SIGABRT`); internal byte-alignment invariant | [x] |
| E02 | `cp_peak_bits` via `cp_inflate` | incrementing `word_index` would make `word_index > word_count` | assertion failure (`SIGABRT`); internal index invariant | [x] |
| E03 | `cp_consume_bits` via `cp_inflate` | `s->count < num_bits_to_read` after refill | assertion failure (`SIGABRT`) | [x] |
| E04 | `cp_read_bits` via `cp_inflate` | requested bit count is greater than 32 | assertion failure (`SIGABRT`) | [x] |
| E05 | `cp_read_bits` via `cp_inflate` | requested bit count is less than 0 | assertion failure (`SIGABRT`); no exported path supplies a negative count | [x] |
| E06 | `cp_read_bits` via `cp_inflate` | `s->bits_left <= 0` before a read | assertion failure (`SIGABRT`) | [x] |
| E07 | `cp_read_bits` via `cp_inflate` | `s->count > 64` before a read | assertion failure (`SIGABRT`); internal reservoir invariant | [x] |
| E08 | `cp_read_bits` via `cp_inflate` | `(s->bits_left + s->count) - num_bits_to_read < 0` | assertion failure (`SIGABRT`) | [x] |
| E09 | `cp_build` via `cp_inflate` | a Huffman code length is 16 or greater | assertion failure (`SIGABRT`); reachable by changing exported `cp_fixed_table` | [x] |
| E10 | `cp_decode` via `cp_inflate` | decoded prefix does not match the selected Huffman tree key | assertion failure (`SIGABRT`) | [x] |
| E11 | `cp_stored` via `cp_inflate` | `LEN != (uint16_t)~NLEN` | return `0`; reason is `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` | [x] |
| E12 | `cp_stored` via `cp_inflate` | `s->bits_left / 8 > LEN` (the exact C predicate) | return `0`; reason is `Stored block extends beyond end of input stream.` | [x] |
| E13 | `cp_block` via `cp_inflate` | a literal is decoded when `s->out + 1 > s->out_end` | return `0`; reason is `Attempted to overwrite out buffer while outputting a symbol.` | [x] |
| E14 | `cp_block` via `cp_inflate` | a match has `s->out - backwards_distance < s->begin` | return `0`; reason is `Attempted to write before out buffer (invalid backwards distance).` | [x] |
| E15 | `cp_block` via `cp_inflate` | a match has `s->out + length > s->out_end` | return `0`; reason is `Attempted to overwrite out buffer while outputting a string.` | [x] |
| E16 | `cp_inflate` | DEFLATE `BTYPE == 3` | return `0`; reason is `Detected unknown block type within input stream.` | [x] |
| E17 | `unfilter` | first-row filter byte is not in `0..=4` and `h > 0` | return `0` | [x] |
| E18 | `unfilter` | any later-row filter byte is not in `0..=4` and `h > 1` | return `0` after applying earlier rows | [x] |
| E19 | `cp_inflate` | null input pointer with a positive input length | no null check; process receives `SIGSEGV` when input is read | [x] |
| E20 | `cp_inflate` | zero input length | no length rejection; first bit read reaches an assertion failure (`SIGABRT`) | [x] |
| E21 | `cp_inflate` | null output pointer and a literal-producing stream with zero output capacity | return `0` through E13 before writing | [x] |
| E22 | `unfilter` | null `raw` with `h > 0` | no null check; process receives `SIGSEGV` when filter byte is read | [x] |
| E23 | `unfilter` | null `raw` with `h == 0`, `w == 0`, and `bpp == 0` | return `1`; no dereference occurs on this implementation | [x] |
| E24 | `unfilter` | filter byte `5` (one past the accepted `0..=4` range) | return `0` (same first/later distinction as E17/E18) | [x] |

Rows E01, E02, E05, and E07 are source-level assertions whose failing states
cannot be independently constructed through the exported ABI without first
invoking C undefined behavior or corrupting private `cp_state_t` storage. They
remain in the table because omitting internal assertions would make the source
inventory incomplete.
