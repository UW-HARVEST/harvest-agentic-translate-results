# Configuration-surface table

This table is derived from the public dynamic entry points plus every branch,
limit, and special input shape in `../c_src/src/lib.c`. There are no Cargo
features, compile-time option branches, or binary targets. `Node.active` is
included because `find_node_by_id` returns a writable `Node *` and the C code
branches on that field.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `add_node` | non-full storage; empty name; arbitrary IDs and finite value | [x] |
| 2 | `add_node` | non-full storage; short NUL-terminated name (1–48 bytes) | [x] |
| 3 | `add_node` | non-full storage; name exactly 49 bytes before NUL | [x] |
| 4 | `add_node` | non-full storage; name longer than 49 bytes (stored name is truncated and terminated) | [x] |
| 5 | `add_node`, `find_node_by_id` | duplicate active IDs; lookup returns the first stored match | [x] |
| 6 | `add_node` | insertion that raises count from 99 to the `MAX_NODES` limit of 100 | [x] |
| 7 | `find_node_by_id` | empty storage / absent ID | [x] |
| 8 | `find_node_by_id` | present active ID | [x] |
| 9 | `find_node_by_id` | present ID whose returned node has been changed to `active == 0` | [x] |
| 10 | `get_children_count` | no matching active children (empty, absent parent, or only inactive matches) | [x] |
| 11 | `get_children_count` | exactly one active child | [x] |
| 12 | `get_children_count` | multiple active children, with inactive non-counted siblings | [x] |
| 13 | `calculate_subtree_sum` | missing or inactive requested root | [x] |
| 14 | `calculate_subtree_sum` | active leaf node | [x] |
| 15 | `calculate_subtree_sum` | active root with multiple direct active children | [x] |
| 16 | `calculate_subtree_sum` | multi-level tree; recursion includes active descendants and excludes inactive branches | [x] |
| 17 | `process_string` | empty string (first byte NUL) | [x] |
| 18 | `process_string` | one-byte string | [x] |
| 19 | `process_string` | multi-byte string | [x] |
| 20 | `process_string` | bytes with the high bit set, exercising platform signed-`char` accumulation | [x] |
| 21 | `safe_double_to_int` | finite in-range positive/negative integers and zero | [x] |
| 22 | `safe_double_to_int` | finite in-range fractions, truncated toward zero | [x] |
| 23 | `safe_double_to_int` | exact `INT_MIN`/`INT_MAX` boundaries and adjacent representable in-range values | [x] |
| 24 | `maxnmin` | `param1 % 6 + 1` selects root node 1 (name + full subtree branches) | [x] |
| 25 | `maxnmin` | first selection is internal node 2 or 3 | [x] |
| 26 | `maxnmin` | first selection is leaf node 4, 5, or 6 | [x] |
| 27 | `maxnmin` | negative `param1` remainder selects no node, skipping first-node contribution | [x] |
| 28 | `maxnmin` | second selection found; `param3` is zero, positive, or negative and product remains in range | [x] |
| 29 | `maxnmin` | negative `param2` remainder selects no second node | [x] |
| 30 | `maxnmin` | second-node multiplication exceeds positive or negative integer range and clamps | [x] |
| 31 | `maxnmin` | `param4 % 3 + 1` selects parent 1/2 (two children), parent 3 (one child), or a missing parent (zero children) | [x] |
| 32 | `maxnmin` | `param3 == -1`; final division produces NaN for zero numerator or signed infinity for nonzero numerator | [x] |
| 33 | `maxnmin` | ordinary nonzero denominator; final calculation is positive, negative, fractional, or zero | [x] |
| 34 | `maxnmin` | final calculation exceeds positive or negative integer range and clamps | [x] |
| 35 | `maxnmin` | integer boundary operands exercise the C build's signed add / `+ 1` behavior before conversion to `double` | [x] |
