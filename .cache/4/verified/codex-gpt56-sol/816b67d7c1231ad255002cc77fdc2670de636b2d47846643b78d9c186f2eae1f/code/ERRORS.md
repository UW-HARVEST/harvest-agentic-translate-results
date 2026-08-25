# Error Surface

Derived from every `assert`, explicit error branch, and public-boundary
condition in `c_src/src/lib.c`. Rows 1-10 are internal assertions in `static`
helpers. They are not separately callable through the shared-library ABI;
where reachable from `pinflate`, tests compare termination in isolated child
processes. Assertions described as invariant-only cannot be falsified by a
well-defined call to the sole exported function.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 [x] | `cp_ptr` via `pinflate` | Internal state reaches `cp_ptr` with `(s->bits_left & 7) != 0` | `assert` abort; invariant-only because `cp_stored` byte-aligns before the call |
| 2 [x] | `cp_peak_bits` via `pinflate` | Loading a full word makes `s->word_index > s->word_count` | `assert` abort; invariant-only because the load is guarded by `word_index < word_count` |
| 3 [x] | `cp_consume_bits` via `pinflate` | `s->count < num_bits_to_read` after refill | `assert` abort |
| 4 [x] | `cp_read_bits` via `pinflate` | A caller requests `num_bits_to_read > 32` | `assert` abort; invariant-only because all call sites request at most 16 bits |
| 5 [x] | `cp_read_bits` via `pinflate` | A caller requests `num_bits_to_read < 0` | `assert` abort; invariant-only because all call-site values are nonnegative |
| 6 [x] | `cp_read_bits` via `pinflate` | `s->bits_left <= 0` before a read | `assert` abort |
| 7 [x] | `cp_read_bits` via `pinflate` | `s->count > 64` before a read | `assert` abort; invariant-only because refill adds at most one 32-bit word |
| 8 [x] | `cp_read_bits` via `pinflate` | `(s->bits_left + s->count) - num_bits_to_read < 0` | `assert` abort |
| 9 [x] | `cp_build` via `pinflate` | A nonzero Huffman code length is `>= 16` | `assert` abort; invariant-only because fixed lengths are at most 9 and dynamic lengths are read from 3 bits |
| 10 [x] | `cp_decode` via `pinflate` | The selected tree key does not prefix-match `search` | `assert` abort for an invalid/incomplete Huffman stream |
| 11 [x] | `cp_stored` via `pinflate` | Stored-block `LEN != (uint16_t)~NLEN` | return `0`; `cp_error_reason` = `Failed to find LEN and NLEN as complements within stored (uncompressed) stream.` |
| 12 [x] | `cp_stored` via `pinflate` | After `LEN`/`NLEN`, `s->bits_left / 8 > LEN` | return `0`; `cp_error_reason` = `Stored block extends beyond end of input stream.` |
| 13 [x] | `cp_block` via `pinflate` | Literal output has `s->out + 1 > s->out_end` | return `0`; `cp_error_reason` = `Attempted to overwrite out buffer while outputting a symbol.` |
| 14 [x] | `cp_block` via `pinflate` | Back-reference has `s->out - backwards_distance < s->begin` | return `0`; `cp_error_reason` = `Attempted to write before out buffer (invalid backwards distance).` |
| 15 [x] | `cp_block` via `pinflate` | Back-reference has `s->out + length > s->out_end` | return `0`; `cp_error_reason` = `Attempted to overwrite out buffer while outputting a string.` |
| 16 [x] | `pinflate` | Three-bit block header has `BTYPE == 3` | return `0`; `cp_error_reason` = `Detected unknown block type within input stream.` |
| 17 [x] | `pinflate` | `in == NULL` with a positive `in_bytes` that requires input access | no null check; C has undefined behavior, normally process termination |
| 18 [x] | `pinflate` | `out == NULL` and the stream emits at least one byte | no null check; C has undefined behavior, normally process termination |
| 19 [x] | `pinflate` | `in_bytes == 0` | reaches row 6 and aborts |
| 20 [x] | `pinflate` | `out_bytes == 0` and a compressed stream emits a literal | same exact rejection as row 13 |
| 21 [x] | `pinflate` | `in_bytes < 0` | no range check; C performs invalid arithmetic/access (undefined behavior or assertion abort) |
| 22 [x] | `pinflate` | `out_bytes < 0` | no range check; pointer arithmetic is undefined and later output rejects or faults |
| 23 [x] | `pinflate` | `in_bytes > INT_MAX / 8` (one step past the largest value whose bit count fits in `int`) | signed-overflow undefined behavior in `in_bytes * 8`; no error sentinel |

There are no enum parameters and no documented public min/max constants. The
only public integer type is `int`; therefore no out-of-range enum case exists.

All rows are covered by `tests/differential.rs`. Reachable sentinels compare
the exact return and error text. Process-failure cases run each shared object
in an isolated child. Invariant-only rows are checked mechanically against
both implementations because no exported C entry point can construct those
states.
