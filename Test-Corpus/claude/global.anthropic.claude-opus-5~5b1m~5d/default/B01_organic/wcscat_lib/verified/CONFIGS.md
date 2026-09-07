# CONFIGS.md — Phase B configuration-surface table

## Axes derived from the C source

`c_src/src/lib.c` exposes exactly one public entry point, `wcscat`, and it is
also the lowest-level entry point — there is no convenience wrapper layer, no
internal helper with separate semantics, and no one-shot vs. streaming split. So
the "full set of public entry points" is `{ wcscat }`.

There are **no runtime options, modes or flags**: no global state, no
`set_*`/`init` function, no `#ifdef` in `lib.c` or `lib.h`, and no Cargo
features in `translation/Cargo.toml`. Consequently the configuration surface is
driven entirely by **input shape**. The shapes the C code actually branches on:

**Axis 1 — position `k` of the first NUL inside the `dst` window**
(this is what the first `while` loop computes: `while (ptr < dst+numElem && *ptr != 0) ptr++`)
- `k == 0` — destination is an empty string (pure copy)
- `0 < k < numElem-1` — destination has a prefix (true append)
- `k == numElem-1` — NUL sits on the last window element (zero room left for src)
- no NUL in window — first loop saturates at `numElem` (error shape, see ERRORS.md E7)

**Axis 2 — `strlen(src)` relative to the room left, `room = numElem - k`**
(this is what the second `while` loop consumes)
- `strlen(src) == 0` — empty source, copies only the terminator
- `0 < strlen(src) < room - 1` — fits with slack
- `strlen(src) == room - 1` — **exact fit**, terminator lands on the very last window element
- `strlen(src) >= room` — overflow (error shape, ERRORS.md E9/E11)

**Axis 3 — `numElem` magnitude**
- `1` (minimum accepted value; `0` is rejected, ERRORS.md E4)
- small (2..8, where boundary arithmetic is dense)
- typical (tens)
- oversized relative to the string contents (`1 << 40`, ERRORS.md G3)

**Axis 4 — `numElem` versus the real allocation of `dst`**
- `numElem == capacity` (the normal contract)
- `numElem < capacity` (a short window inside a larger buffer — proves the C
  never touches `dst[numElem..]`, and that a NUL beyond the window is invisible
  to the first loop)

**Axis 5 — element value domain (`wchar_t` is signed 4-byte `i32` here)**
- plain ASCII (`1..=127`)
- values above the BMP / `> 0xFFFF` (e.g. `0x10FFFF`) — would truncate under a
  wrong 16-bit `wchar_t` mapping
- high-bit-set / negative values (`i32::MIN`, `-1`, `0x8000_0000` as `i32`) —
  would compare wrongly under an unsigned mapping
- `i32::MAX`, and dense random draws over the whole `i32` domain excluding `0`

**Axis 6 — residual bytes in `dst` beyond the NUL**
- zero-filled tail vs. non-zero garbage tail. The C copies over the tail but
  never zero-pads, so a garbage tail makes any spurious padding in the Rust
  immediately visible.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed, a
xorshift-based deterministic PRNG in the test file) rather than one hand-picked
value, and asserts the return code **and the entire `dst` buffer including the
bytes past `numElem`** match byte-for-byte between the C and Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C1 | `wcscat` | `k == 0` (empty dst), `strlen(src) == 0`, `numElem == 1`, `numElem == capacity` — minimal accepted call | [x] |
| C2 | `wcscat` | `k == 0`, `strlen(src) == 0`, random `numElem` 1..64, zero-filled tail | [x] |
| C3 | `wcscat` | `k == 0`, `strlen(src)` fits with slack, random `numElem` 2..64, ASCII src | [x] |
| C4 | `wcscat` | `k == 0`, `strlen(src) == numElem - 1` — **exact fit**, random `numElem` | [x] |
| C5 | `wcscat` | `0 < k < numElem-1` (true append), `strlen(src)` fits with slack, random `k`/`numElem`, ASCII | [x] |
| C6 | `wcscat` | `0 < k < numElem-1`, `strlen(src) == numElem - k - 1` — **exact fit on append** | [x] |
| C7 | `wcscat` | `k == numElem - 1` (NUL on last window element), `strlen(src) == 0` — the only src that still fits | [x] |
| C8 | `wcscat` | `numElem == 1` crossed with `dst[0] ∈ {0, nonzero}` × `src` empty / non-empty (full 2×2 of the degenerate window) | [x] |
| C9 | `wcscat` | `numElem` small (2..8), full cross-product of `k ∈ 0..numElem` × `strlen(src) ∈ 0..numElem+2` — dense exhaustive boundary sweep, no randomization gaps | [x] |
| C10 | `wcscat` | `numElem < capacity` — short window inside a larger buffer, with **non-zero garbage** in `dst[numElem..capacity]`; asserts the tail is untouched | [x] |
| C11 | `wcscat` | `numElem < capacity` where a NUL exists only *beyond* the window (so the first loop saturates) — proves the out-of-window NUL is invisible | [x] |
| C12 | `wcscat` | valid append with `src` containing values `> 0xFFFF` (`0x10FFFF`, `0x1_0000`) — 16-bit-`wchar_t` regression guard | [x] |
| C13 | `wcscat` | valid append with **negative / high-bit-set** `wchar_t` in `src` (`-1`, `i32::MIN`, `i32::MAX`) — signedness guard | [x] |
| C14 | `wcscat` | valid append where `dst`'s prefix contains negative / high-bit-set values, so the first loop must skip them as non-NUL | [x] |
| C15 | `wcscat` | valid append with a **non-zero garbage tail** after the NUL in `dst`, `src` fits with slack — proves no zero-padding is added past the copied terminator | [x] |
| C16 | `wcscat` | oversized `numElem` (`1 << 40`) with `k == 0` in a small real buffer and a short `src` — nominal window vastly exceeds the allocation but the NUL stops the copy | [x] |
| C17 | `wcscat` | fully randomized fuzz over all axes at once: random `capacity`, random `numElem <= capacity`, random NUL position (or none), random `strlen(src)` (fitting or overflowing), random `i32` element values over the whole domain | [x] |

## Final status

All 17 configuration rows (C1–C17, plus C17b) have a passing differential test
in `tests/phase_b_configs.rs` (18 tests), each driven with many randomized
inputs from a fixed-seed PRNG, plus one fully exhaustive sweep (C9 covers every
`(numElem, k, strlen(src), value-domain, tail)` combination for `numElem` 1..8)
and one 8000-iteration all-axis fuzz (C17).

Phase B: **PASS** — 18/18 tests, 0 unchecked rows.

### No binary target

Neither project builds an executable driver: `c_src/CMakeLists.txt` contains
only `add_library(... SHARED ...)` (no `add_executable`), and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` with no
`[[bin]]` and no `src/main.rs`. The "compare C and Rust stdout" gate is therefore
**N/A**.
