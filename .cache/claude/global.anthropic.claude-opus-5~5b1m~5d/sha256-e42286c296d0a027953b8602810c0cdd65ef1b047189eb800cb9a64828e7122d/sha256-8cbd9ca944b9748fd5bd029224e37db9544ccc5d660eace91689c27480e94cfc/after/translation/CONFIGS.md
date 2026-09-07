# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

**Public ABI.** `c_src/include/lib.h` exposes exactly one entry point:
`int dataentry(int mode, int param1, int param2, int param3)`. Every other
function (`find_entry`, `process_name`, `calculate_lookup`, `create_entries`,
`modify_entries`) is `static` and has **no dynamic symbol** (see `SYMBOLS.md`),
so it is unreachable across the FFI wall by construction. The "lowest-level
entry points" are therefore reached *through* `dataentry`, and the table below
is built so that every static helper and every one of its internal branches is
driven directly:

| static helper | reached via |
|---|---|
| `create_entries` | mode 1 (base_id 100), mode 2 (base_id 200) |
| `find_entry` | mode 1 |
| `modify_entries` | mode 2 |
| `calculate_lookup` | mode 3 |
| `process_name` | default mode |

**Axis 1 — runtime mode (`mode`), the library's only option/flag.** The
`switch` at lib.c:141 distinguishes exactly four states: `1`, `2`, `3`, and
`default:` (every other `int`). Each toggles a completely different pipeline.

**Axis 2 — `param1` shape.**
* mode 1/2: element **count**. The ternary `param1 > 0 ? param1 : 5` (mode 1) /
  `: 3` (mode 2) splits it into `> 0` vs. `<= 0`; sizes matter (empty-ish/one/
  few/many), and `count * sizeof(DataEntry)` makes huge values a distinct shape.
* mode 3: lookup-table **row**, checked against `[0, 4)`.
* default: multiplicand of `strlen(buffer) * param1` — overflow shape.

**Axis 3 — `param2` shape.**
* mode 1: **id offset**; `find_entry` searches for `100 + param2`, so
  `0 <= param2 < count` hits and anything else misses (first / middle / last
  element are distinct positions in the pointer-walk loop).
* mode 2: **multiplier** in `value * multiplier`; `0`, `1`, negative, and
  overflow-inducing magnitudes are distinct.
* mode 3: lookup-table **column**, checked against `[0, 3)`.
* default: ignored.

**Axis 4 — `param3` shape.** Added to `result` in modes 2 and 3 only, and in
mode 2 only when `result != 0`. Ignored in mode 1 and default.

**Axis 5 — byte-level string shape.** `sprintf(temp_name, "Entry_%d", base_id+i)`
then `strcpy` into `name[32]`. Digit-count of `base_id + i` (1..7+ digits,
crossing the 100→1000→10000... boundaries) is a distinct shape, as is the
`strcpy(buffer, found->name)` in mode 1 and the `"Default"` → `"TestName"`
rewrite in the default arm.

**`#ifdef` axis:** none. There is no conditional compilation in `lib.c`, and
`translation/Cargo.toml` declares no `[features]`, so there is exactly one build
configuration.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed, see
`tests/phase_b_configs.rs`), not one hand-picked value. Rows 31-36 are
EXHAUSTIVE enumerations (`tests/phase_b_exhaustive.rs`) that leave no gaps in a
bounded box around every boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1 == 1` (single entry), `param2 == 0` → hits the only element | [x] |
| 2 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1` in 2..=10, `param2 == 0` → hits **first** element (loop exits on first iteration) | [x] |
| 3 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1` in 2..=10, `param2 == param1-1` → hits **last** element (loop walks to `end-1`) | [x] |
| 4 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1` in 3..=10, `param2` strictly interior → hits a **middle** element | [x] |
| 5 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1 == 0` → count defaults to 5; `param2` random in -8..=8 | [x] |
| 6 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1 < 0` (incl. `INT_MIN`) → count defaults to 5; `param2` random | [x] |
| 7 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param1` large (1_000 .. 200_000) → `Entry_%d` names of 4..7 digits, deep pointer walk; `param2` random in range and out | [x] |
| 8 | `dataentry` → `create_entries`,`find_entry` | mode 1, `param2` random full-`i32` range (mostly misses → `-2`), `param1` random valid | [x] |
| 9 | `dataentry` | mode 1, `param3` random full-`i32` range — must be **ignored** (result independent of `param3`) | [x] |
| 10 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1 == 1`, `param2 == 1` (identity multiplier), `param3 == 0` | [x] |
| 11 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1` in 1..=10, `param2 == 1`, `param3` random | [x] |
| 12 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1` in 1..=10, `param2 == 0` → all values become 0, total 0, **`param3` not added** | [x] |
| 13 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1` in 1..=10, `param2` small negative, `param3` random | [x] |
| 14 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1 == 0` → count defaults to 3; `param2`,`param3` random | [x] |
| 15 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1 < 0` (incl. `INT_MIN`) → count defaults to 3 | [x] |
| 16 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param2` full-`i32` random → `value * multiplier` **wraps** on every entry | [x] |
| 17 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param1` large (1_000 .. 200_000) → running `total` wraps repeatedly; `param2`,`param3` random | [x] |
| 18 | `dataentry` → `create_entries`,`modify_entries` | mode 2, `param3 == INT_MAX` / `INT_MIN` → `result + param3` wraps | [x] |
| 19 | `dataentry` → `calculate_lookup` | mode 3, **all 12** valid `(row, col)` pairs `param1` in 0..4 x `param2` in 0..3, `param3 == 0` | [x] |
| 20 | `dataentry` → `calculate_lookup` | mode 3, all 12 valid pairs x `param3` random full-`i32` (wrapping `lookup*2 + param3`) | [x] |
| 21 | `dataentry` → `calculate_lookup` | mode 3, corner cells `(0,0)`, `(0,2)`, `(3,0)`, `(3,2)` — table edges | [x] |
| 22 | `dataentry` → `process_name` | default arm, `mode == 0`, `param1` random small → `8 * param1` | [x] |
| 23 | `dataentry` → `process_name` | default arm, `mode == 4` (one past the last case), `param1` random | [x] |
| 24 | `dataentry` → `process_name` | default arm, `mode` random negative / `mode > 4` / `INT_MIN` / `INT_MAX`, `param1` random | [x] |
| 25 | `dataentry` → `process_name` | default arm, `param1` full-`i32` random → `8 * param1` **wraps** (`param1 == INT_MAX`, `INT_MIN`, `0x2000_0000`) | [x] |
| 26 | `dataentry` | default arm, `param2`/`param3` random — must be **ignored** | [x] |
| 27 | `dataentry` (all four modes) | uniformly random `mode` over the full `i32` range x uniformly random `param1..3` — unconstrained fuzz over the whole cross-product (100_000 cases) | [x] |
| 28 | `dataentry` (modes 1 and 2) | `param1` restricted to a small set so the same allocation size is reused, repeated many times → checks no cross-call state leaks (both libs are stateless) | [x] |
| 29 | `dataentry` (mode 1) | `param1` chosen so `base_id + i` crosses a digit-count boundary (`param1` = 900, 901, 9_900, 9_901) → `Entry_%d` string length changes mid-loop | [x] |
| 30 | `dataentry` (mode 2) | `param1` chosen at digit-count boundaries for `base_id 200` (`param1` = 800, 801, 9_800, 9_801) | [x] |

| 31 | `dataentry` (all arms) | EXHAUSTIVE box: `mode` in -3..=6 x `param1` in -3..=14 x `param2` in -3..=14 x `param3` in -2..=2 (16 200 points, no gaps) | [x] |
| 32 | `dataentry` (all arms) | EXHAUSTIVE box: `mode` in -2..=5 x `param1` in -2..=12 x `param2` in -2..=12 x `param3` in {`INT_MIN`, `INT_MIN+1`, -1, 0, 1, `INT_MAX-1`, `INT_MAX`} (12 600 points) | [x] |
| 33 | `dataentry` → `calculate_lookup` | EXHAUSTIVE: `param1` in -2..=6 x `param2` in -2..=5 x a dense 601-wide `param3` band straddling `INT_MAX` and `INT_MIN` (wrap frontier) | [x] |
| 34 | `dataentry` → `find_entry` | EXHAUSTIVE: mode 1, every `count` in 1..=24 x every `param2` in `-(count+3)..=(count+3)` — the entire hit/miss frontier | [x] |
| 35 | `dataentry` → `modify_entries` | EXHAUSTIVE: mode 2, every `count` in 1..=16 x every multiplier in -600..=600 x `param3` in {0, 1, -1}, plus the power-of-two and `INT_MIN`/`INT_MAX` multipliers | [x] |
| 36 | `dataentry` dispatch | EXHAUSTIVE: every `mode` in -256..=256 x a grid of `param1`/`param2`, plus 257-wide bands at `INT_MIN` and near `INT_MAX` — proves the switch is not narrowed to char/short | [x] |

### Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(... SHARED src/lib.c)` — no
`add_executable`, and `translation/Cargo.toml` declares
`crate-type = ["cdylib", "rlib"]` with no `[[bin]]` (the `rlib` exists only so
`cargo test` rebuilds the cdylib; nothing links it). There is no driver program, so the
"compare C and Rust binary stdout" gate is **not applicable**. `dataentry`
itself writes nothing to stdout (`sprintf`, not `printf`).

## Observability note (found during Phase B)

`dataentry`'s only observable output is its `int` return value — it writes
nothing to stdout, takes no out-parameters, and returns no pointers. Three parts
of the C therefore cannot be distinguished from outside the FFI boundary, and
the mutation-based negative control in `mutation_check.sh` documents them as
known-equivalent rather than pretending to cover them:

1. **`sprintf(temp_name, "Entry_%d", base_id + i)` and the `strcpy` into
   `entries[i].name`.** `name` is never read back through the ABI. Mode 1 copies
   it into the local `buffer`, which `dataentry` discards. Overflowing
   `name[32]` is unreachable: for any `count` whose `count * 40` allocation
   succeeds, `base_id + i` renders to at most 15 characters.
2. **`process_name`'s return value.** The default arm overwrites `result` with
   `strlen(buffer) * param1` immediately afterwards, and `strlen("TestName")`
   is never 0.
3. **`sizeof(DataEntry)` / field layout.** The malloc size and the pointer
   stride derive from the same constant, so a different layout stays internally
   consistent and yields identical results.

Rows 29 and 30 still drive the digit-count boundaries of the `Entry_%d`
formatting, because a *buffer-overflowing* formatter WOULD be observable
(it would corrupt the following entry's `id`/`value`); those rows confirm no
such corruption happens in either library.

## Harness soundness

`cargo test` does not rebuild a `cdylib`-only artifact, so an early version of
this harness loaded a **stale** `.so` and passed every test vacuously. Two
independent guards now prevent that:

* `tests/common/mod.rs::assert_fresh` refuses to run if either `.so` is older
  than its sources, and `rust_so_path` rebuilds the cdylib into
  `target/harness` when cargo's own artifact is stale;
* `mutation_check.sh` injects 23 semantics-changing mutations into
  `src/lib.rs` one at a time and requires the suite to FAIL for each
  (**23 killed / 0 survived**), which proves the suite is really comparing the
  two libraries.
