# CONFIGS.md — configuration-surface table (Phase B)

The valid-input mirror of `ERRORS.md`. The axes below are the ones the C code
actually branches on, read out of the sources rather than guessed:

**Build-time configuration.** `Cargo.toml` declares **no `[features]`** (and no
`cfg`-gated code exists in the crate), and `c_src/CMakeLists.txt` declares no
options, no `#ifdef`-driven variants, and no compile definitions — it just
compiles `src/main.c` + `src/inventory.c` with `include/` on the header path. So
there is exactly **one** build configuration: the default. `cargo check`/`cargo
test` are still run with `--no-default-features` and with `--all-features` to
prove the point (both are identical to the default here).

**Runtime axes** (from the sources):

| axis | values the C code distinguishes | where |
|------|--------------------------------|-------|
| element type instantiation | `int` (4 B), `double` (8 B), `item_t` (120 B), `order_t` (80 B) | `DEFINE_ARRAY`/`DEFINE_LIST` in `inventory.c` |
| container kind | dynamic array (contiguous, doubling) vs singly linked list (head/tail) | `generic_containers.h` |
| `initial_capacity` | `0` → forced to 16 (`initial_capacity > 0 ? initial_capacity : 16`), `1`, exact fit, larger than needed | `array_TYPE_create` |
| growth | `size < capacity` (no realloc) vs `size >= capacity` (`capacity *= 2` + `realloc`), repeated doublings | `array_TYPE_push` |
| list insertion order | `append` into empty (`if (!list->head)`) vs non-empty; `prepend` into empty (`if (!list->tail)`) vs non-empty; interleaved | `list_TYPE_append`/`_prepend` |
| reuse after `clear` | `size = 0` with capacity kept (array); `head = tail = NULL` (list), so the next insert takes the "empty" branch | `*_clear` |
| element count | 0, 1, many (crossing several doublings) | all iterators |
| numeric values | `int`: 0, `INT_MIN`, `INT_MAX`, random; `double`: 0.0, −0.0, ±inf, ±NaN, subnormal, 1e±308, tie-rounding values, random bit patterns | `%d`/`%.1f`/`%.2f` formatting, comparisons |
| string shape | length 0, < field, exactly field−1, exactly field, longer (truncation), non-ASCII bytes | `strncpy` in `create_item`/`create_order`, `%s`, `strcmp` |
| stats data shape | first element (seeds `min_price`), all-positive vs all-negative prices (`max_price` starts at `0.0`), NaN present, `quantity` summing to 0 (division by zero), `int` overflow of `total_items`, `min_order` seeded to `-1.0` with negative amounts | `calculate_inventory_stats`, `calculate_order_stats` |
| match count | 0 / some / all matches | `find_items_by_category`, `find_expensive_items` |
| stdin shape (`main`) | each menu value 1–7, sequences, invalid text, out-of-range values, `long`-overflowing values, leading whitespace, trailing garbage, embedded NUL, CRLF, line > 255 bytes, missing trailing newline, EOF | `main` loop |
| allocator interop | container allocated by the C `.so` consumed by the Rust `.so` and vice versa (same `malloc` heap, same `#[repr(C)]` layout) | both |

Every row below is driven through the exported symbols of **both** `.so`s with
many randomized inputs (SplitMix64, fixed seeds, 200–5000 cases per row) and the
results — return values, mutated container state read back field by field, and
captured `stdout` bytes — are compared for equality. Rows are checked off only
after passing across the whole randomized set.

## Array entry points (`array_{int,double,item_t,order_t}_*` — every row runs on all four instantiations)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B1 | `create`, `size`, `destroy` | `initial_capacity = 0` → capacity forced to 16; empty array state (`size`, `capacity`, `data != NULL`) | [x] |
| B2 | `create`, `push`, `get`, `size` | `initial_capacity = 16`, push exactly 16 → no realloc; read back every index | [x] |
| B3 | `create`, `push`, `get`, `size` | `initial_capacity = 16`, push 17 → one doubling to 32; verify all 17 elements survive | [x] |
| B4 | `create`, `push`, `get`, `size` | `initial_capacity = 1`, push 1/2/3 → capacity sequence 1 → 2 → 4 | [x] |
| B5 | `create`, `push`, `get`, `size` | `initial_capacity = 1`, push 500 random values → 9 doublings, capacity 512 | [x] |
| B6 | `create`, `push`, `get` | `initial_capacity` random in 1..64, push random count 0..300, random values (including type extremes/specials) | [x] |
| B7 | `create`, `push`, `clear`, `size`, `get` | `clear` on a populated array: `size = 0`, capacity kept, slots still readable via `get` (`size <= index < capacity`) | [x] |
| B8 | `create`, `push`, `clear`, `push` | reuse after `clear`: refill from index 0, no further growth while under the retained capacity | [x] |
| B9 | `create`, `push`, `get` | element values at the extremes: `int` `{0, 1, -1, INT_MIN, INT_MAX}`; `double` `{0.0, -0.0, ±inf, NaN, 5e-324, 1.7976931348623157e308}`; `item_t`/`order_t` built from random strings + extreme numbers | [x] |
| B10 | `create`+`push` in one library, `size`/`get` in the other | interop: C-built array read by Rust exports and vice versa (identical layout + libc heap) | [x] |

## List entry points (`list_{int,double,item_t,order_t}_*` — every row runs on all four instantiations)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B11 | `create`, `size`, `destroy` | freshly created list: `head == NULL`, `tail == NULL`, `size == 0` | [x] |
| B12 | `create`, `append`, `size` | `append` into empty list (`!list->head` branch) then into non-empty; 1 and many elements; traverse the node chain | [x] |
| B13 | `create`, `prepend`, `size` | `prepend` into empty list (`!list->tail` branch) then into non-empty; resulting order is reversed; `tail` stays the first-prepended node | [x] |
| B14 | `create`, `append`, `prepend` | randomly interleaved append/prepend (300 ops) → full chain, `head`, `tail`, `size` compared node by node | [x] |
| B15 | `create`, `append`, `clear`, `append` | reuse after `clear`: `head`/`tail` NULL again so the next `append` takes the empty branch | [x] |
| B16 | `create`, `append`, `prepend`, `size`, `clear`, `destroy` | randomized op sequences (append/prepend/size/clear mixed, 500 ops) with the same script driven against both libraries | [x] |
| B17 | `create`+`append` in one library, `size`/traversal in the other | interop: C-built list read by Rust exports and vice versa | [x] |

## `inventory.c` entry points

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B18 | `create_item` | random ASCII `name` of length 0..80 × `category` of length 0..40 → covers shorter-than-field, exact fit, and truncation with the forced NUL; all 120 bytes of the result compared field by field | [x] |
| B19 | `create_item` | boundary lengths exactly: name 62/63/64/65, category 30/31/32/33 | [x] |
| B20 | `create_item` | non-ASCII/high-byte and whitespace-containing strings; `id`/`quantity` `{0, ±1, INT_MIN, INT_MAX}`; `price` specials | [x] |
| B21 | `create_order` | random `customer_name` 0..80 chars, `customer_id` extremes, `total_amount` random + specials | [x] |
| B22 | `print_item` | random `item_t` (by value across the ABI) → captured stdout compared byte for byte | [x] |
| B23 | `print_item` | `price` shapes: 0.0, −0.0, ±inf, ±NaN, 1e308, 5e-324, tie cases (0.125, 0.375, 1.005, 2.675, 0.005), values needing >300 digits | [x] |
| B24 | `print_order` | random `order_t` (by value) + the same `total_amount` shapes | [x] |
| B25 | `calculate_inventory_stats` | array of 1 item; of 2; of 10; of 100 random items (random prices/quantities) — exercises `min_price` seeded from `data[0]`, both comparison branches | [x] |
| B26 | `calculate_inventory_stats` | all prices negative (so `max_price` never leaves its `0.0` seed) and all positive; mixed signs | [x] |
| B27 | `calculate_inventory_stats` | `quantity` values summing to 0 → `total_value / total_items` divides by zero (`-nan`/`inf`); also `quantity` sums that overflow `int` | [x] |
| B28 | `calculate_inventory_stats` | NaN / ±inf prices mixed in (every `>`/`<` comparison false for NaN) | [x] |
| B29 | `calculate_order_stats` | list of 1 order; of 8; of 100 random orders; built by `append` and by `prepend` (different traversal order → different `min`/`max` walk) | [x] |
| B30 | `calculate_order_stats` | all `total_amount` negative → `min_order < 0` stays true, so the printed "smallest" is the *last* order (C quirk, must be reproduced); all zero; mixed | [x] |
| B31 | `calculate_order_stats` | NaN / ±inf amounts mixed in | [x] |
| B32 | `find_items_by_category` | matches: none / one / some / all, over arrays of 0..50 random items; random category strings | [x] |
| B33 | `find_items_by_category` | category shapes: empty string, exactly 31 chars, 32+ chars (can never match a truncated field), non-ASCII bytes, prefix-of-another-category | [x] |
| B34 | `find_items_by_category` | non-NULL but **empty** array (`size == 0`): header + "no items" message, unlike the NULL case which prints nothing | [x] |
| B35 | `find_expensive_items` | `min_price` below all / between / above all / `-inf` / `+inf` / NaN / `-0.0` vs `0.0`, over lists of 0..50 random items | [x] |

## `main.c` entry points

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B36 | `print_menu` | no inputs; exact byte stream | [x] |
| B37 | `demo_integer_containers` | fixed data path: array capacity 10 with 5 pushes, list of 5, `%d`/`%.2f`/`%lld` output | [x] |
| B38 | `demo_double_containers` | fixed data path that **crosses a realloc** (capacity 5, 7 pushes) and prints `%.1f°C` (multi-byte literal) and `%.2f` | [x] |
| B39 | `demo_inventory_array` | 10 `item_t`s in a capacity-20 array; `print_item` loop, stats, two `find_items_by_category` calls, low-stock filter | [x] |
| B40 | `demo_order_list` | 8 orders appended; `print_order` loop, stats, large-order filter | [x] |
| B41 | `demo_mixed_operations` | 5 items pushed into an array **and** appended to a list; both iterations + `%s: $%.2f` lines | [x] |
| B42 | all six `main.c` functions | called repeatedly / in sequence in one process (no cross-call state leakage) | [x] |
| B43 | `main` (via the `driver` / `driver_c` executables) | each menu choice 1..7 individually | [x] |
| B44 | `main` | sequences and repeats: `6,7`, `1..5,7`, `3,3,3,7`, all choices then EOF | [x] |
| B45 | `main` | accepted-but-unusual numeric spellings: `" 3"`, `"\t7"`, `"+3"`, `"007"`, `"3junk"`, `"3 4"`, `"3\r"` | [x] |
| B46 | `main` | line shapes: no trailing newline at EOF, empty line, 300-byte and 600-byte lines (`fgets` 255-byte chunking), embedded NUL | [x] |
| B47 | `main` | exit status and full stdout compared for every scenario above | [x] |

## Row → test mapping (all rows verified in both the debug and release profiles)

| rows | test |
|------|------|
| B1 | `containers::b1_array_create_capacities` |
| B2, B3, B4, B5 | `containers::b2345_array_growth_ladder` |
| B6 | `containers::b6_array_randomized` (200 random shapes × 4 types) |
| B7, B8 | `containers::b78_array_clear_and_reuse` |
| B9 | `containers::b9_array_extreme_values` |
| B10 | `containers::b10_array_cross_library_interop` |
| B11 | `containers::b11_list_create` |
| B12 | `containers::b12_list_append` |
| B13 | `containers::b13_list_prepend` |
| B14 | `containers::b14_list_interleaved` |
| B15 | `containers::b15_list_clear_and_reuse` |
| B16 | `containers::b16_list_random_scripts` (500-op scripts × 4 types) |
| B17 | `containers::b17_list_cross_library_interop` |
| B18 | `inventory_funcs::create_item_random_strings` (3000 cases) |
| B19 | `inventory_funcs::create_item_boundary_lengths` |
| B20 | `inventory_funcs::create_item_non_ascii_and_specials` |
| B21 | `inventory_funcs::create_order_random` (3000 cases) |
| B22 | `inventory_funcs::print_item_random`, `print_item_string_shapes` |
| B23 | `inventory_funcs::print_item_price_shapes`, `print_item_formatting_stress` (20 000 values) |
| B24 | `inventory_funcs::print_order_random_and_shapes`, `print_order_formatting_stress` (10 000 values) |
| B25 | `inventory_funcs::inventory_stats_counts_and_random_data` |
| B26 | `inventory_funcs::inventory_stats_price_sign_shapes` |
| B27 | `inventory_funcs::stats_division_edge_cases`, `stats_quantity_overflow` |
| B28 | `inventory_funcs::inventory_stats_special_prices`, `inventory_stats_nan_inf_exhaustive`, `stats_nan_positions_in_long_sums` |
| B29 | `inventory_funcs::order_stats_counts_and_random_data` (append and prepend) |
| B30 | `inventory_funcs::order_stats_negative_and_zero_amounts` |
| B31 | `inventory_funcs::order_stats_special_amounts`, `order_stats_nan_inf_exhaustive` |
| B32 | `inventory_funcs::find_by_category_match_counts` |
| B33 | `inventory_funcs::find_by_category_string_shapes` |
| B34 | `inventory_funcs::find_by_category_empty_array`, `errors::empty_containers_everywhere` |
| B35 | `inventory_funcs::find_expensive_thresholds`, `find_expensive_header_formatting_stress` |
| B36 | `demos::b36_print_menu` |
| B37 | `demos::b37_demo_integer_containers` |
| B38 | `demos::b38_demo_double_containers` |
| B39 | `demos::b39_demo_inventory_array` |
| B40 | `demos::b40_demo_order_list` |
| B41 | `demos::b41_demo_mixed_operations` |
| B42 | `demos::b42_all_demos_in_sequence` (plus 10 randomized call orders) |
| B43–B47 | `driver_e2e::stdin_scenarios` (46 scenarios), `driver_e2e::main_symbol_scenarios` (the exported `main` of both `.so`s, in a forked child), `driver_e2e::random_stdin_sessions` (60 random sessions) |

Every row above is also driven through the *cross-library* path where it makes
sense: a container built by the C `.so` is handed to the Rust entry point and
vice versa (`compare_inventory_stats`, `compare_order_stats`,
`compare_find_expensive`, `b10`, `b17`), which only works because the
`#[repr(C)]` layouts and the libc allocator are shared.

### Divergence found and fixed while working through these rows

`inventory_stats_special_prices` / `order_stats_special_amounts` failed in the
**release** profile only: when two NaNs meet in `total_value +=` /
`total_revenue +=`, the optimizer's operand swap made the wrong NaN survive and
printed `$-nan` where C prints `$nan`. Fixed in `src/cio.rs::fadd_c`; see note D
of `ERRORS.md`. This is why every row is now run in both profiles.
