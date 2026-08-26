# ERRORS.md — error-surface table (Phase C)

Every distinct way the C code rejects, refuses, or degenerates on input, grepped
out of `c_src/include/generic_containers.h`, `c_src/src/inventory.c` and
`c_src/src/main.c`. There are no `assert`s, no error enums and no `errno` use in
this library; rejection is expressed as `return NULL`, `return -1`, an early
`return` (sometimes with a message), a fixed message on the "nothing found"
path, or — in `main` — a menu message.

Rows 1–16 come from the `DEFINE_ARRAY(TYPE)` / `DEFINE_LIST(TYPE)` macro bodies,
so each one exists **four times** in the binary (`int`, `double`, `item_t`,
`order_t`). Every such row is tested against all four instantiations of both
libraries.

Legend for "tested": `[x]` = a differential test constructs the condition and
asserts that both `.so`s return the same sentinel / print the same bytes / die
from the same signal. Every row below is `[x]`. Rows whose C behaviour is
undefined (NULL dereference, reading past a field) are asserted as far as they
are deterministic, which the notes at the bottom spell out.

| # | function | trigger (exact invalid input/condition) | expected C result | tested |
|---|----------|------------------------------------------|-------------------|--------|
| 1 | `array_TYPE_create` | `malloc(sizeof(array_TYPE_t))` fails (`if (!arr) return NULL`) | returns `NULL` | `[x]` `errors::malloc_failure_paths` (heap exhausted in a forked child) |
| 2 | `array_TYPE_create` | `initial_capacity` so large that `malloc(sizeof(TYPE) * capacity)` fails (`if (!arr->data) { free(arr); return NULL; }`) — e.g. `SIZE_MAX`, `SIZE_MAX/2`, `1<<62` | returns `NULL`, header freed | `[x]` `errors::create_huge_capacity_returns_null` |
| 3 | `array_TYPE_push` | `arr == NULL` (`if (!arr) return -1`) | returns `-1`, nothing written | `[x]` `errors::push_null_array_returns_minus_one` |
| 4 | `array_TYPE_push` | full array whose grow step fails: `size >= capacity` and `realloc(data, sizeof(TYPE) * capacity*2)` returns `NULL` (`if (!new_data) return -1`) | returns `-1`; `data`, `size`, `capacity` unchanged | `[x]` `errors::push_realloc_failure_returns_minus_one` |
| 5 | `array_TYPE_get` | `index >= size` — no bounds check at all (`return arr->data[index]`) | reads the raw slot: deterministic for `size <= index < capacity` (e.g. after `clear`), UB past the allocation | `[x]` `errors::get_index_past_size_reads_slot` (deterministic part) |
| 6 | `array_TYPE_get` | `arr == NULL` — no NULL check | dereferences NULL → `SIGSEGV` | `[x]` `errors::ub_null_deref_matches` (forked, compares fatal signal) |
| 7 | `array_TYPE_size` | `arr == NULL` (`return arr ? arr->size : 0`) | returns `0` | `[x]` `errors::size_null_returns_zero` |
| 8 | `array_TYPE_clear` | `arr == NULL` (`if (arr) arr->size = 0`) | no-op, no crash | `[x]` `errors::clear_destroy_null_are_noops` |
| 9 | `array_TYPE_destroy` | `arr == NULL` (`if (arr) { … }`) | no-op, no crash | `[x]` `errors::clear_destroy_null_are_noops` |
| 10 | `list_TYPE_create` | `malloc(sizeof(list_TYPE_t))` fails (`if (!list) return NULL`) | returns `NULL` | `[x]` `errors::malloc_failure_paths` |
| 11 | `list_TYPE_append` | `list == NULL` (`if (!list) return -1`) | returns `-1` | `[x]` `errors::list_null_append_prepend_return_minus_one` |
| 12 | `list_TYPE_append` | node `malloc` fails (`if (!node) return -1`) | returns `-1`, list unchanged | `[x]` `errors::malloc_failure_paths` |
| 13 | `list_TYPE_prepend` | `list == NULL` (`if (!list) return -1`) | returns `-1` | `[x]` `errors::list_null_append_prepend_return_minus_one` |
| 14 | `list_TYPE_prepend` | node `malloc` fails (`if (!node) return -1`) | returns `-1`, list unchanged | `[x]` `errors::malloc_failure_paths` |
| 15 | `list_TYPE_size` | `list == NULL` (`return list ? list->size : 0`) | returns `0` | `[x]` `errors::size_null_returns_zero` |
| 16 | `list_TYPE_clear` / `list_TYPE_destroy` | `list == NULL` (`if (!list) return`) | no-op, no crash | `[x]` `errors::clear_destroy_null_are_noops` |
| 17 | `calculate_inventory_stats` | `items == NULL` (`if (!items \|\| items->size == 0)`) | prints exactly `No items in inventory\n`, returns | `[x]` `errors::inventory_stats_null_and_empty` |
| 18 | `calculate_inventory_stats` | non-NULL array with `size == 0` (same guard, second clause) | prints exactly `No items in inventory\n`, returns | `[x]` `errors::inventory_stats_null_and_empty` |
| 19 | `calculate_order_stats` | `orders == NULL` (`if (!orders \|\| orders->size == 0)`) | prints exactly `No orders to analyze\n`, returns | `[x]` `errors::order_stats_null_and_empty` |
| 20 | `calculate_order_stats` | non-NULL list with `size == 0` (same guard, second clause) | prints exactly `No orders to analyze\n`, returns | `[x]` `errors::order_stats_null_and_empty` |
| 21 | `find_items_by_category` | `items == NULL` (`if (!items \|\| !category) return`) | returns immediately, **no output at all** (not even the header) | `[x]` `errors::find_by_category_null_args_silent` |
| 22 | `find_items_by_category` | `category == NULL` (same guard, second clause) | returns immediately, no output | `[x]` `errors::find_by_category_null_args_silent` |
| 23 | `find_items_by_category` | valid array, no `strcmp` match (`if (found == 0)`) | header line, then `No items found in this category\n` | `[x]` `errors::find_by_category_no_match_message` |
| 24 | `find_expensive_items` | `items == NULL` (`if (!items) return`) | returns immediately, no output | `[x]` `errors::find_expensive_null_silent` |
| 25 | `find_expensive_items` | valid list, no item with `price >= min_price` (incl. `min_price = NaN`, where every comparison is false) | header line, then `No items found above this price\n` | `[x]` `errors::find_expensive_no_match_message` |
| 26 | `create_item` | `name == NULL` or `category == NULL` → `strncpy` from NULL | `SIGSEGV` (no check in C) | `[x]` `errors::ub_null_deref_matches` (forked, compares fatal signal) |
| 27 | `create_order` | `customer_name == NULL` → `strncpy` from NULL | `SIGSEGV` (no check in C) | `[x]` `errors::ub_null_deref_matches` (forked, compares fatal signal) |
| 28 | `create_item` / `create_order` | `name`/`category` longer than the field (`MAX_NAME_LENGTH-1` = 63, `MAX_CATEGORY_LENGTH-1` = 31) | silently truncated, byte `N-1` forced to `'\0'`; no error signalled | `[x]` `inventory_funcs::create_item_random_strings` |
| 29 | `calculate_inventory_stats` | `total_items` sums to `0` → `total_value / total_items` divides by zero | prints `$-nan` (for `0.0/0`), `$inf`/`$-inf` when `total_value != 0`; no error | `[x]` `inventory_funcs::stats_division_edge_cases` |
| 30 | `calculate_inventory_stats` | `int total_items` overflows (`total_items += item.quantity`) | wraps (two's complement at `-O0`); printed with `%d` | `[x]` `inventory_funcs::stats_quantity_overflow` |
| 31 | `print_item` / `print_order` / `find_items_by_category` | fixed-size buffer with no NUL in it | `printf("%s")` / `strcmp` keep reading past the field, into the bytes that follow | `[x]` `errors::unterminated_fields_read_into_the_next_field` (within-struct part; see note B) |
| 32 | `main` | `fgets` returns `NULL`: empty stdin / EOF / stdin closed | loop breaks, process exits `0` | `[x]` `driver_e2e::stdin_scenarios` (`""`, EOF after menu) |
| 33 | `main` | `sscanf(input, "%d", &choice) != 1`: no digits — `"abc"`, `"\n"`, `" "`, `"+"`, `"-"`, `"."`, `"x1"` | prints `Invalid input\n`, re-shows menu | `[x]` `driver_e2e::stdin_scenarios` |
| 34 | `main` | `switch` `default`: parsed value outside `1..=7` — `0`, `8`, `-1`, `2147483647`, `-2147483648` | prints `Invalid choice\n`, re-shows menu | `[x]` `driver_e2e::stdin_scenarios` |
| 35 | `main` | value one step past the valid menu range on both sides (`0`, `8`) | `Invalid choice\n` (menu is the only "enum" this API takes across its boundary) | `[x]` `driver_e2e::stdin_scenarios` |
| 36 | `main` | integer that overflows `long` in `sscanf` — `"99999999999999999999"`, `"-99999999999999999999"`, `"4294967296"`, `"2147483648"` | glibc saturates to `LONG_MAX`/`LONG_MIN` then truncates to `int` (`-1`, `0`, `0`, `-2147483648`) → `Invalid choice` | `[x]` `driver_e2e::stdin_scenarios` |
| 37 | `main` | input line longer than `sizeof(input)-1` = 255 bytes | `fgets` keeps the first 255 bytes; the tail is consumed by the following iteration(s) | `[x]` `driver_e2e::stdin_scenarios` (300-byte and 600-byte lines) |
| 38 | `main` | leading whitespace / trailing garbage: `"   3\n"`, `"3junk\n"`, `"3 4\n"`, `"\t7\n"`, `"007\n"`, `"+3\n"` | `%d` skips leading whitespace, stops at the first non-digit → still `1` conversion | `[x]` `driver_e2e::stdin_scenarios` |
| 39 | `main` | embedded NUL in the line (`"3\0009\n"`) | `sscanf` stops at the NUL → choice `3` | `[x]` `driver_e2e::stdin_scenarios` |

## Notes

**A. Small-allocation failures.** Rows 1, 10, 12 and 14 fire only when `malloc`
returns `NULL` for a 24/88/128-byte request. `errors::malloc_failure_paths`
provokes exactly that: a forked child caps its address space with
`RLIMIT_AS`, allocates until every size class fails, and only then calls
`array_*_create` / `list_*_create` / `list_*_append` / `list_*_prepend`,
reporting the outcome through its exit code. The test asserts that the C build
really did return `NULL`/`-1` (so the row cannot pass vacuously) and that the
Rust build reported the identical verdict. Row 2 exercises the same
"`malloc` said no" logic through the one request whose size the caller controls,
and row 4 does it for `realloc`.

**B. Unterminated fields.** Row 31: `item_t.name` / `item_t.category` /
`order_t.customer_name` are always NUL-terminated when the struct comes from
`create_item`/`create_order` (C forces `buf[MAX-1] = '\0'`). A hand-built struct
with no NUL makes C's `printf("%s")` and `strcmp` walk into the following bytes.
The Rust translation reproduces that: `%s` is emitted with an unbounded read from
the field's address (`cio::c_str_ptr`), and `strcmp` is a raw byte loop, so the
walk continues into `category`/the padding/`price` exactly as in C. The test
builds `item_t`/`order_t` from all 120 / 80 raw bytes (padding included) and
places the terminating NUL in a *later field*, which makes the walk deterministic
and lets it be compared byte for byte. Where the walk would leave the struct
entirely, the bytes are the callee's stack and no implementation-independent
result exists, so that sub-case is deliberately not asserted.

**C. Forked UB comparison.** Rows 6, 26 and 27 dereference a NULL pointer in
both implementations. `errors::ub_null_deref_matches` runs each call in a
`fork()`ed child and asserts that the C child and the Rust child die from the
*same* fatal signal, which is as close to "same rejection" as an unchecked
dereference can get.

**D. NaN payload propagation (found by the randomized rows).** `total_value +=
price * quantity` and `total_revenue += amount` can add two NaNs. x86-64's
`ADDSD` keeps the destination operand's NaN, and the unoptimized C build makes
the *added value* the destination, so the last NaN in the sum is printed. LLVM
treats `fadd` as commutative and swaps the operands when optimizing, so a
release build of a naive `+=` translation printed `$-nan` where C prints `$nan`.
`src/cio.rs::fadd_c` now resolves the NaN cases explicitly, and
`inventory_funcs::{inventory_stats_nan_inf_exhaustive,
order_stats_nan_inf_exhaustive, stats_nan_positions_in_long_sums}` cover every
ordered combination of `{0.0, -0.0, ±inf, ±NaN, ±1.5}` in the accumulators, in
both the debug and release profiles.
