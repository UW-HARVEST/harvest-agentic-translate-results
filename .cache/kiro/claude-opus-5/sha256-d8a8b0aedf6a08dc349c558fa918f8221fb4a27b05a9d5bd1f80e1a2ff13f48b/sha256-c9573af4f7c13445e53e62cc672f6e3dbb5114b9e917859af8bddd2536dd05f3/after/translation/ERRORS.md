# ERRORS.md — Phase A error-surface table

Derived mechanically by grepping `c_src/src/lib.c` for every `return`, every
null check, every guard, and every implicit conversion that can reject input.
There are no `assert`s, no error enums, no `RETURN_ERROR`-style macros and no
explicit range checks in this library — its entire rejection surface is
`NULL`-returns from `allocate_block`, the `-1` return from `betagamma`, and the
two guards in `free_block`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `allocate_block` | `malloc(sizeof(MemoryBlock))` fails (line 50, `if (!mb) return NULL;`). Not reachable by input alone on a 64-bit host; covered indirectly — no `count` value can make the fixed 16-byte `malloc` fail. | `NULL` | `err_e1_allocate_block_malloc_failure_guard_never_fires` (asserts the guard never fires, for both impls, across all reachable `count`) | [x] |
| E2 | `allocate_block` | `calloc(count, sizeof(int))` fails (lines 53–56) because `count * 4` overflows / is unsatisfiable: `count = SIZE_MAX`, `SIZE_MAX/4`, `SIZE_MAX/2`, `SIZE_MAX-1`, `0x4000_0000_0000_0000`, `1<<62`. Note `mb` must be `free`d first — no leak, and the return is `NULL` not a half-built block. | `NULL` | `err_e2_allocate_block_calloc_overflow_returns_null` | [x] |
| E3 | `allocate_block` | `count = 0` — **not** an error: `calloc(0, 4)` returns a unique non-NULL pointer under glibc, so the function succeeds with `size == 0` and an empty (but non-NULL, dereference-free) `data`. Included because it is the boundary one step below the valid range. | non-`NULL`, `size == 0`, `data != NULL` | `err_e3_allocate_block_zero_count_is_not_an_error` | [x] |
| E4 | `free_block` | `mb == NULL` (line 68, `if (mb)`) | silent no-op, no crash | `err_e4_free_block_null_is_noop` | [x] |
| E5 | `free_block` | `mb != NULL` but `mb->data == NULL` (line 69, `if (mb->data)`) — inner `free` skipped, outer `free(mb)` still runs | frees only `mb`, no crash | `err_e5_free_block_null_data_skips_inner_free` | [x] |
| E6 | `betagamma` | `mem1 == NULL || mem2 == NULL` (lines 130–134). Reached whenever `(param1 % 10) + 5 < 0`, i.e. `param1 % 10 ∈ {-6,-7,-8,-9}` ⇔ `param1 < 0 && |param1| % 10 ∈ {6,7,8,9}`. The negative `int` converts to a huge `size_t`, `calloc` fails, both blocks are freed, `-1` is returned **before** any hash/sum work. | `-1` | `err_e6_betagamma_negative_block_size_returns_minus_one` | [x] |
| E7 | `betagamma` | `param1 = INT_MIN` — `INT_MIN % 10 == -8` ⇒ `block_size = -3` ⇒ same `-1` path. Explicit extreme-boundary row for E6. | `-1` | `err_e7_betagamma_int_min_param1` | [x] |
| E8 | `betagamma` | `param1 % 10 == -5` (`param1 = -5, -15, -25, …`) ⇒ `block_size == 0`. One step *inside* the valid range: `calloc(0,4)` succeeds, both sum loops execute zero iterations, and the `mem1->data > NULL` guard (line 156) is still true for the unique zero-size allocations. | a real result, **not** `-1` | `err_e8_betagamma_zero_block_size_is_not_an_error` | [x] |
| E9 | `compute_hash` | `mb1 == mb2` (aliasing) — no null check exists, so this is not rejected; both `data` and pointer comparisons take the equal branch and neither `+=` fires. | `0` | `err_e9_compute_hash_aliased_pointers` | [x] |
| E10 | `compute_hash` | `NULL` argument — the C dereferences `mb1->data` / `mb2->data` with **no** null check (line 79). This is a hard crash in C, not a rejection; the Rust must not "improve" it into a check. Compared by actual termination signal in a forked child, not asserted structurally. | undefined behaviour / SIGSEGV — no error code | `err_e10_compute_hash_has_no_null_guard`: runs each impl in its own forked child and asserts they die with the SAME signal | [x] |
| E11 | `create_block` | `name == NULL` — `strcpy(block.name, NULL)` has no guard (line 42). Hard crash in C, not a rejection. Compared by actual termination signal in a forked child. | undefined behaviour / SIGSEGV | `err_e11_create_block_has_no_null_guard`: same forked-child signal comparison | [x] |
| E12 | `create_block` | `strlen(name) > 31` — unbounded `strcpy` into `char name[32]`; C overflows the struct. Not rejected. Tested only up to the largest **non**-overflowing length (31 chars + NUL) as the boundary; longer inputs are UB in both and are not exercised. | no error; 31-char name copied verbatim with NUL at index 31 | `err_e12_create_block_max_length_name_boundary` | [x] |
| E13 | `allocate_block` | `init_value + i` overflows `int` for `init_value` near `INT_MAX` (line 61). C computes in `size_t` then truncates on assignment — wraps, never errors. | wrapped values, no error | `err_e13_allocate_block_init_value_overflow_wraps` | [x] |

## Generic FFI boundary cases (covered even though absent from the table)

* **Null pointers** — `free_block(NULL)` (E4), `MemoryBlock{data: NULL}` (E5),
  and the *absence* of null guards in `compute_hash` / `create_block`
  (E10/E11).
* **Zero lengths** — `allocate_block(0, …)` (E3) and `betagamma` with
  `block_size == 0` (E8).
* **Oversized lengths** — `allocate_block(SIZE_MAX, …)` and the
  `count * 4` overflow family (E2).
* **One step past a valid range** — `block_size` sweeping `-4 … 14` across the
  `-1` / non-`-1` frontier (E6/E8), and `INT_MIN` / `INT_MAX` for every `int`
  parameter (E7, E13).
* **Out-of-range enum values** — this library declares **no** `enum` type
  (`grep -c enum c_src/src/lib.c` = 0), so there is no invalid-variant class to
  exercise. The equivalent "any bit pattern is a legal input" axis here is the
  `uint8_t flags` parameter of `create_block`, which is swept over **all 256
  values**, and the four unconstrained `int` parameters of `betagamma`, which
  are swept over the full `i32` range randomly plus both extremes.
