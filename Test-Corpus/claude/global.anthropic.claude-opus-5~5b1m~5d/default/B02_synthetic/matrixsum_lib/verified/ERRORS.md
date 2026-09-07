# ERRORS.md — Error-surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/lib.c`: every `return NULL`, every
`return 0` that is a rejection, every `return -1`, every null check, every
implicit "no-op on null", and every allocation-failure branch. There are no
`assert`s, no error enums, and no explicit range/min/max constants in the C
source — the entire rejection surface is null-pointer checks plus allocator
failure.

Line numbers refer to `c_src/src/lib.c`.

Every row has a passing differential test in `tests/phase_c_errors.rs`
(`rowNN_*`); the `[x]` column is ticked only after that test passed against both
`.so`s in both the debug and release profiles.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `init_array` (L47)  | `malloc(sizeof(DynamicArray))` returns NULL — 24-byte allocation fails | returns `NULL`; nothing allocated | [x] |
| 2  | `init_array` (L50-53) | `malloc(initial_capacity * sizeof(int))` returns NULL. Reachable with a huge `initial_capacity` (e.g. `1<<50` ⇒ 4 PiB request) | `free(arr)` then return `NULL` | [x] |
| 3  | `init_array` (L49)  | `initial_capacity` so large that `initial_capacity * sizeof(int)` **wraps** the `size_t` multiply (e.g. `SIZE_MAX/4 + 1` ⇒ 0 bytes; `(SIZE_MAX/4)+2` ⇒ 4 bytes). Unsigned wrap, not a trap. | small/zero malloc **succeeds** ⇒ returns non-NULL array whose `capacity` is the huge unwrapped value | [x] |
| 4  | `init_array` (L49)  | `initial_capacity == 0` ⇒ `malloc(0)` | glibc returns a unique non-NULL pointer ⇒ non-NULL array, `size==0`, `capacity==0` | [x] |
| 5  | `expand_array` (L61) | `arr == NULL` | returns `0`, no dereference | [x] |
| 6  | `expand_array` (L64-68) | `realloc` returns NULL, i.e. `arr->capacity*2*sizeof(int)` is unsatisfiable (e.g. `capacity == 1<<50`) | returns `0`; `arr->data` and `arr->capacity` left **unmodified** (old buffer not freed) | [x] |
| 7  | `expand_array` (L63) | `arr->capacity == 0` ⇒ `new_capacity == 0` ⇒ `realloc(data, 0)` | glibc frees `data` and returns `NULL` ⇒ `new_data` NULL ⇒ returns `0`, `arr->data` left dangling, `capacity` still 0 | [x] |
| 8  | `expand_array` (L63) | `arr->capacity` huge so `capacity*2` wraps (e.g. `capacity == SIZE_MAX/2 + 1` ⇒ `new_capacity == 0`) or `new_capacity*4` wraps | wrapped (small) `realloc` size; result follows the wrapped request, `capacity` set to wrapped value on success | [x] |
| 9  | `add_element` (L76) | `arr == NULL` | returns `0`, no write | [x] |
| 10 | `add_element` (L78-82) | `arr->size >= arr->capacity` **and** `expand_array` fails (rows 6/7/8) | returns `0`; `size` **not** incremented, no element stored | [x] |
| 11 | `add_element` (L78) | `arr->size >= arr->capacity` with `capacity == 0` (fresh `init_array(0)`) | `expand_array` returns 0 ⇒ `add_element` returns `0` | [x] |
| 12 | `free_array` (L89) | `arr == NULL` | no-op, no crash, no return value | [x] |
| 13 | `matrixsum` (L153-156) | `init_array(2)` returns NULL (allocator exhausted) | returns `-1` — the only sentinel in the public header's API | [x] |
| 14 | `process_flags` (L95) | *No rejection path.* Accepts **any** `int`, including negatives, `INT_MIN`, `INT_MAX`, and values with no valid flag bits set / bits outside the 4 defined flags. Only bits 0-3 are examined. | always returns `0..=4`; never errors | [x] |
| 15 | `matrixsum` (L128) | *No input validation.* Any `int` quadruple accepted, including `INT_MIN`/`INT_MAX` where `sum` and `sum * 0x10` overflow. | wraps (two's complement); returns the wrapped `int`, never an error | [x] |

## Generic FFI boundary cases also covered in Phase C

* NULL passed to every pointer-taking export: `expand_array`, `add_element`,
  `free_array` (rows 5, 9, 12).
* Zero length / capacity: `init_array(0)`, `expand_array` on `capacity==0`
  (rows 4, 7, 11).
* Oversized length: `init_array(1<<50)`, `expand_array` on `capacity==1<<50`
  (rows 2, 6).
* One step past a wrap boundary: `SIZE_MAX/4`, `SIZE_MAX/4+1`, `SIZE_MAX/2+1`,
  `SIZE_MAX` for `init_array`; same for `expand_array`'s `capacity` (rows 3, 8).
* Out-of-range "enum" values: the C library declares no `enum`. The nearest
  equivalent is the `FLAG_*` bitmask passed to `process_flags`, which is typed
  `int` and therefore accepts every 32-bit value. Phase C sweeps all 16 valid
  flag combinations **plus** values with no valid variant (bit 4 and above set,
  negative values, `INT_MIN`, `INT_MAX`, `-1`) to confirm identical handling.
* `free_array` on a pointer whose `data` is NULL (`free(NULL)` is a no-op) —
  reachable state after row 7.
