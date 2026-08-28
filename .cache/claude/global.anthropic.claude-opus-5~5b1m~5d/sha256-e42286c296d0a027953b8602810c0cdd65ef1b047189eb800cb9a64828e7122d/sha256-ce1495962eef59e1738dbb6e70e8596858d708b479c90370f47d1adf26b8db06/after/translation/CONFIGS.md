# CONFIGS.md — Configuration surface table (Phase A → gated by Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Rows are the cross-product of the
axes the C code actually branches on, pruned to the combinations
`c_src/src/lib.c` treats differently.

## Public entry points

`c_src/include/lib.h` exposes exactly one:

```c
int dataentry(int a, int b, int c, int d);   // (mode, param1, param2, param3)
```

All other functions are `static`, so `dataentry`'s `mode`/`param` arguments *are*
the configuration mechanism. The lowest-level routines are reached as follows —
every one is driven directly by the rows below, not only through a convenience
wrapper:

| lowest-level C routine | reached from | rows |
|------------------------|--------------|------|
| `create_entries`  | mode 1, mode 2 | 1–10, 15–22 |
| `find_entry`      | mode 1 | 1–10 |
| `modify_entries`  | mode 2 | 15–22 |
| `calculate_lookup`| mode 3 | 23–26 |
| `process_name`    | default arm | 27–30 |
| `lookup_table`    | mode 3 | 23–26 |
| `sprintf("Entry_%d")` + `strcpy` into `DataEntry::name` | mode 1, mode 2 | 1–10, 15–22 |
| `strcpy(buffer, found->name)` | mode 1, entry found | 1–7 |

## Axes the C branches on

* **A1 `mode`** — `switch (mode)`: `1` / `2` / `3` / `default` (4 branches).
* **A2 `count` selection** — `count = param1 > 0 ? param1 : <5 mode 1 | 3 mode 2>`.
  Distinct shapes: `param1 <= 0` (default count), `param1 == 1` (single element),
  `2..10` (small), `> MAX_ENTRIES(10)` (constant is *not* enforced), large.
* **A3 `param2` as index** (mode 1) — `target_id = 100 + param2`: first element
  (`0`), last element (`count-1`), interior, out of range (→ `ERRORS.md`).
* **A4 `param2` as multiplier** (mode 2) — `1` (identity), `-1` (negation),
  `>1`, negative, magnitude large enough to wrap `int`, `0` (→ E11).
* **A5 `param3` addend** — `0` (mode 2 line 174 / mode 3 line 183 add nothing
  observable), positive, negative, `INT_MAX`/`INT_MIN` (wrapping add).
  Not read at all in mode 1 and in the default arm.
* **A6 lookup cell** (mode 3) — the full 4×3 `lookup_table` grid.
* **A7 element/string shape** — `sizeof(DataEntry) == 40`, `NAME_LENGTH == 32`,
  `"Entry_%d"` rendering for ids `100..` and `200..` (1–4 digit decimals, and
  wrapped-negative ids for huge counts).
* **A8 byte order / element type** — the ABI shape is fixed: four `int` (`i32`)
  arguments and one `int` return, `#[repr(C)] struct DataEntry { i32, i32, [c_char;32] }`
  with 40-byte stride. Verified implicitly by every row (a stride or field-order
  mismatch changes `find_entry`/`modify_entries` results).

Every row is exercised with **many randomized inputs** (fixed seed, SplitMix64)
over the free parameters of that row, not a single hand-picked value.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `dataentry` mode 1 → `create_entries`+`find_entry`+`strcpy` | `param1 <= 0` (count defaults to 5), `param2 = 0` → first element found | [x] |
| 2 | mode 1 | `param1 <= 0` (count 5), `param2 = 4` → **last** element found | [x] |
| 3 | mode 1 | `param1 <= 0` (count 5), `param2` interior `1..3`, randomized `param3` (ignored) | [x] |
| 4 | mode 1 | `param1 == 1` (single-element array), `param2 == 0` | [x] |
| 5 | mode 1 | `param1` in `2..=10` (small counts), `param2` randomized in range | [x] |
| 6 | mode 1 | `param1 > MAX_ENTRIES` (11..=64) — constant is **not** enforced, `param2` in range | [x] |
| 7 | mode 1 | `param1` large (256..=4096, multi-page alloc, 3–4 digit ids ⇒ longer `"Entry_%d"`), `param2` in range | [x] |
| 7b | mode 1 | `param1` medium-large where `malloc` still SUCCEEDS (2^16 … 2^22+7 ⇒ up to 168 MiB, 7-digit ids = longest `"Entry_%d"` on the success path), `param2` ∈ {first, 1, mid, last-1, last, just-past-end} | [x] |
| 8 | mode 1 | full sweep: every `param2` in `0..count` for every `count` in `1..=24` (exhaustive index×size grid) | [x] |
| 9 | mode 1 | `param3` swept over extremes (`0`, `±1`, `INT_MIN`, `INT_MAX`) to confirm it is **unused** in mode 1 | [x] |
| 10 | mode 1 | randomized `(param1, param2, param3)` over the whole `int` range, small-count clamp — mixes found/not-found | [x] |
| 15 | `dataentry` mode 2 → `create_entries`+`modify_entries` | `param1 <= 0` (count defaults to 3), `param2 == 1` (identity multiplier), `param3 == 0` | [x] |
| 16 | mode 2 | `param1 <= 0` (count 3), `param2 == -1`, randomized `param3` | [x] |
| 17 | mode 2 | `param1 == 1` (single element), randomized non-zero `param2`, randomized `param3` | [x] |
| 18 | mode 2 | `param1` in `2..=10`, randomized non-zero `param2`, randomized `param3` | [x] |
| 19 | mode 2 | `param1 > MAX_ENTRIES` (11..=64), randomized `param2`, `param3` | [x] |
| 20 | mode 2 | `param1` large (256..=4096) ⇒ `total` accumulates through many signed wraps | [x] |
| 20b | mode 2 | `param1` medium-large (2^16 … 2^23) × extreme multipliers ⇒ millions of wrapping `total +=` steps | [x] |
| 21 | mode 2 | `param2` at extremes (`INT_MAX`, `INT_MIN`, `±2^16`, `±2^24`) ⇒ `value * multiplier` wraps per element | [x] |
| 22 | mode 2 | `param3` at extremes (`INT_MAX`, `INT_MIN`) ⇒ wrapping `result += param3` | [x] |
| 23 | `dataentry` mode 3 → `calculate_lookup` | exhaustive in-range grid: all 12 `(param1, param2)` cells, `param3 == 0` | [x] |
| 24 | mode 3 | all 12 cells × randomized `param3` (wrapping add) | [x] |
| 25 | mode 3 | all 12 cells × `param3 ∈ {INT_MIN, -1, 0, 1, INT_MAX}` (boundary addend) | [x] |
| 26 | mode 3 | in-range boundary corners `(0,0)`, `(0,2)`, `(3,0)`, `(3,2)` × extreme `param3` | [x] |
| 27 | `dataentry` default → `process_name`+`strcpy`+`strlen` | `mode == 0`, `param1` randomized (result `8 * param1`) | [x] |
| 28 | default arm | `mode` randomized outside `{1,2,3}` (negative, `>3`, `INT_MIN`, `INT_MAX`), `param1` randomized | [x] |
| 29 | default arm | `param2`/`param3` swept over extremes to confirm they are **unused** | [x] |
| 30 | default arm | `param1 ∈ {0, ±1, INT_MIN, INT_MAX, 2^28, 3·2^28}` ⇒ `8 * param1` wraps | [x] |
| 31 | `dataentry` (all modes) | mode swept over every value in `-8..=8` × randomized params — confirms switch dispatch incl. `MAX_ENTRIES`-adjacent modes | [x] |
| 32 | `dataentry` (all modes) | **fully randomized fuzz**: all four args uniform over `i32`, with `param1` clamped in the mode-1/2 arms to keep allocations sane; 200 000 cases | [x] |

(Row numbers 11–14 are intentionally absent: they were folded into rows 8–10 and
the `ERRORS.md` rows E1–E6 during derivation, and the numbering is kept stable so
the checked-off rows here match the test names.)

30 rows, all passing — one `#[test]` per row in `tests/phase_b_configs.rs`, named
`rowNN_...` to match.

## Not observable through the public API

`sprintf("Entry_%d", id)` + `strcpy` into `DataEntry::name` (and the
`strcpy(buffer, found->name)` at line 158) run on every mode-1/mode-2 call, but
`dataentry` returns only an `int` and `name`/`buffer` are discarded, so the
rendered string cannot be read back by any external caller. Rows 7, 7b, 20 and
20b nonetheless drive that code with the longest ids the success path can reach
(7 digits, 13 bytes + NUL, well under `NAME_LENGTH == 32`), which is what makes a
buffer overrun there impossible rather than merely unobserved.

## Harness self-test

Because a differential suite that never fails proves nothing,
`mutation_check.sh` injects 19 deliberate bugs into `src/lib.rs` (one per live
branch: sentinels, base ids, default counts, loop bounds, table cells, string
lengths, allocation size, switch dispatch) and requires the suite to FAIL on
each. It also carries 3 negative controls that MUST survive, which pin down
exactly which C branches are unreachable or semantically equivalent:

* `count <= 0` in `create_entries` (unreachable — see `ERRORS.md` E8/E9)
* `found->id == 0` in mode 1 (unreachable — E5)
* `if (temp_value)` in `modify_entries` — an *equivalent* mutant: when
  `temp_value == 0` the guarded body computes `0 * multiplier == 0` and adds
  `0`, so guarding or not guarding gives identical results (E11 context).

Result: 19/19 killed, 3/3 controls survived.
