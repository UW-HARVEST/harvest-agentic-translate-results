# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return NULL`, `return -1`,
null check, and implicit-failure branch in the C source is one row.
There are no `assert`s, no error enums, and no explicit range checks in the C.

Grep evidence:
```
$ grep -n 'return NULL\|return -1\|if (!\|if (mb)\|> NULL\|assert' c_src/src/lib.c
50:    if (!mb) return NULL;
53:    if (!mb->data) {
55:        return NULL;
68:    if (mb) {
69:        if (mb->data) free(mb->data);
130:    if (!mem1 || !mem2) {
133:        return -1;
156:    if (mem1->data > NULL && mem2->data > NULL) {
```

| # | function | trigger (exact invalid input / condition) | expected C result | test | [x] |
|---|----------|-------------------------------------------|-------------------|------|-----|
| E1 | `allocate_block` | `malloc(sizeof(MemoryBlock))` fails (line 50 `if (!mb) return NULL;`) | returns `NULL` | `err_e1_documented_via_e2_e3` — a 16-byte malloc cannot be forced to fail from outside; the same `NULL` exit is driven by E2/E3 | [x] |
| E2 | `allocate_block` | `calloc(count, 4)` fails because `count * 4` overflows `size_t` — e.g. `count = SIZE_MAX`, `SIZE_MAX/2`, `SIZE_MAX/4+1` (line 53) | frees `mb`, returns `NULL` | `err_e2_allocate_block_calloc_overflow` | [x] |
| E3 | `allocate_block` | `calloc(count, 4)` fails because the request is huge but non-overflowing — e.g. `count = SIZE_MAX/4`, `1<<60`, `1<<48` | frees `mb`, returns `NULL` | `err_e3_allocate_block_calloc_too_large` | [x] |
| E4 | `allocate_block` | `count == 0` (**not** an error): `calloc(0,4)` returns a unique non-NULL pointer, loop body never runs | non-NULL `MemoryBlock*` with `size == 0`, `data != NULL` | `err_e4_allocate_block_zero_count` | [x] |
| E5 | `free_block` | `mb == NULL` (line 68 `if (mb)`) | no-op, no crash | `err_e5_free_block_null` | [x] |
| E6 | `free_block` | `mb != NULL` but `mb->data == NULL` (line 69 `if (mb->data)`) | frees only `mb`, no crash | `err_e6_free_block_null_data` | [x] |
| E7 | `free_block` | `mb` valid with `size == 0` and non-NULL `data` (from E4) | frees both, no crash | `err_e7_free_block_zero_size` | [x] |
| E8 | `betagamma` | `param1 % 10 ∈ {-9..-6}` ⇒ `block_size = (param1%10)+5 < 0` ⇒ sign-extended to a huge `size_t` ⇒ both `allocate_block` calls fail (line 130) | `return -1` | `err_e8_betagamma_negative_block_size` | [x] |
| E9 | `betagamma` | `param1 % 10 == -5` ⇒ `block_size == 0` — the **exact near-miss, one step inside** the valid range. `calloc(0,4)` succeeds, so this must **not** return -1 | normal (non-`-1`) result | `err_e9_betagamma_block_size_zero_is_valid` | [x] |
| E9b | `betagamma` | `param1 % 10 ∈ {-4..-1}` ⇒ `block_size` ∈ {1..4} (small but valid) | normal (non-`-1`) result | `err_e9_betagamma_block_size_zero_is_valid` | [x] |
| E10 | `betagamma` | `param1 = INT_MIN` (extreme boundary: `INT_MIN % 10 == -8` ⇒ `block_size == -3` ⇒ error path) | `return -1` | `err_e10_betagamma_int_min` | [x] |
| E11 | `betagamma` | `param1 = INT_MAX` (`INT_MAX % 10 == 7` ⇒ `block_size == 12`, valid) plus extreme `param2..4` causing signed overflow in `result`/`sum` | same wrapped value in both | `err_e11_betagamma_int_extremes` | [x] |
| E12 | `compute_hash` | `mb1 == mb2` (same pointer): both pointer comparisons are equal ⇒ neither branch taken | returns `0` | `err_e12_compute_hash_identical` | [x] |
| E13 | `compute_hash` | `mb1`/`mb2` with `data == NULL` in one or both operands (NULL pointer compared, no deref of `data`) | `100`/`200`/`0` + `10`/`20` per pointer ordering | `err_e13_compute_hash_null_data` | [x] |
| E14 | `compute_hash` | `mb1 > mb2` in address order (reversed argument order) | `200 + 20` for the reversed pair | `err_e14_compute_hash_reversed` | [x] |
| E15 | `create_block` | `name` longer than 31 chars ⇒ `strcpy` overflows `block.name` (C has no length check at all) | both implementations write past `name` identically; only the first 32 bytes are defined | `err_e15_create_block_exact_31_and_boundary` (checks 0/1/30/31-char names, i.e. one step past the last safe length) | [x] |
| E16 | `create_block` | empty `name` (`""`) — zero-length boundary | `name[0] == 0`, rest of `name` uninitialised | `err_e16_create_block_empty_name` | [x] |
| E17 | `create_block` | `flags` = full `uint8_t` range incl. values with no meaning to the code (0, 0xFF, and every "out of range enum"-style int truncated to `uint8_t`) | `flags` stored verbatim | `err_e17_create_block_flag_range` | [x] |

### Generic FFI-boundary sweep (required even though not a table row)

| # | coverage | test | [x] |
|---|----------|------|-----|
| G1 | `free_block(NULL)` (null pointer across the boundary) | `err_e5_free_block_null` | [x] |
| G2 | zero length (`count == 0`) and oversized length (`SIZE_MAX`, `SIZE_MAX/2`, `1<<63`, `SIZE_MAX/4 ± 1`, `1<<61`, `1<<62`) | `err_generic_boundary_sweep` | [x] |
| G3 | one step past each documented range: `SIZE_MAX/4` vs `SIZE_MAX/4 + 1` (the calloc overflow edge), `param1 % 10 == -5` vs `-6` (the `block_size` sign edge), `name` len 30 vs 31 | `err_generic_boundary_sweep`, `err_e9_betagamma_block_size_zero_is_valid`, `err_e15_create_block_exact_31_and_boundary` | [x] |
| G4 | out-of-range values in the narrow (`uint8_t`) parameter — the enum analogue: all 256 values plus `256`, `257`, `-1`, `-128`, `1000`, `INT_MIN`, `INT_MAX` truncated on the way in | `err_e17_create_block_flag_range` | [x] |
| G5 | every `param1` residue from −25..25 crossed with `INT_MAX`/`INT_MIN` in the other params, compared out of process | `err_generic_boundary_sweep` | [x] |

## Notes on rows that are *not* separately testable

* **E1** — `malloc(16)` cannot be made to fail from outside the library without
  interposing the allocator; both implementations take the *identical* early
  `return NULL` path, and that path is exercised byte-for-byte by E2/E3
  (`calloc` failure ⇒ `free(mb)` ⇒ `NULL`).
* There are **no out-of-range enum parameters** in this API (no `enum` type is
  declared anywhere in `lib.h`/`lib.c`); the closest analogue is `uint8_t flags`,
  whose *entire* 0..255 domain is covered exhaustively by E17, and `int` params,
  covered at `INT_MIN`/`INT_MAX`/0/±1 by E8–E11.
* **NULL `MemoryBlock*` into `compute_hash`** is *not* a row: the C
  unconditionally does `mb1->data`, so passing `NULL` is UB that segfaults both
  libraries identically. It is therefore not a defined "rejection" and is not
  tested (a crash-vs-crash comparison would abort the test harness).
