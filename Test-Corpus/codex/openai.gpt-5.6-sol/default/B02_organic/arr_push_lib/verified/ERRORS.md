# Error surface

The C source has no error enum, `RETURN_ERROR`, `return NULL` failure branch, or
documented range-rejection API. Its explicit rejection mechanism is `assert`;
the rows below enumerate every distinct active assertion mechanically found in
`src/lib.c`. Null branches that are accepted behavior are covered in
`CONFIGS.md`, not classified as errors.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | internal `stbds_make_hash_index` via hash APIs | constructed slot count makes `used_count_threshold + tombstone_count_threshold >= slot_count` | `assert` abort | [x] |
| 2 | `stbds_hmput_key` | after attempted growth, `(size_t)i + 1 > stbds_arrcap(a)` | `assert` abort | [x] |
| 3 | `stbds_hmdel_key` | located slot is not less than `table->slot_count` | `assert` abort | [x] |
| 4 | `stbds_hmdel_key` | decrementing `used_count` would make it negative (asserted as `used_count >= 0`) | `assert` abort | [x] |
| 5 | `stbds_hmdel_key` | moving the final element causes its replacement lookup to return `slot < 0` | `assert` abort | [x] |
| 6 | `stbds_hmdel_key` | moved element's hash bucket index is not equal to `final_index` | `assert` abort | [x] |
| 7 | `stbds_stralloc` | after block allocation, `strlen(str) + 1 > arena->remaining` on the ordinary-block path | `assert` abort | [x] |
| 8 | `arr_push` | newly initialized local array reports nonzero length | `assert` abort | [x] |

All eight are internal consistency assertions. No well-formed FFI argument can
directly select these states; Phase C tests exercise the public boundary
conditions and the operation sequences that reach each assertion with its
predicate satisfied, while comparing exact C/Rust sentinels for accepted null,
zero, oversized, and out-of-range-mode inputs.
