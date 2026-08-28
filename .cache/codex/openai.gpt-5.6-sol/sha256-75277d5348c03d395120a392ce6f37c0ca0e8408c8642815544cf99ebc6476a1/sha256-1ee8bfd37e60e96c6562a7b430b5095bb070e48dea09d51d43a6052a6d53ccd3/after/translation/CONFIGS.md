# Configuration Surface

Mechanical source scan covered the public header and every `if`/loop branch in
`../c_src/src/lib.c`. There are no runtime options, modes, flags, feature
macros, or alternate public entry points. `merge_sort` is both the lowest-level
and only public API. Its ABI has one fixed element shape:
`unsigned long long texture_id` followed by `int sort_bits`.

For every row, `a` and `b` are distinct, aligned buffers large enough for
`size` elements. Both buffers are compared byte-for-byte after each call
because `a` is the output and `b` is caller-visible scratch storage.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `merge_sort` | `size == 0`; empty input, so copy and recursion do no element work | [x] |
| 2 | `merge_sort` | `size == 1`; copy one element and take the recursion base case | [x] |
| 3 | `merge_sort` | `size == 2`; left `sort_bits <` right, taking the left/comparator branch before right exhaustion | [x] |
| 4 | `merge_sort` | `size == 2`; equal `sort_bits` with arbitrary/opposing `texture_id`, exercising equality and stability | [x] |
| 5 | `merge_sort` | `size == 2`; left `sort_bits >` right, taking the right branch before left exhaustion | [x] |
| 6 | `merge_sort` | odd `size >= 3`; mixed ordering and duplicates, exercising unequal recursive partitions | [x] |
| 7 | `merge_sort` | even `size >= 4`; mixed ordering and duplicates, exercising equal recursive partitions | [x] |
| 8 | `merge_sort` | many elements with all equal `sort_bits`; arbitrary `texture_id`, exercising stable left selection and half exhaustion | [x] |
| 9 | `merge_sort` | many elements spanning `INT_MIN`, `INT_MAX`, zero, duplicate keys, and full-width texture IDs | [x] |
