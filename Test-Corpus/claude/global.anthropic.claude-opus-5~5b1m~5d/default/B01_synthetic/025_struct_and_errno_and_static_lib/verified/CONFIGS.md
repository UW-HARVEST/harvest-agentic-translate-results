# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C source actually takes.

## The axes the C code distinguishes

**Axis 1 — entry point.** The `.so` exports exactly two functions
(see `SYMBOLS.md`):

* `run(int extra_bedrooms)` — the **low-level** entry point. Not in
  `driver.h`, but externally linked and therefore part of the public ABI. It
  must be driven directly, not only through `driver`.
* `driver(const char *in)` — the convenience one-shot wrapper. It parses,
  then calls `run` **twice**.

**Axis 2 — runtime options / modes.** `driver.c` declares **no** flags, no
option struct, no setter, no `#ifdef`, and no `switch`. The complete set of
`if` statements in the file is:

```
70:  if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX)   (parse_val)
80:  if (parse_val(in, &x))                                              (driver)
```

So the only "mode" the library has is **implicit, persistent global state**.

**Axis 3 — persistent global state (the real hidden configuration).**
`static house_t the_house = {2, 5, 2.5};` lives for the lifetime of the
process and *every* call mutates it:

| per `run()` call | effect |
|---|---|
| `floors`    | `+1` (via `add_floor_to_the_house` → `add_floor`) |
| `bathrooms` | `+1.0` |
| `bedrooms`  | `+= extra_bedrooms` |

⇒ the output of call *n* depends on all *n−1* preceding calls. "Fresh state",
"state after a few calls" and "state after many calls" are genuinely different
configurations, as are the *interleavings* of `run` and `driver`.

**Axis 4 — input shape of `extra_bedrooms` (`int`).** zero / small positive /
small negative / `INT_MAX` / `INT_MIN` / arbitrary 32-bit patterns (drives the
wraparound in `add_bedrooms` and the `%d` sign in `printf`).

**Axis 5 — input shape of the `driver` string.** Every form `strtol(str, &e, 10)`
treats differently: plain digits, leading whitespace (space/`\t\n\v\f\r`),
explicit `+`/`-` sign, leading zeros, `-0`, trailing non-numeric garbage,
boundary values `INT_MAX`/`INT_MIN`, `long`-range-but-not-`int` values,
`ERANGE`-triggering huge values, empty, and non-numeric. (Rejecting shapes are
in `ERRORS.md`; the accepting shapes are the rows below.)

**Axis 6 — output-formatting shape.** `printf("… %d … %d … %.1f …")`:
`%d` with positive / negative / wrapped values, and `%.1f` of a `double` that
is always `k + 0.5` (exactly representable, so no banker's-rounding ambiguity)
and grows without bound as state accumulates.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, splitmix64 PRNG) unless the row is a fixed boundary.
Both `.so`s are driven in lock-step through their exported symbols and stdout
is captured per call and compared byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `run` | fresh global state, `extra_bedrooms == 0` (identity add; isolates the `floors`/`bathrooms` mutation) | [x] |
| 2  | `run` | small positive `extra_bedrooms` (1..=1000), randomized | [x] |
| 3  | `run` | small negative `extra_bedrooms` (-1000..=-1), randomized — drives `%d` with a negative `bedrooms` | [x] |
| 4  | `run` | full-range random `int` (uniform over all 2^32 patterns), randomized — drives `add_bedrooms` wraparound at random | [x] |
| 5  | `run` | boundary `extra_bedrooms`: `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `-1`, `1`, `0` | [x] |
| 6  | `run` | **state accumulation**: 300 consecutive `run` calls with randomized args, so `floors`/`bathrooms` climb and `bedrooms` random-walks/wraps; every one of the 4 lines of every call compared | [x] |
| 7  | `run` | **deep state + `%.1f` growth**: 500 further calls so `bathrooms` reaches ≥ 800.5, checking the `%.1f` formatting of a large half-integer `double` | [x] |
| 8  | `run` | repeated `INT_MAX` additions (10×) forcing `bedrooms` to wrap around several times in a row | [x] |
| 9  | `driver` | plain decimal digits, randomized `int` values rendered with `to_string()` | [x] |
| 10 | `driver` | randomized value **+ leading whitespace** (random mix of `' '`, `\t`, `\n`, `\v`, `\f`, `\r`) | [x] |
| 11 | `driver` | randomized non-negative value **+ explicit `+` sign** | [x] |
| 12 | `driver` | randomized value **+ leading zeros** (1..=8 zeros after the optional sign) | [x] |
| 13 | `driver` | randomized value **+ trailing garbage** (`abc`, ` 43`, `.5`, `e3`, `,000`, `x1A`, `%`, `\n`) — `strtol` stops early, `endp != str`, so accepted | [x] |
| 14 | `driver` | **all decorations combined**: whitespace + sign + leading zeros + trailing garbage (cross-product, randomized) | [x] |
| 15 | `driver` | boundary strings that must be **accepted**: `"2147483647"`, `"-2147483648"`, `"0"`, `"-0"`, `"+0"`, `"1"`, `"-1"` | [x] |
| 16 | `driver` | `"0x1A"`, `"0b11"`, `"0o17"`, `"08"`, `"09"` — base is hard-coded to 10, so only the leading `0`/digits convert | [x] |
| 17 | `driver` | very long but valid input: value padded with 4096 leading zeros, and 1 MiB of trailing garbage | [x] |
| 18 | `driver` | rejected input **followed by** a valid input — confirms the rejected call left `the_house` untouched in *both* implementations (state-divergence detector) | [x] |
| 19 | `run` + `driver` | **randomized interleaving** of 200 `run` / `driver` / rejecting-`driver` calls against the same accumulating global state — the composed pipeline, invisible to per-function tests | [x] |
| 20 | `driver` | pre-existing non-zero `errno` in the caller (set via a failed syscall) before each call, across accepting *and* rejecting inputs — line 67 must neutralise it | [x] |
| 21 | `run` | called with the *same* argument twice in a row, mirroring `driver`'s `run(x); run(x);` pattern, but entered at the low level so the state offset differs from row 19 | [x] |
| 22 | `driver` | `driver` vs. two manual `run` calls: assert `driver(s)` output == `run(x); run(x)` output for the same parsed `x`, in **both** libraries (call-hierarchy equivalence) | [x] |
| 23 | `run` | **state deeper than `f32` can represent**: 8 388 608+ calls so `bathrooms` passes 2^23 (and then 2^24), stepping across the limit one call at a time. Below 2^23 every value `k + 0.5` is exact in `f32`, so a narrowed `bathrooms` vararg is invisible; above it, it is not. | [x] |
| 24 | `run` + `driver` | deep state reached with `extra_bedrooms = INT_MAX/3` (200 000 calls) so `bedrooms` wraps thousands of times, then accepting *and* rejecting `driver` calls at that depth | [x] |
| 25 | `run` + `driver` | **`LC_NUMERIC` with a comma decimal separator** (`de_DE.utf8`): `%.1f` must render `,5`. Catches formatting reimplemented in Rust rather than delegated to libc `printf`. | [x] |
| 26 | `run` | locale round-trip `C` / `C.utf8` / `POSIX` / `en_US.utf8` and back | [x] |
| 27 | `run` | **called from a second (and a chain of eight further) OS threads**: `the_house` is one object per *process*, so a `thread_local!` translation must be detected | [x] |
| 28 | `driver` | input string placed in an `mprotect(PROT_READ)` page, plus a byte-for-byte snapshot comparison of the buffer afterwards — `parse_val` casts the `const` away and must never write through it | [x] |

## Row → test mapping

| rows | test |
|------|------|
| 1–22 | `tests/phase_b_configs.rs` (`row01_*` … `row22_*`) |
| 23 | `tests/phase_b_deep_state.rs::deep_state_bathrooms_past_f32_precision` |
| 24 | `tests/phase_b_deep_state.rs::deep_state_via_driver_and_wrapping_bedrooms` |
| 25 | `tests/phase_b_env_axes.rs::locale_lc_numeric_comma_decimal_separator` |
| 26 | `tests/phase_b_env_axes.rs::locale_round_trip_c_and_back` |
| 27 | `tests/phase_b_env_axes.rs::global_state_is_process_global_not_thread_local` |
| 28 | `tests/phase_b_env_axes.rs::input_is_readonly_and_unmodified` |

Rows 23, 25, 27 and 28 were added *because* mutation testing
(`mutation_check.py`) proved rows 1–22 could not distinguish an
`f32`-narrowed `bathrooms`, a Rust-side reimplementation of `printf`,
`thread_local!` global state, or a write through the `const` input.

## Deliberately excluded

* **Concurrency.** `the_house` is a plain non-atomic global; concurrent calls
  are a data race in the C with no defined output ordering, so no
  byte-for-byte differential assertion exists. Tests therefore serialise all
  calls under a mutex (see `tests/common/mod.rs`).
* **`floors` overflow.** Requires 2^31 `run` calls.
