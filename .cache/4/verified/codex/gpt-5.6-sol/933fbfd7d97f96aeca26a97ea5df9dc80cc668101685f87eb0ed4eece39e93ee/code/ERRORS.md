# Error Surface

The C API has no error enum and no `RETURN_ERROR` macro. The rows below are
the complete set of sentinel-return, null-rejection, and assertion sites found
mechanically in `c_src/src/lib.c`. Assertions guard private data-structure
invariants rather than documented caller inputs.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `stbds_make_hash_index` (private) | `used_count_threshold + tombstone_count_threshold >= slot_count` after threshold calculation | `assert` abort | [x] |
| 2 | `stbds_hm_find_slot` (private) | first, non-wrapped bucket scan reaches `STBDS_HASH_EMPTY` before finding the key | returns `-1` | [x] |
| 3 | `stbds_hm_find_slot` (private) | wrapped bucket scan reaches `STBDS_HASH_EMPTY` before finding the key | returns `-1` | [x] |
| 4 | `stbds_hmget_key_ts` | `a == NULL` | creates a zero default element, sets `*temp = -1`, returns a non-null map pointer | [x] |
| 5 | `stbds_hmget_key_ts` | map exists but its hash table is null | sets `*temp = -1`, returns the same map pointer | [x] |
| 6 | `stbds_hmget_key_ts` | hash table exists but key lookup returns a negative slot | sets `*temp = -1`, returns the same map pointer | [x] |
| 7 | `stbds_hmdel_key` | `a == NULL` | returns `NULL` | [x] |
| 8 | `stbds_hmdel_key` | map exists but its hash table is null | sets header temp to `0`, returns the same map pointer | [x] |
| 9 | `stbds_hmdel_key` | key lookup returns a negative slot | sets header temp to `0`, returns the same map pointer | [x] |
| 10 | `stbds_hmput_key` | post-grow `(size_t)i + 1 > arrcap(a)` | `assert` abort | [x] |
| 11 | `stbds_hmdel_key` | located slot is outside `table->slot_count` | `assert` abort | [x] |
| 12 | `stbds_hmdel_key` | `table->used_count < 0` after decrement | `assert` abort; condition is unreachable because the field is unsigned | [x] |
| 13 | `stbds_hmdel_key` | moved final element cannot be found in the hash index | `assert` abort | [x] |
| 14 | `stbds_hmdel_key` | moved element's bucket index is not `final_index` | `assert` abort | [x] |
| 15 | `stbds_stralloc` | normal block allocation leaves `len > a->remaining` | `assert` abort | [x] |
| 16 | `arr_push` | newly initialized local array has nonzero length before the loop | `assert` abort | [x] |

## FFI Boundary Audit

These functions are pointer-based C primitives. Except for the explicit null
branches above and `stbds_hash_bytes(NULL, 0, seed)`, null pointers, invalid
allocation headers, unterminated strings, impossible lengths, zero element
sizes used with hash maps, and invalid arena/map pointers invoke C undefined
behavior rather than returning an error. They therefore have no byte-stable C
result to compare. Tests cover every defined null/zero boundary and integer
mode values outside the named constants.
