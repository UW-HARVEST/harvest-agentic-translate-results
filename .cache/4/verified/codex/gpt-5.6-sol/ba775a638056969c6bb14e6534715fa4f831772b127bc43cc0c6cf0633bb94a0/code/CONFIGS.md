# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
configuration options or preprocessor definitions. The complete valid Cargo
feature set is:

| # | command feature argument | C configuration | status |
|---|--------------------------|-----------------|--------|
| 1 | empty set (`--no-default-features`) | default | [x] |

## Runtime Configurations

Rows are the cross-product the generated C code distinguishes: container kind,
element ABI, empty/nonempty/full state, insertion direction, and boundary
position. Randomized repetitions include zero, signed extrema where defined,
finite/non-finite floating values, and varied struct field values.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `array_int_create`, `array_int_size` | capacity 0 selects default 16; empty `int` array | [x] |
| 2 | `array_double_create`, `array_double_size` | capacity 0 selects default 16; empty `double` array | [x] |
| 3 | `array_item_t_create`, `array_item_t_size` | capacity 0 selects default 16; empty `item_t` array | [x] |
| 4 | `array_order_t_create`, `array_order_t_size` | capacity 0 selects default 16; empty `order_t` array | [x] |
| 5 | `array_int_create` | explicit positive capacities 1 and many | [x] |
| 6 | `array_double_create` | explicit positive capacities 1 and many | [x] |
| 7 | `array_item_t_create` | explicit positive capacities 1 and many | [x] |
| 8 | `array_order_t_create` | explicit positive capacities 1 and many | [x] |
| 9 | `array_int_push`, `array_int_size` | push without growth into empty/nonfull array | [x] |
| 10 | `array_double_push`, `array_double_size` | push without growth into empty/nonfull array | [x] |
| 11 | `array_item_t_push`, `array_item_t_size` | push without growth into empty/nonfull array | [x] |
| 12 | `array_order_t_push`, `array_order_t_size` | push without growth into empty/nonfull array | [x] |
| 13 | `array_int_push` | full array doubles capacity, preserving `int` values | [x] |
| 14 | `array_double_push` | full array doubles capacity, preserving `double` bit patterns | [x] |
| 15 | `array_item_t_push` | full array doubles capacity, preserving `item_t` fields | [x] |
| 16 | `array_order_t_push` | full array doubles capacity, preserving `order_t` fields | [x] |
| 17 | `array_int_get` | first, middle, and last valid indexes | [x] |
| 18 | `array_double_get` | first, middle, and last valid indexes | [x] |
| 19 | `array_item_t_get` | first, middle, and last valid indexes | [x] |
| 20 | `array_order_t_get` | first, middle, and last valid indexes | [x] |
| 21 | `array_int_clear`, `array_int_size` | clear empty and nonempty arrays, then reuse | [x] |
| 22 | `array_double_clear`, `array_double_size` | clear empty and nonempty arrays, then reuse | [x] |
| 23 | `array_item_t_clear`, `array_item_t_size` | clear empty and nonempty arrays, then reuse | [x] |
| 24 | `array_order_t_clear`, `array_order_t_size` | clear empty and nonempty arrays, then reuse | [x] |
| 25 | `array_int_destroy` | destroy allocated empty/nonempty arrays | [x] |
| 26 | `array_double_destroy` | destroy allocated empty/nonempty arrays | [x] |
| 27 | `array_item_t_destroy` | destroy allocated empty/nonempty arrays | [x] |
| 28 | `array_order_t_destroy` | destroy allocated empty/nonempty arrays | [x] |
| 29 | `list_int_create`, `list_int_size` | newly created empty list | [x] |
| 30 | `list_double_create`, `list_double_size` | newly created empty list | [x] |
| 31 | `list_item_t_create`, `list_item_t_size` | newly created empty list | [x] |
| 32 | `list_order_t_create`, `list_order_t_size` | newly created empty list | [x] |
| 33 | `list_int_append` | append to empty and nonempty list | [x] |
| 34 | `list_double_append` | append to empty and nonempty list | [x] |
| 35 | `list_item_t_append` | append to empty and nonempty list | [x] |
| 36 | `list_order_t_append` | append to empty and nonempty list | [x] |
| 37 | `list_int_prepend` | prepend to empty and nonempty list | [x] |
| 38 | `list_double_prepend` | prepend to empty and nonempty list | [x] |
| 39 | `list_item_t_prepend` | prepend to empty and nonempty list | [x] |
| 40 | `list_order_t_prepend` | prepend to empty and nonempty list | [x] |
| 41 | `list_int_append`, `list_int_prepend` | randomized mixed insertion order; head/tail/size traversal | [x] |
| 42 | `list_double_append`, `list_double_prepend` | randomized mixed insertion order; head/tail/size traversal | [x] |
| 43 | `list_item_t_append`, `list_item_t_prepend` | randomized mixed insertion order; head/tail/size traversal | [x] |
| 44 | `list_order_t_append`, `list_order_t_prepend` | randomized mixed insertion order; head/tail/size traversal | [x] |
| 45 | `list_int_clear`, `list_int_size` | clear empty and nonempty lists, then reuse | [x] |
| 46 | `list_double_clear`, `list_double_size` | clear empty and nonempty lists, then reuse | [x] |
| 47 | `list_item_t_clear`, `list_item_t_size` | clear empty and nonempty lists, then reuse | [x] |
| 48 | `list_order_t_clear`, `list_order_t_size` | clear empty and nonempty lists, then reuse | [x] |
| 49 | `list_int_destroy` | destroy allocated empty/nonempty lists | [x] |
| 50 | `list_double_destroy` | destroy allocated empty/nonempty lists | [x] |
| 51 | `list_item_t_destroy` | destroy allocated empty/nonempty lists | [x] |
| 52 | `list_order_t_destroy` | destroy allocated empty/nonempty lists | [x] |
| 53 | `create_item` | name/category lengths 0 and one byte | [x] |
| 54 | `create_item` | name length 62, 63, and greater than 63 | [x] |
| 55 | `create_item` | category length 30, 31, and greater than 31 | [x] |
| 56 | `create_item` | randomized IDs, finite prices, and quantities | [x] |
| 57 | `create_item` | price `-0`, infinities, and NaN payloads | [x] |
| 58 | `create_order` | customer name lengths 0 and one byte | [x] |
| 59 | `create_order` | customer name length 62, 63, and greater than 63 | [x] |
| 60 | `create_order` | randomized customer IDs and finite totals | [x] |
| 61 | `create_order` | total `-0`, infinities, and NaN payloads | [x] |
| 62 | `print_item` | randomized populated fields and C strings | [x] |
| 63 | `print_order` | randomized populated fields and C strings | [x] |
| 64 | `calculate_inventory_stats` | allocated empty array | [x] |
| 65 | `calculate_inventory_stats` | one item; positive, zero, and negative quantity | [x] |
| 66 | `calculate_inventory_stats` | many items; first/middle/last min and max prices | [x] |
| 67 | `calculate_inventory_stats` | all-negative, zero, and positive prices | [x] |
| 68 | `calculate_inventory_stats` | randomized many-item arrays | [x] |
| 69 | `calculate_order_stats` | allocated empty list | [x] |
| 70 | `calculate_order_stats` | one order; negative, zero, and positive total | [x] |
| 71 | `calculate_order_stats` | many orders; first/middle/last extrema | [x] |
| 72 | `calculate_order_stats` | randomized many-order lists | [x] |
| 73 | `find_items_by_category` | allocated empty array and nonnull category | [x] |
| 74 | `find_items_by_category` | nonempty array with no matches | [x] |
| 75 | `find_items_by_category` | one/some/all exact byte-string matches | [x] |
| 76 | `find_items_by_category` | case differences and truncated category boundaries | [x] |
| 77 | `find_expensive_items` | allocated empty list | [x] |
| 78 | `find_expensive_items` | nonempty list with no matching prices | [x] |
| 79 | `find_expensive_items` | one/some/all prices above threshold | [x] |
| 80 | `find_expensive_items` | price exactly equal to threshold | [x] |
| 81 | `find_expensive_items` | negative, infinite, and NaN thresholds | [x] |

