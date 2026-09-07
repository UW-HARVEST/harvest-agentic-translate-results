# CONFIGS.md — configuration-surface table (valid inputs)

The public API is a single entry point with four `int` axes:

```c
int dataentry(int mode, int param1, int param2, int param3);
```

There are no runtime option setters, no global state, no `#ifdef`, and no
compile-time features (`translation/Cargo.toml` has no `[features]` table), so
the configuration surface is entirely the cross-product of the axes the `switch`
and the `if`s actually branch on.

## Axes derived from the C source

| axis | values the C distinguishes | source of the branch |
|---|---|---|
| `mode` | `1`, `2`, `3`, everything else (`default`) | `switch (mode)` L136 |
| mode 1 `param1` | `> 0` (count = param1) vs `<= 0` (count = 5) | `count = param1 > 0 ? param1 : 5` L138 |
| mode 1 `param1` magnitude | 1, 2, small, large-but-allocatable, unallocatable | `malloc(count*40)` L86 |
| mode 1 `param2` | in `[0, count)` (hit) vs `< 0` or `>= count` (miss) | `find_entry(.., 100 + param2)` L151 |
| mode 1 `param3` | ignored entirely on this path | — |
| mode 2 `param1` | `> 0` (count = param1) vs `<= 0` (count = 3) | `count = param1 > 0 ? param1 : 3` L164 |
| mode 2 `param2` | `0` (total collapses to 0) vs nonzero; positive vs negative; magnitude that overflows `int` | `current->value = temp_value * multiplier` L121 |
| mode 2 `param3` | added only when total != 0 | `if ((result = modify_entries(...))) result += param3` L172 |
| mode 3 `param1` (row) | `0,1,2,3` valid; else rejected | `param1 >= 0 && param1 < 4` L181 |
| mode 3 `param2` (col) | `0,1,2` valid; else rejected | `param2 >= 0 && param2 < 3` L181 |
| mode 3 `param3` | free addend, incl. values that overflow | `result = lookup_result + param3` L182 |
| default `param1` | free multiplier, incl. `0`, negative, overflowing | `result = count * param1` L192 |
| `NAME_LENGTH` shape | id strings `"Entry_<id>"`: 1-, 2-, 3-digit and negative ids exercise different `sprintf` widths into the 32-byte field | `sprintf(temp_name, "Entry_%d", base_id + i)` L98 |

Internal call hierarchy (tested bottom-up through the one public export, since
the helpers are `static` and not separately callable):

```
dataentry
 ├ mode 1: create_entries -> find_entry
 ├ mode 2: create_entries -> modify_entries
 ├ mode 3: calculate_lookup   (lookup_table)
 └ default: process_name      (strcpy/strlen)
```

## Configuration rows

Each row is exercised with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) for the free axes, plus the listed boundary values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `dataentry` mode 1 → `create_entries`, `find_entry` | `param1 > 0`, `param2` in `[0, param1)` — hit, small counts 1..3 | [x] |
| 2 | `dataentry` mode 1 → `create_entries`, `find_entry` | `param1 > 0` randomized in `[1, 512]`, `param2` randomized in `[0, param1)` — hit, many counts/indices | [x] |
| 3 | `dataentry` mode 1 | `param1 <= 0` (count defaults to 5), `param2` in `[0,5)` — hit on the default count | [x] |
| 4 | `dataentry` mode 1 | `param1 <= 0` (count 5), `param2` in `{5, 6, 100, -1, INT_MIN, INT_MAX}` — miss | [x] |
| 5 | `dataentry` mode 1 | `param1 > 0`, `param2 == param1` and `param2 == param1 - 1` — exact boundary of the found range | [x] |
| 6 | `dataentry` mode 1 | `param1 == 1` (single entry), `param2 == 0` — "one" shape | [x] |
| 7 | `dataentry` mode 1 | large allocatable count (`param1` in `{1_000, 65_536, 200_000}`), `param2` at 0, mid, `count-1` — many shape + wide `sprintf` ids | [x] |
| 8 | `dataentry` mode 1 | `param3` randomized while `param1`/`param2` fixed — confirms `param3` is ignored identically | [x] |
| 9 | `dataentry` mode 2 → `create_entries`, `modify_entries` | `param1 <= 0` (count defaults to 3), `param2` randomized nonzero, `param3` randomized | [x] |
| 10 | `dataentry` mode 2 | `param1 > 0` randomized in `[1, 512]`, `param2` randomized nonzero, `param3` randomized | [x] |
| 11 | `dataentry` mode 2 | `param2 == 0` → total 0, `param3` **not** added (randomized `param1`, `param3`) | [x] |
| 12 | `dataentry` mode 2 | `param2` negative randomized — negative multiplier, negative totals | [x] |
| 13 | `dataentry` mode 2 | `param2` / `param1` large enough that the total overflows `int` (`param2` in `{INT_MAX, INT_MIN, 0x10001, -0x10001}`, `param1` in `{1,3,64,512}`) | [x] |
| 14 | `dataentry` mode 2 | `param1 == 1` (one entry) and large allocatable `param1` (`{1_000, 65_536}`) | [x] |
| 15 | `dataentry` mode 3 → `calculate_lookup` | all 12 valid `(param1, param2)` cells of the 4x3 table, `param3 == 0` | [x] |
| 16 | `dataentry` mode 3 | all 12 valid cells x randomized `param3` (incl. values that overflow the addition) | [x] |
| 17 | `dataentry` mode 3 | valid cells with `param3` in `{INT_MAX, INT_MIN, -1, 0, 1}` — addition boundaries | [x] |
| 18 | `dataentry` default → `process_name` | `mode` in `{0, 4, 5, 6, 7, 100, -1, -7, INT_MIN, INT_MAX}`, `param1 == 1` | [x] |
| 19 | `dataentry` default | randomized `mode` (excluding 1/2/3) x randomized `param1` — `8 * param1` incl. overflow | [x] |
| 20 | `dataentry` default | `param1` in `{0, 1, -1, INT_MAX, INT_MIN, 0x1000_0000}` — multiplier boundaries/overflow | [x] |
| 21 | `dataentry` default | randomized `param2`, `param3` — confirms both are ignored identically | [x] |
| 22 | `dataentry` all modes | fully randomized 4-tuple sweep over a bounded space (all four axes at once, 200k+ cases) — cross-product smoke of every branch interaction | [x] |
| 23 | `dataentry` all modes | exhaustive small-cube sweep `mode in [-4,8]` x `param1 in [-4,8]` x `param2 in [-4,8]` x `param3 in [-4,8]` | [x] |
| 24 | `dataentry` modes 1, 2 | allocation-threshold band: `param1 = 2^20 .. 2^26` (allocatable, up to 671 MB and 2^26 wrapped totals) crossed with index `0` and `count-1`, then `2^28 .. INT_MAX` (not allocatable) | [x] |

## Row -> test mapping

| rows | test file | tests |
|---|---|---|
| 1-23 | `tests/phase_b_configs.rs` | `row01_*` .. `row23_*` (23 tests) |
| 24 + cross-row brute force | `tests/phase_e_sweeps.rs` | `sweep_allocation_threshold_band`, `sweep_mode1_dense`, `sweep_mode2_dense`, `sweep_mode2_wrapping_totals`, `sweep_mode3_exhaustive_cube`, `sweep_default_dense`, `sweep_random_full_range`, `sweep_random_unclamped` (8 tests) |

All rows verified passing in four configurations: `{debug, release}` x
`{default features, --no-default-features}` (see `run_all_features.sh`).
Approximately 1.5 million differential `dataentry` calls, every one of them made
through the `dataentry` dynamic symbol of both shared objects.

### Allocation-threshold detail

On this host the `malloc` cut-off falls between `2^27` entries (5.4 GB,
succeeds) and `2^28` entries (10.7 GB, fails). C and Rust agree on both sides of
that transition and on the wrapped mode-2 totals for every allocatable count
scanned (`2^20` through `2^27`), so the C `malloc(count * sizeof(DataEntry))`
and the Rust `Vec::try_reserve_exact` size computations are equivalent.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`; there is
no `add_executable`, and the Rust crate declares `crate-type = ["cdylib"]` with
no `[[bin]]`. **The project builds no binary driver**, so the stdout-comparison
item of the completion gate is not applicable.
