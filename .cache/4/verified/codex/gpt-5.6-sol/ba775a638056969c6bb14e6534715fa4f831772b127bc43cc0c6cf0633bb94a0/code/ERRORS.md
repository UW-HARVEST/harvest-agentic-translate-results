# Error Surface

The rows below expand each macro branch once per generated public function.
There are no C assertions, error enums, `RETURN_ERROR` uses, or range-rejection
branches. Unchecked pointer dereferences (`array_*_get`, string arguments to
`create_*`, and malformed container internals) have undefined C behavior and
therefore no stable result to compare.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `array_int_create` | allocation of array metadata fails | `NULL` | [x] |
| 2 | `array_double_create` | allocation of array metadata fails | `NULL` | [x] |
| 3 | `array_item_t_create` | allocation of array metadata fails | `NULL` | [x] |
| 4 | `array_order_t_create` | allocation of array metadata fails | `NULL` | [x] |
| 5 | `array_int_create` | allocation of `sizeof(int) * capacity` fails | frees metadata and returns `NULL` | [x] |
| 6 | `array_double_create` | allocation of `sizeof(double) * capacity` fails | frees metadata and returns `NULL` | [x] |
| 7 | `array_item_t_create` | allocation of `sizeof(item_t) * capacity` fails | frees metadata and returns `NULL` | [x] |
| 8 | `array_order_t_create` | allocation of `sizeof(order_t) * capacity` fails | frees metadata and returns `NULL` | [x] |
| 9 | `array_int_push` | `arr == NULL` | `-1` | [x] |
| 10 | `array_double_push` | `arr == NULL` | `-1` | [x] |
| 11 | `array_item_t_push` | `arr == NULL` | `-1` | [x] |
| 12 | `array_order_t_push` | `arr == NULL` | `-1` | [x] |
| 13 | `array_int_push` | full array and `realloc` fails | leaves recorded fields unchanged and returns `-1` | [x] |
| 14 | `array_double_push` | full array and `realloc` fails | leaves recorded fields unchanged and returns `-1` | [x] |
| 15 | `array_item_t_push` | full array and `realloc` fails | leaves recorded fields unchanged and returns `-1` | [x] |
| 16 | `array_order_t_push` | full array and `realloc` fails | leaves recorded fields unchanged and returns `-1` | [x] |
| 17 | `list_int_create` | allocation of list metadata fails | `NULL` | [x] |
| 18 | `list_double_create` | allocation of list metadata fails | `NULL` | [x] |
| 19 | `list_item_t_create` | allocation of list metadata fails | `NULL` | [x] |
| 20 | `list_order_t_create` | allocation of list metadata fails | `NULL` | [x] |
| 21 | `list_int_append` | `list == NULL` | `-1` | [x] |
| 22 | `list_double_append` | `list == NULL` | `-1` | [x] |
| 23 | `list_item_t_append` | `list == NULL` | `-1` | [x] |
| 24 | `list_order_t_append` | `list == NULL` | `-1` | [x] |
| 25 | `list_int_append` | node allocation fails | list unchanged and `-1` | [x] |
| 26 | `list_double_append` | node allocation fails | list unchanged and `-1` | [x] |
| 27 | `list_item_t_append` | node allocation fails | list unchanged and `-1` | [x] |
| 28 | `list_order_t_append` | node allocation fails | list unchanged and `-1` | [x] |
| 29 | `list_int_prepend` | `list == NULL` | `-1` | [x] |
| 30 | `list_double_prepend` | `list == NULL` | `-1` | [x] |
| 31 | `list_item_t_prepend` | `list == NULL` | `-1` | [x] |
| 32 | `list_order_t_prepend` | `list == NULL` | `-1` | [x] |
| 33 | `list_int_prepend` | node allocation fails | list unchanged and `-1` | [x] |
| 34 | `list_double_prepend` | node allocation fails | list unchanged and `-1` | [x] |
| 35 | `list_item_t_prepend` | node allocation fails | list unchanged and `-1` | [x] |
| 36 | `list_order_t_prepend` | node allocation fails | list unchanged and `-1` | [x] |
| 37 | `array_int_destroy` | `arr == NULL` | no-op | [x] |
| 38 | `array_double_destroy` | `arr == NULL` | no-op | [x] |
| 39 | `array_item_t_destroy` | `arr == NULL` | no-op | [x] |
| 40 | `array_order_t_destroy` | `arr == NULL` | no-op | [x] |
| 41 | `array_int_size` | `arr == NULL` | `0` | [x] |
| 42 | `array_double_size` | `arr == NULL` | `0` | [x] |
| 43 | `array_item_t_size` | `arr == NULL` | `0` | [x] |
| 44 | `array_order_t_size` | `arr == NULL` | `0` | [x] |
| 45 | `array_int_clear` | `arr == NULL` | no-op | [x] |
| 46 | `array_double_clear` | `arr == NULL` | no-op | [x] |
| 47 | `array_item_t_clear` | `arr == NULL` | no-op | [x] |
| 48 | `array_order_t_clear` | `arr == NULL` | no-op | [x] |
| 49 | `list_int_destroy` | `list == NULL` | no-op | [x] |
| 50 | `list_double_destroy` | `list == NULL` | no-op | [x] |
| 51 | `list_item_t_destroy` | `list == NULL` | no-op | [x] |
| 52 | `list_order_t_destroy` | `list == NULL` | no-op | [x] |
| 53 | `list_int_size` | `list == NULL` | `0` | [x] |
| 54 | `list_double_size` | `list == NULL` | `0` | [x] |
| 55 | `list_item_t_size` | `list == NULL` | `0` | [x] |
| 56 | `list_order_t_size` | `list == NULL` | `0` | [x] |
| 57 | `list_int_clear` | `list == NULL` | no-op | [x] |
| 58 | `list_double_clear` | `list == NULL` | no-op | [x] |
| 59 | `list_item_t_clear` | `list == NULL` | no-op | [x] |
| 60 | `list_order_t_clear` | `list == NULL` | no-op | [x] |
| 61 | `calculate_inventory_stats` | `items == NULL` | prints `No items in inventory\n`, returns | [x] |
| 62 | `calculate_inventory_stats` | `items->size == 0` | prints `No items in inventory\n`, returns | [x] |
| 63 | `calculate_order_stats` | `orders == NULL` | prints `No orders to analyze\n`, returns | [x] |
| 64 | `calculate_order_stats` | `orders->size == 0` | prints `No orders to analyze\n`, returns | [x] |
| 65 | `find_items_by_category` | `items == NULL` | no output, returns | [x] |
| 66 | `find_items_by_category` | `category == NULL` | no output, returns | [x] |
| 67 | `find_expensive_items` | `items == NULL` | no output, returns | [x] |

The `MAX_NAME_LENGTH` (64) and `MAX_CATEGORY_LENGTH` (32) constants do not
reject input. `create_item` and `create_order` truncate to 63 and 31 bytes
respectively and force a trailing null; those valid behaviors are represented
in `CONFIGS.md`.

