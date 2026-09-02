# CONFIGS.md — configuration surface table (Phase A, gates Phase B)

## Axis enumeration (derived from the C source, not guessed)

**Runtime options / modes / flags:** `grep -n '#if\|#ifdef\|switch\|static .*=' c_src/src/driver.c`
finds **none**. The library has no global state, no setters, no init function, no
compile-time toggles, no environment variable reads. `include/driver.h` declares
exactly one prototype (`driver`). Therefore the only configuration axes are the
**input shapes of the entry points**.

**Public entry points (ALL of them, lowest level first — from `nm -D`, not just
the one declared in the header):**

| level | entry point | in `driver.h`? |
|-------|-------------|----------------|
| 0 (lowest) | `printLine(const char *)` | no (exported anyway) |
| 0 (lowest) | `printIntLine(int)` | no (exported anyway) |
| 1 | `bad(int)` | no (exported anyway) |
| 1 | `good(int)` (composes the two private `goodG2B`/`goodB2G`) | no (exported anyway) |
| 2 (wrapper) | `driver(int, int)` | yes |

**Input shapes the code actually special-cases** (the `if` conditions in
`driver.c` are the complete list of branch points):

* pointer: `NULL` vs non-`NULL` (`printLine`)
* string shape: empty / 1 byte / typical / very long (64 KiB); printf-specifier
  bytes; high (non-ASCII/non-UTF-8) bytes; interior content is otherwise opaque
* integer sign: `< 0` vs `>= 0` (`bad`, `goodB2G`)
* integer range vs the literal `10`: `< 10` vs `>= 10` (`goodB2G` only)
* in-bounds index position: `0` (first), `1..8` (middle), `9` (last)
* out-of-bounds distance for `bad`: `10`/`11` (absorbed by frame slack) vs
  `>= 12` (clobbers frame pointer / return address — UB, see ERRORS.md row 9)
* extremes: `INT_MIN`, `INT_MAX`
* `driver`'s two independent parameters ⇒ cross-product of the `good` axis with
  the `bad` axis

## Configuration rows

Each row is exercised with **many randomized inputs drawn from that row's class**
(fixed seed, deterministic xorshift PRNG in `tests/common/mod.rs`) — not one
hand-picked value — and compared byte-for-byte between the C `.so` and the Rust
`.so`, each call executed in a forked child with `stdout` redirected so the exact
byte stream and exit status are both captured.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `printLine` | non-null, empty string `""` | [x] |
| 2  | `printLine` | non-null, single ASCII byte | [x] |
| 3  | `printLine` | non-null, random printable ASCII, random length 1..=256 | [x] |
| 4  | `printLine` | non-null, random bytes over full `0x01..=0xFF` (non-UTF-8, high bytes) | [x] |
| 5  | `printLine` | non-null, contains printf specifiers (`%s`, `%d`, `%n`, `%%`) | [x] |
| 6  | `printLine` | non-null, very long buffer (64 KiB) — crosses stdio buffer size | [x] |
| 7  | `printLine` | non-null, string that is only `\n`/whitespace/`\t` | [x] |
| 8  | `printIntLine` | `0` | [x] |
| 9  | `printIntLine` | small positives `1..=9` (all) | [x] |
| 10 | `printIntLine` | small negatives `-1..=-9` (all) | [x] |
| 11 | `printIntLine` | random full-range `i32` (200 seeded samples) | [x] |
| 12 | `printIntLine` | extremes `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1` | [x] |
| 13 | `bad` | in-bounds first element, `data == 0` | [x] |
| 14 | `bad` | in-bounds middle, every `data` in `1..=8` | [x] |
| 15 | `bad` | in-bounds last element, `data == 9` | [x] |
| 16 | `bad` | in-bounds, random `data` in `0..=9` (100 seeded samples) | [x] |
| 17 | `bad` | negative (rejection path), random `data` in `INT_MIN..0` (100 seeded samples) | [x] |
| 18 | `bad` | OOB-but-absorbed `data == 10`, `data == 11` | [x] |
| 19 | `bad` | OOB-destructive `data >= 12` — UB, recorded not equality-asserted (ERRORS.md row 9) | [x] |
| 20 | `good` | in-bounds, every `data` in `0..=9` | [x] |
| 21 | `good` | in-bounds, random `data` in `0..=9` (100 seeded samples) | [x] |
| 22 | `good` | above range, `data == 10`, `11`, `12`, random `10..=INT_MAX` (100 samples) | [x] |
| 23 | `good` | negative, `data == -1`, `INT_MIN`, random `INT_MIN..0` (100 samples) | [x] |
| 24 | `driver` | both params in-bounds: full 10×10 cross-product `goodData,badData ∈ 0..=9` | [x] |
| 25 | `driver` | `goodData` in-bounds × `badData` negative | [x] |
| 26 | `driver` | `goodData` out-of-range (`>=10`) × `badData` in-bounds | [x] |
| 27 | `driver` | `goodData` negative × `badData` in-bounds | [x] |
| 28 | `driver` | both invalid: `goodData < 0` × `badData < 0` | [x] |
| 29 | `driver` | `goodData >= 10` × `badData` negative | [x] |
| 30 | `driver` | extremes: `{INT_MIN, -1, 0, 9, 10, INT_MAX}` × `{INT_MIN, -1, 0, 9, 10, 11}` cross-product (36 combos) | [x] |
| 31 | `driver` | random seeded pairs over the whole legal-for-both domain (`INT_MIN..=11`), 200 samples | [x] |
| 32 | interleaved sequence | `printLine` → `printIntLine` → `bad` → `good` → `driver` driven back-to-back in ONE process (composed pipeline / shared stdout stream, catches ordering + flush bugs invisible to per-call tests) | [x] |
| 33 | `goodG2B` dead branch | assert `good()` output never contains `ERROR: Array index is negative.` for any input (ERRORS.md row 12) | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` section ⇒ exactly one combination
(`default` == `--no-default-features` == ∅). `scripts/check_features.sh`
enumerates the feature list from `Cargo.toml` and re-runs the whole suite for
each combination it finds; with zero declared features it runs the single
configuration both with and without `--no-default-features`.

## Status

All 33 rows pass. Each row is driven through both `.so` exports in a fresh child
process and compared on the exact stdout byte stream plus exit status; the
randomized rows draw from a fixed-seed xorshift64\* PRNG
(`SEED = 0x5EED_1234_ABCD_0001`), so a failure is reproducible. Roughly 1,900
individual FFI calls are compared per run.

Two properties of the design worth noting:

* Rows are executed as **batches inside one child process**, so successive calls
  share one `stdout` FILE stream — that is what row 32 is for, and it catches
  ordering/flush divergence that per-call tests cannot see. When a batch
  diverges the harness replays its calls one at a time to name the first
  offending call.
* `bad`, `good`, `printLine` and `printIntLine` are exercised **directly**, not
  only through the `driver` convenience wrapper (`driver` is the only function
  declared in `driver.h`, but all five are exported, so all five are entry
  points).
