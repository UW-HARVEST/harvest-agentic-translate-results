# CONFIGS.md — configuration surface of `c_src/src/lib.c`

The mirror of `ERRORS.md`, for **valid** inputs. Axes were derived mechanically
from what the C actually branches on, not from what looks important.

## Axes the C code branches on

**A1 — `charinbuf`'s `mode` (`switch (mode)`, lines 101–207).** The one real
"mode" flag of the library. Six distinct behaviours: `0`, `1`, `2`, `3`, `4`,
`default`. Selecting a mode also selects *which* of the nine lower-level
functions run, so this axis multiplies into A3–A7.

**A2 — hidden state: `static int counter` (line 27).** Not a parameter, but a
configuration axis all the same: four exported functions mutate it, `charinbuf`
zeroes it on entry (line 100), and mode 3 both reads and writes it. The state a
call *starts* from therefore changes its result, so call **sequences** are an
axis, not just single calls. Each `.so` owns a private `counter`, so sequences
must be replayed against each library independently.

**A3 — `value` shape vs the `[0, 65535]` window** (`validate_uint16_range`,
reached by `mode 0`): below / at-lower-edge / interior / at-upper-edge / above.

**A4 — `int` wraparound shape for `value`/`opt1`/`opt2`** (`mode 3`, which does
`=`, `+=`, `*=`, `-=` on `counter`): magnitudes that do and do not overflow
`int`. Signed overflow is UB in C but the built `.so` wraps two's-complement,
and `multiply_counter` is reachable with overflowing operands, so the Rust
`wrapping_*` must reproduce the wrap bit-for-bit.

**A5 — buffer/needle shape for `find_char_in_buffer`**: `size` (0 / 1 / many /
longer-and-shorter than the match position) × target position (first / interior
/ last / absent) × target byte class (ASCII / `'\0'` / high-bit-set).

**A6 — string shape for `is_string_empty` and `create_buffer`**: empty / 1 byte /
long, first byte zero / ASCII / high-bit-set, embedded NUL (so `strlen` and the
`*str` dereference disagree with the buffer's true length).

**A7 — callback identity for `apply_operation`**: each of the four exported
counter operations, plus a callback defined *outside* both libraries (tests the
raw `int(*)(int)` ABI, and that Rust's `Option<extern "C" fn>` niche is a plain
pointer), × the `value` passed through.

**No build-time axes.** `Cargo.toml` has no `[features]`; `c_src` has no
`#ifdef`, `option()` or `target_compile_definitions`. One configuration only:
the empty feature set (`--no-default-features` == default).

## Configuration table

Every row: both `.so`s driven through `dlopen`/`dlsym` in that configuration,
return value **and** captured `stdout` bytes compared. "randomized" = 256+
inputs from a fixed-seed SplitMix64 stream (reproducible), not one hand-picked
value.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | `validate_uint16_range` | A3 exhaustive edges: `-1, 0, 1, 2, 65534, 65535, 65536` + `INT_MIN`, `INT_MAX` | [x] |
| 2  | `validate_uint16_range` | A3 randomized over the full `int` domain (in-range and out-of-range mixed) | [x] |
| 3  | `is_string_empty` | A6: empty `""` | [x] |
| 4  | `is_string_empty` | A6: every possible non-zero first byte `0x01..=0xFF`, incl. high-bit-set (signed-`char` trap) | [x] |
| 5  | `is_string_empty` | A6: embedded NUL — `"a\0b"` (non-empty) vs `"\0ab"` (empty), and long strings | [x] |
| 6  | `create_buffer` | A6: empty string (1-byte alloc) | [x] |
| 7  | `create_buffer` | A6: randomized lengths 1..=512 of random non-NUL bytes; asserts returned heap bytes + NUL terminator match, then `free`s | [x] |
| 8  | `create_buffer` | A6: bytes with the high bit set, and content that is not valid UTF-8 (must not be treated as a Rust `str`) | [x] |
| 9  | `find_char_in_buffer` | A5: `size == 0` with target present at `buffer[0]` | [x] |
| 10 | `find_char_in_buffer` | A5: `size == 1`, target at `buffer[0]` (hit) and target elsewhere (miss) | [x] |
| 11 | `find_char_in_buffer` | A5: target at first / interior / last position within `size`; returned pointer offset compared, not just null-ness | [x] |
| 12 | `find_char_in_buffer` | A5: `size` > match position (hit) and `size` <= match position (miss) over the same buffer | [x] |
| 13 | `find_char_in_buffer` | A5: `target == '\0'` inside `size` (a hit — not `strchr` semantics) | [x] |
| 14 | `find_char_in_buffer` | A5: `target` high-bit-set `0x80..=0xFF`; C sign-extends `char`→`int`, Rust zero-extends, `memchr` masks to `unsigned char` — all 128 values checked | [x] |
| 15 | `find_char_in_buffer` | A5 randomized: random buffers 0..=256 B of random bytes × random target × random `size` in `0..=len` (duplicate occurrences ⇒ first-match position matters) | [x] |
| 16 | `reset_counter` | A2: fresh state, randomized values incl. `INT_MIN`/`INT_MAX` | [x] |
| 17 | `increment_counter` | A2+A4: repeated calls accumulating from a known base; randomized values driving `+=` past `INT_MAX` (wrap) | [x] |
| 18 | `decrement_counter` | A2+A4: repeated calls; randomized values driving `-=` past `INT_MIN` (wrap) | [x] |
| 19 | `multiply_counter` | A2+A4: `*=` by `0`, `1`, `-1`, and randomized large values that overflow `int` | [x] |
| 20 | counter fns (all 4) | A2: randomized **interleaved sequences** of 64 ops from the full 4-op alphabet with random operands — the composed pipeline, not per-function calls | [x] |
| 21 | counter fns + `charinbuf` | A2 interaction: dirty the counter directly, then call `charinbuf(3, …)` (which zeroes it first), then read the counter back via `increment_counter(0)` | [x] |
| 22 | `apply_operation` | A7: each of the 4 counter callbacks from the *same* `.so`, randomized `value`, from a normalized counter | [x] |
| 23 | `apply_operation` | A7: callback defined in the test binary (external to both `.so`s), randomized `value` — pins the `int(*)(int)` ABI | [x] |
| 24 | `charinbuf` | A1 mode 0 × A3: `value` at every edge of `[0, 65535]` and outside it | [x] |
| 25 | `charinbuf` | A1 mode 0 × A3 randomized `value` over the full `int` domain; `opt1`/`opt2` randomized (must be ignored) | [x] |
| 26 | `charinbuf` | A1 mode 1: fixed-string path; `value`/`opt1`/`opt2` randomized (must all be ignored) | [x] |
| 27 | `charinbuf` | A1 mode 2: `malloc`/`strcpy`/`strlen`/`free` path, incl. the `%zu` `size_t` format; params randomized (ignored) | [x] |
| 28 | `charinbuf` | A1 mode 3 × A4: `value`/`opt1`/`opt2` small, non-overflowing — the plain `reset`→`+=`→`*=`→`-=5` chain | [x] |
| 29 | `charinbuf` | A1 mode 3 × A4: operands chosen to overflow at `+=` and at `*=` (`INT_MAX`, `INT_MIN`, `0`, `-1`, large primes) — two's-complement wrap must match | [x] |
| 30 | `charinbuf` | A1 mode 3 × A4 randomized over the full `int` domain for all three params (256 triples) | [x] |
| 31 | `charinbuf` | A1 mode 3 × A2: called twice in a row, to prove the entry-point `counter = 0` makes the second call independent of the first | [x] |
| 32 | `charinbuf` | A1 mode 4: `memchr` path where the target `'X'` **is** present; asserts the printed position and the returned index | [x] |
| 33 | `charinbuf` | A1 `default` × randomized `mode` outside `0..=4` (incl. `INT_MIN`, `INT_MAX`, `5`, `-1`) | [x] |
| 34 | `charinbuf` | A1 exhaustive sweep of `mode` over `-8..=12` × several `value`/`opt1`/`opt2` triples (cross-product, so mode/param interactions cannot hide) | [x] |
| 35 | all 10 symbols | A1+A2 randomized **whole-library** sequence: 512 random calls picked uniformly from all ten entry points with random arguments, replayed identically against both `.so`s — catches cross-function state leakage the per-function rows cannot | [x] |

## Status

All 35 rows pass. Each row is one `cfg_NN_*` test in
`tests/phase_b_configs.rs`, plus `cfg_low_level_functions_write_nothing_to_stdout`
which pins the assumption the fast comparison helpers rely on (none of the nine
lower-level C functions writes to `stdout`).

Verified in **four** invocations — debug and release, each with
`--no-default-features` and with the default feature set — 66 tests each,
via `./verify.sh`:

```
== 5. Phase B + C + D differential tests, every configuration
  ok    tests <no features>          (66 passed)
  ok    tests <default>              (66 passed)
  ok    tests release <no features>  (66 passed)
  ok    tests release <default>      (66 passed)
```

The suite was validated against three deliberately injected regressions to
confirm it is not passing vacuously:

| injected bug | rows that caught it |
|--------------|---------------------|
| `UINT16_MAX` 65535 → 65534 | 7 tests (1, 2, 21, 24, 25, 34, 35) |
| `wrapping_add` → `saturating_add` in `increment_counter` | 9 tests (17, 20, 21, 22, 29, 30, 31, 34, 35) |
| mode-4 `memchr` offset off by one | 3 tests (32, 34, 35) |

A fourth, more dangerous failure mode was found and fixed during this work:
`cargo test` does **not** rebuild a `cdylib` (no Rust target can depend on one),
so a source edit was invisible to the test run and every test passed against a
**stale** `.so`. `tests/common/mod.rs` now compares the `.so` mtime against the
newest source mtime and fails loudly with `STALE SHARED OBJECT`, and
`verify.sh` always runs `cargo build` before `cargo test`.
