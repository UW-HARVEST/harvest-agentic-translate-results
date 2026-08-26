# CONFIGS.md — Phase B configuration-surface table

## Build-time configuration axes

* `translated_rust/Cargo.toml` has **no `[features]` section** ⇒ the only valid
  feature combination is the empty set. Both `cargo check` and
  `cargo check --no-default-features` are the same configuration and both are
  run by `run_all_features.sh`.
* `c_src/CMakeLists.txt` has no `option()`, no `target_compile_definitions`, and
  no `#ifdef` in `src/lib.c` ⇒ a single C configuration (default `CMAKE_BUILD_TYPE`,
  shared library, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`).

## Runtime configuration axes (derived from the `switch`/`if` branches in lib.c)

| axis | values the C code distinguishes |
|---|---|
| `mode` (selector, lib.c:141) | `1` (create+find+free), `2` (create+modify+free), `3` (table lookup), anything else → `default:` |
| `param1` in modes 1/2 (lib.c:143,165) | `> 0` ⇒ `count = param1`; `<= 0` ⇒ `count = 5` (mode 1) / `3` (mode 2). Shapes: 1, small, large, `INT_MAX` (alloc failure) |
| `param2` in mode 1 (lib.c:151) | target id `100 + param2`: first (`0`), last (`count-1`), middle, one-past (`count`), negative, wraparound extremes |
| `param2` in mode 2 (lib.c:173) | multiplier: `0` (⇒ total 0, `param3` skipped), `1`, positive, negative, magnitude causing signed wraparound |
| `param1`,`param2` in mode 3 (lib.c:181) | row ∈ {0,1,2,3} × col ∈ {0,1,2} — all 12 in-range cells are distinct values |
| `param3` | added only in modes 2 and 3, and only when the preceding value is non-zero; extremes `INT_MAX`/`INT_MIN` exercise wraparound |
| `param1` in `default:` | multiplier of `strlen("TestName") == 8`: `0`, positive, negative, extremes |
| element/data shape | `DataEntry{int id; int value; char name[32]}`, `sizeof == 40`; ids `base_id+i`, values `(base_id+i)*10`, names `"Entry_%d"` (1–11 digits) |
| entry-count shape | empty-ish (`count == 1`), few (`3`,`5`), many (up to 2 000 000 = 80 MB), allocation-failing (`INT_MAX`) |

## Entry points

`dataentry` is the only exported entry point, but it composes every internal
level of the library. Each row below states which internal call chain it drives
so the low-level helpers (`create_entries` → `sprintf`/`strcpy`,
`find_entry` pointer walk, `modify_entries` accumulate, `calculate_lookup`
table index, `process_name`) are each exercised directly rather than only
through one convenience path.

All rows are differential: both `.so`s are loaded with `libloading` and called
through the `dataentry` FFI export; each row runs **many randomized inputs**
from a fixed-seed PRNG (`SEED = 0x5EED_1234_ABCD_9876`) plus its hand-picked
boundary values.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `dataentry`→`create_entries`→`find_entry` | mode 1, `param1 <= 0` (count=5 default), `param2` ∈ 0..4 (hit), `param3` random (ignored) | [x] |
| 2  | mode 1 chain | mode 1, `param1 <= 0` (count=5), `param2` outside 0..4 (miss ⇒ -2), `param3` random | [x] |
| 3  | mode 1 chain | mode 1, `param1 == 1` (single element), `param2 == 0` (first == last) | [x] |
| 4  | mode 1 chain | mode 1, `param1` random 2..64, `param2 == 0` (first element hit) | [x] |
| 5  | mode 1 chain | mode 1, `param1` random 2..64, `param2 == count-1` (last element hit) | [x] |
| 6  | mode 1 chain | mode 1, `param1` random 2..64, `param2` random in 1..count-2 (middle hit) | [x] |
| 7  | mode 1 chain | mode 1, `param1` random 2..64, `param2 == count` (one past ⇒ miss) | [x] |
| 8  | mode 1 chain | mode 1, `param1` random 2..64, `param2` random negative (miss) | [x] |
| 9  | mode 1 chain | mode 1, `param1` random 1..4096 (multi-page alloc, 3-4 digit names crossing `sprintf` widths), `param2` random in range | [x] |
| 10 | mode 1 chain | mode 1, `param1` large (100_000..2_000_000 ⇒ 4–80 MB, 5–7 digit names), `param2` random in range and out of range | [x] |
| 11 | mode 1 chain | mode 1, `param2` ∈ {`INT_MAX`, `INT_MIN`, `INT_MAX-99`, `-100`, `-101`} (wraparound of `100+param2`) | [x] |
| 12 | mode 1 chain | mode 1, fully random `param1` ∈ -8..64, `param2` ∈ -8..72, `param3` random full-range | [x] |
| 13 | `dataentry`→`create_entries`→`modify_entries` | mode 2, `param1 <= 0` (count=3 default), multiplier random non-zero small, `param3` random | [x] |
| 14 | mode 2 chain | mode 2, `param1 == 1` (single element), multiplier random | [x] |
| 15 | mode 2 chain | mode 2, `param1` random 2..64, multiplier `== 0` ⇒ total 0 ⇒ `param3` NOT added | [x] |
| 16 | mode 2 chain | mode 2, `param1` random 2..64, multiplier `== 1` (identity sum) | [x] |
| 17 | mode 2 chain | mode 2, `param1` random 2..64, multiplier negative (`-1` and random negative) | [x] |
| 18 | mode 2 chain | mode 2, `param1` random 2..64, multiplier random full-range i32 (signed wraparound of `value*multiplier` and of `total`) | [x] |
| 19 | mode 2 chain | mode 2, `param1` random, `param3` ∈ {`INT_MAX`, `INT_MIN`, `0`, ±1} (wraparound of `total + param3`) | [x] |
| 20 | mode 2 chain | mode 2, `param1` large (10_000..1_000_000), multiplier random small (long accumulation, wraparound) | [x] |
| 21 | mode 2 chain | mode 2, fully random `param1` ∈ -8..256, `param2`,`param3` full-range | [x] |
| 22 | `dataentry`→`calculate_lookup` | mode 3, all 12 in-range `(row, col)` cells × `param3 == 0` | [x] |
| 23 | mode 3 | mode 3, all 12 cells × random full-range `param3` (wraparound) | [x] |
| 24 | mode 3 | mode 3, all 12 cells × `param3` ∈ {`INT_MAX`, `INT_MIN`, ±1} | [x] |
| 25 | mode 3 | mode 3, boundary rows/cols `(0,0)`, `(0,2)`, `(3,0)`, `(3,2)` × random `param3`, `param1`/`param2` unused-arg cross-check | [x] |
| 26 | `dataentry`→`process_name` (default branch) | `mode == 0`, `param1` random small (result `8 * param1`) | [x] |
| 27 | default branch | `mode` ∈ {`-1`, `4`, `5`, `1000`, `INT_MIN`, `INT_MAX`} × `param1` random | [x] |
| 28 | default branch | default mode, `param1` ∈ {`0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, `268435456`} (`8*param1` wraparound) | [x] |
| 29 | default branch | default mode, random `mode` (excluding 1..3) × random full-range `param1`,`param2`,`param3` | [x] |
| 30 | all branches | global fuzz: `mode` ∈ -4..8, `param1` ∈ -16..96, `param2`/`param3` from a mixed generator (small, boundary, full-range) — 20 000 cases | [x] |
| 31 | all branches | global fuzz with full-range random `mode` (hits `default:` almost always) and full-range params, allocation-safe `param1` clamp — 20 000 cases | [x] |
| 32 | `dataentry` (modes 1 & 2) | repeated-call lifecycle: 50 000 allocating calls per implementation, result stability + bounded RSS (catches a missing `free`/`dealloc`) | [x] |
| 33 | `dataentry` (all branches) | interleaved probe set (64 random `(mode,p1,p2,p3)` tuples × 50 rounds) compared against freshly-`dlopen`ed baselines — proves both are stateless across modes | [x] |
| 34 | `dataentry` (all branches) | reentrancy: 8 threads × 4 rounds × 256 random cases, each thread with its own `dlopen`ed pair, all compared against the single-threaded results | [x] |

## Verification runs

```
cargo build --release && cargo test --release      # 33 + 12 tests pass
cargo build          && cargo test                 # debug profile (overflow checks) pass
./run_all_features.sh                              # every feature combo + symbol parity
C_SO_PATH=$TMPDIR/libc_O0.so cargo test --release   # C oracle at -O0  pass
C_SO_PATH=$TMPDIR/libc_O2.so cargo test --release   # C oracle at -O2  pass
C_SO_PATH=$TMPDIR/libc_O3.so cargo test --release   # C oracle at -O3  pass
```

The last three runs pin the signed-overflow-dependent rows (mode 2 accumulation,
`8 * param1`, `100 + param2`, `cell*2 + param3`) against gcc at three
optimization levels, so the Rust `wrapping_*` choices match the C as built by
its own CMake default *and* under optimization.

## Suite discrimination (mutation testing)

The suite was validated by injecting 15 deliberate bugs into `src/lib.rs`,
rebuilding the Rust `.so` and re-running: **12 were caught** (wrong sentinel,
wrong default counts, off-by-one in the `find_entry` end pointer, wrong
`value`/lookup multipliers, inverted `process_name` guard, `param3` added when
the total is 0, wrong `strlen` source string, `param1`↔`param2` swap, both mode-3
bound relaxations — the last two abort the harness on the out-of-bounds table
index). The 3 survivors are provably **unobservable through the only exported
entry point** and are therefore not test gaps:

| mutation | why it cannot be observed |
|---|---|
| `100.wrapping_add(param2)` → `saturating_add` | differs only when `100+param2` overflows; both the wrapped and the saturated target id miss every generated id (a hit needs `count ≈ 2^31`, which fails to allocate first) ⇒ `-2` either way |
| removing the `nbytes > isize::MAX` guard in the allocator shim | unreachable: `count ∈ 1..=INT_MAX` ⇒ `nbytes ≤ 85 GiB ≪ isize::MAX`; the negative-`count` path is dead because `count = param1 > 0 ? param1 : 5/3` |
| `"Entry_"` → `"entry_"` in the `sprintf` shim | entry names are only ever copied into `dataentry`'s local `buffer`, which never contributes to the return value; no C code path reads it back |
