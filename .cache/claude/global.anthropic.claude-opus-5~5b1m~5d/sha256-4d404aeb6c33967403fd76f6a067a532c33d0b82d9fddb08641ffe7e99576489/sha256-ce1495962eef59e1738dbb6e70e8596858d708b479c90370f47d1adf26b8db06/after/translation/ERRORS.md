# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c`. Every `return NULL`, `return -1`,
every null guard, every implicit-deref (unchecked pointer) site, and every
min/max constant the code depends on gets its own row. There are **no**
`assert`s, no `RETURN_ERROR`-style macros, and no error enums in this library —
the entire rejection surface is `NULL` / `-1` sentinels plus unchecked
dereferences.

Grep basis:

```
50:    if (!mb) return NULL;                     -> row 1
53:    if (!mb->data) { free(mb); return NULL; } -> row 2
68:    if (mb) {                                 -> rows 5, 6
69:        if (mb->data) free(mb->data);         -> row 6
77:    if (mb1->data < mb2->data)                -> rows 7, 8  (unchecked deref)
83:    if (mb1 < mb2)                            -> rows 7, 8
130:   if (!mem1 || !mem2) { ...; return -1; }   -> rows 3, 4
43:    strcpy(block.name, name);                 -> rows 9, 10
```

Legend for "expected C result": `NULL` = null pointer return, `-1` = the int
sentinel, `SIGSEGV` = process death by signal 11 (the C performs an unchecked
dereference; the Rust must die the same way rather than, say, panicking with a
Rust message or returning a value).

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `allocate_block` | `malloc(sizeof(MemoryBlock))` (16 B) fails → `!mb` (`lib.c:50`) | returns `NULL`; nothing leaked |
| 2  | `allocate_block` | `calloc(count, 4)` fails → `!mb->data` (`lib.c:53`). Reachable for any `count > SIZE_MAX/4`, where glibc's `nmemb*size` overflow check trips: `count = SIZE_MAX`, `SIZE_MAX-1`, `SIZE_MAX/4+1 = 0x4000000000000000`, `0x8000000000000000` | frees `mb`, returns `NULL` |
| 3  | `betagamma` | `block_size = (param1 % 10) + 5` is **negative** and sign-extends to a huge `size_t`, so `allocate_block` → `NULL` for `mem1`. Happens iff `param1 % 10 ∈ {-6,-7,-8,-9}`, i.e. `param1 ∈ {-6..-9, -16..-19, -26..-29, …}` incl. `INT_MIN` (`-2147483648 % 10 == -8`) (`lib.c:126,130,133`) | returns `-1` (both `free_block(NULL)` calls are no-ops) |
| 4  | `betagamma` | same as row 3 for `mem2` — note the C calls `allocate_block` **twice unconditionally** before testing `!mem1 \|\| \|mem2`, so both are `NULL` together (`block_size` is shared) | returns `-1` |
| 5  | `free_block` | `mb == NULL` — outer `if (mb)` guard (`lib.c:68`) | no-op, no crash, `void` |
| 6  | `free_block` | `mb != NULL` but `mb->data == NULL` — inner `if (mb->data)` guard (`lib.c:69`) | frees only `mb`, no `free(NULL)`… no crash |
| 7  | `compute_hash` | `mb1 == NULL` — `mb1->data` is dereferenced with **no** null check (`lib.c:77`) | `SIGSEGV` |
| 8  | `compute_hash` | `mb2 == NULL` — `mb2->data` dereferenced unchecked (`lib.c:77`) | `SIGSEGV` |
| 9  | `create_block` | `name == NULL` — passed straight to `strcpy` (`lib.c:43`) | `SIGSEGV` |
| 10 | `create_block` | `strlen(name) > 31` — `strcpy` overflows the 32-byte `block.name` array. Pure UB (stack smash); *not* a checked rejection. Row exists to record that C performs **no** length check, so Rust must not add one | no diagnostic; C writes past `name` (behaviour compared only for the in-bounds prefix; see note below) |

## Generic FFI boundary cases also covered (not explicit C checks)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| G1 | `allocate_block` | `count == 0` → `calloc(0, 4)` returns a **unique non-NULL** pointer in glibc, loop body never runs | non-`NULL` block with `size == 0` |
| G2 | `allocate_block` | `count == 1` (minimum non-empty) | `size == 1`, `data[0] == init_value` |
| G3 | `allocate_block` | `init_value == INT_MAX` / `INT_MIN` — `init_value + i` is computed in `size_t` then truncated to `int`, so it wraps | wrapped values, no trap |
| G4 | `compute_hash` | `mb1 == mb2` (aliased, non-NULL) — neither `<` nor `>` branch taken twice | `0` |
| G5 | `compute_hash` | `data` fields with the **high bit set** (e.g. `0xFFFF_FFFF_FFFF_FFFF` vs `0x1`). C pointer relational comparison is **unsigned**; a signed comparison in Rust would invert the result | `200` (+ pointer term) |
| G6 | `betagamma` | `param1 = INT_MIN`, `INT_MAX`; `param2..4 = INT_MIN`, `INT_MAX` — signed overflow in `flag_contribution`, `result`, `sum1`, `sum2` (UB in C, wrapping in practice at `-O0`) | wrapping arithmetic, no trap |
| G7 | `betagamma` | `param1 % 10 == -5` → `block_size == 0`, i.e. the row-G1 `calloc(0)` path reached *through* `betagamma`; must **not** take the `-1` branch | a normal (non-`-1`) result |
| G8 | `betagamma` | `(sum1 - sum2)` negative → `/ 10` must truncate **toward zero**, not floor | C truncation semantics |
| G9 | all | out-of-range "enum" values across the FFI boundary: this library declares **no enums**; the nearest analogue is `uint8_t flags`, for which *every* one of the 256 values is meaningful. All 256 are swept in `create_block`, and all 256 are swept through `compute_hash`-independent `betagamma` masks | value-dependent, never rejected |

Row 10 note: `strcpy` past the end of `block.name` is undefined behaviour in both
languages. The test restricts itself to source lengths 32..35, for which the
copy (35 chars + NUL, starting at offset 4) still lands entirely inside the
40-byte `DataBlock`, so the whole result stays observable. It asserts that
neither implementation rejects the input and that `id`, all 32 bytes of `name`,
and `flags` match. The 3 tail padding bytes (offsets 37..39) are indeterminate
and are **not** compared, and lengths > 35 — which would smash the caller's
frame — are deliberately not exercised.

Uninitialised-padding note (applies to `create_block` generally): C leaves
`DataBlock block;` uninitialised and `strcpy` writes only up to the NUL, so
`name[strlen+1 .. 31]` and the 3 tail padding bytes are indeterminate in the C.
Tests compare `id`, `flags`, and `name[0 ..= strlen]` only.

## Row → test mapping (all rows verified)

Every row has a differential test that constructs the exact condition, calls
BOTH `.so`s, and asserts the SAME sentinel (`NULL` / `-1`) or the SAME fatal
signal. Run with `cargo test --test errors`.

| # | test in `tests/errors.rs` | status |
|---|---------------------------|--------|
| 1 | `row01_allocate_block_malloc_failure_returns_null` — caps `RLIMIT_AS` in a forked child, drains the arena through the library's own `allocate_block`, then asserts both return `NULL` | PASS |
| 2 | `row02_allocate_block_calloc_overflow_returns_null` — 8 overflowing counts × 4 `init_value`s | PASS |
| 3 | `row03_row04_betagamma_negative_block_size_returns_minus_one` — 26 `param1` values incl. `INT_MIN`, each asserted to actually have residue −6..−9 | PASS |
| 4 | same test + `row03_row04_boundary_one_step_either_side` (10 values, one step either side of the −5/−6 cliff) | PASS |
| 5 | `row05_free_block_null_is_noop` — 100 consecutive `free_block(NULL)` calls in a forked child | PASS |
| 6 | `row06_free_block_null_data_field` — heap block with `data = NULL`, three `size` values | PASS |
| 7 | `row07_compute_hash_null_mb1_segfaults` — both sides `SIGSEGV` (signal 11) | PASS |
| 8 | `row08_compute_hash_null_mb2_segfaults`, `row07_row08_compute_hash_both_null_segfaults` | PASS |
| 9 | `row09_create_block_null_name_segfaults` — both sides `SIGSEGV` | PASS |
| 10 | `row10_create_block_no_length_check_overflow_within_struct` — lengths 32..35 (overflow contained inside the 40-byte struct), 800 randomized cases | PASS |

Row 10 finding: the C assigns `block.flags = flags;` **after** the `strcpy`, so
the assignment overwrites the byte the overflow put at offset 36. `flags` therefore
always equals the argument, and only the 3 indeterminate tail padding bytes keep
smashed values. The Rust reproduces this ordering; a mutant that wrote `flags`
before the `strcpy` is caught by this test.

| G-row | test | status |
|-------|------|--------|
| G1, G2 | `g01_g02_allocate_block_zero_and_one` | PASS |
| G3 | `g03_allocate_block_init_value_wraps` | PASS |
| G4 | `g04_compute_hash_aliased_is_zero` | PASS |
| G5 | `g05_compute_hash_pointer_comparison_is_unsigned` | PASS |
| G6 | `g06_betagamma_extreme_params_do_not_trap` | PASS |
| G7 | `g07_betagamma_block_size_zero_is_not_an_error` | PASS |
| G8 | `g08_betagamma_division_truncates_toward_zero` | PASS |
| G9 | `g09_create_block_out_of_range_flags_across_ffi` (12 out-of-range `int`s in a `uint8_t` slot), `g09_allocate_block_negative_count_across_ffi` (5 negative counts), `g09_free_block_dangling_low_pointers_reject_identically` | PASS |
| extra | `row03_row04_int_to_size_t_conversion_boundary` — pins the sign-extension of `int -> size_t` (see the caveat in that test's doc comment) | PASS |
