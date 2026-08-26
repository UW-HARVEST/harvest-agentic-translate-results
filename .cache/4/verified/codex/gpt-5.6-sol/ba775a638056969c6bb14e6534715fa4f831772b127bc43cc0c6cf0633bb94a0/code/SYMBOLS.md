# Dynamic Symbol Surface

Source command:

```sh
nm -D --defined-only --format=posix c_src/build/libinventory_c.so
```

The C shared object is built from the public implementation unit,
`c_src/src/inventory.c`. CMake's only target is the interactive `driver`
executable, so the shared object is compiled from that unchanged source file
with the same include directory and position-independent-code setting.

| # | C symbol | Rust export |
|---|----------|-------------|
| 1 | `array_double_clear` | [x] |
| 2 | `array_double_create` | [x] |
| 3 | `array_double_destroy` | [x] |
| 4 | `array_double_get` | [x] |
| 5 | `array_double_push` | [x] |
| 6 | `array_double_size` | [x] |
| 7 | `array_int_clear` | [x] |
| 8 | `array_int_create` | [x] |
| 9 | `array_int_destroy` | [x] |
| 10 | `array_int_get` | [x] |
| 11 | `array_int_push` | [x] |
| 12 | `array_int_size` | [x] |
| 13 | `array_item_t_clear` | [x] |
| 14 | `array_item_t_create` | [x] |
| 15 | `array_item_t_destroy` | [x] |
| 16 | `array_item_t_get` | [x] |
| 17 | `array_item_t_push` | [x] |
| 18 | `array_item_t_size` | [x] |
| 19 | `array_order_t_clear` | [x] |
| 20 | `array_order_t_create` | [x] |
| 21 | `array_order_t_destroy` | [x] |
| 22 | `array_order_t_get` | [x] |
| 23 | `array_order_t_push` | [x] |
| 24 | `array_order_t_size` | [x] |
| 25 | `calculate_inventory_stats` | [x] |
| 26 | `calculate_order_stats` | [x] |
| 27 | `create_item` | [x] |
| 28 | `create_order` | [x] |
| 29 | `find_expensive_items` | [x] |
| 30 | `find_items_by_category` | [x] |
| 31 | `list_double_append` | [x] |
| 32 | `list_double_clear` | [x] |
| 33 | `list_double_create` | [x] |
| 34 | `list_double_destroy` | [x] |
| 35 | `list_double_prepend` | [x] |
| 36 | `list_double_size` | [x] |
| 37 | `list_int_append` | [x] |
| 38 | `list_int_clear` | [x] |
| 39 | `list_int_create` | [x] |
| 40 | `list_int_destroy` | [x] |
| 41 | `list_int_prepend` | [x] |
| 42 | `list_int_size` | [x] |
| 43 | `list_item_t_append` | [x] |
| 44 | `list_item_t_clear` | [x] |
| 45 | `list_item_t_create` | [x] |
| 46 | `list_item_t_destroy` | [x] |
| 47 | `list_item_t_prepend` | [x] |
| 48 | `list_item_t_size` | [x] |
| 49 | `list_order_t_append` | [x] |
| 50 | `list_order_t_clear` | [x] |
| 51 | `list_order_t_create` | [x] |
| 52 | `list_order_t_destroy` | [x] |
| 53 | `list_order_t_prepend` | [x] |
| 54 | `list_order_t_size` | [x] |
| 55 | `print_item` | [x] |
| 56 | `print_order` | [x] |

Missing C symbols in Rust: **0**.

