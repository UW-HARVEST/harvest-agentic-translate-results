# CONFIGS.md — configuration / valid-input surface table (Phase B gate)

Derived mechanically from `c_src/include/driver.h` + `c_src/src/driver.c`.

## Axes the C actually branches on

**Runtime options / modes / flags: none.**
`grep -n "#if\|#else\|switch\|enum" c_src/src/driver.c c_src/include/driver.h`
finds only the `#ifndef DRIVER_H_` include guard. There is no options struct,
no setter, no global flag, no environment lookup. The only "mode" the library
has is its **implicit persistent state**, the file-scope `static house_t
the_house` — which *is* a configuration axis, because every entry point's
output depends on the values left behind by all previous calls.

**Public entry points (the full set, lowest-level included):**

| entry point | declared in header? | linkage | level |
|-------------|--------------------|---------|-------|
| `run(int extra_bedrooms)` | no | external (`T run`) | **low-level** — mutates `the_house` and prints 4 lines directly, bypassing all parsing |
| `driver(const char *in)` | yes | external (`T driver`) | convenience wrapper — parses, then calls `run` twice |

Both are driven directly through their `.so` exports; `run` is *not* only
exercised via `driver`.

**Input-shape axes**

* `driver`'s `const char *in` (consumed by `strtol(str, &endp, 10)`):
  leading whitespace · sign (`none` / `+` / `-`) · digit count (0, 1, many,
  leading zeros) · trailing garbage after the digits · non-digit prefix ·
  base-10-only interpretation of `0x…`/`0…` · magnitude class
  (0, ±small, `INT_MAX`, `INT_MIN`, one past each, `LONG_MAX`/`LONG_MIN`,
  past `LONG` range) · embedded `NUL` · very long buffers.
* `run`'s `int extra_bedrooms`: 0 · ±1 · ±small · `INT_MAX` · `INT_MIN` ·
  values that make `bedrooms += extra_bedrooms` wrap.
* Accumulated `the_house` state: fresh · after many `run`s (floors large,
  bathrooms large so `%.1f` prints a wide field) · `bedrooms` just wrapped
  negative · `bedrooms` at exactly `INT_MAX`/`INT_MIN`.
* Call-sequence shape: single call · many calls · `driver`-only ·
  `run`-only · `driver`/`run` interleaved (state hand-off between the two
  entry points).

`floors` overflow is *not* reachable — it needs 2^31 `run` calls — so it is
excluded rather than guessed at; `bedrooms` overflow is reachable in one call
and is covered.

Every row is driven with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, deterministic SplitMix64 PRNG in
`tests/common/mod.rs`), and both libraries are called through their `.so`
exports with the byte-for-byte stdout captured and compared.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `run` | `extra_bedrooms = 0` — the identity case; isolates floors/bathrooms mutation from bedrooms mutation. 32 repetitions so the accumulated state advances. | [x] |
| 2 | `run` | `extra_bedrooms` = uniformly random `i32` over the full range, 256 values. Exercises `bedrooms` wrap-around in both directions and the whole `%d` formatting range. | [x] |
| 3 | `run` | `extra_bedrooms` = small positive (1..=1000), 128 values — the "normal" usage shape, keeps `bedrooms` positive and growing. | [x] |
| 4 | `run` | `extra_bedrooms` = small negative (-1000..=-1), 128 values — drives `bedrooms` negative, checking `%d` of negative ints. | [x] |
| 5 | `run` | `extra_bedrooms` ∈ boundary set {`0`, `1`, `-1`, `2`, `-2`, `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`, `65535`, `65536`, `-65536`} — every documented/implicit boundary value, each applied twice (second application lands on already-wrapped state). | [x] |
| 6 | `run` | `extra_bedrooms = INT_MAX` applied repeatedly (16×) so `bedrooms` wraps every call — signed-overflow wrap parity. | [x] |
| 7 | `run` | deep accumulated state: 512 consecutive `run` calls with random values, so `floors` grows past 1000 and `bathrooms` past 500.0, changing the printed field widths of `%d` and `%.1f`. | [x] |
| 8 | `driver` | valid decimal, no sign, random magnitude in `0..=INT_MAX`, 256 values — the main happy path (two `run` calls per invocation). | [x] |
| 9 | `driver` | valid decimal with explicit `-`, random magnitude in `INT_MIN..=-1`, 256 values. | [x] |
| 10 | `driver` | valid decimal with explicit `+` prefix, random magnitude `0..=INT_MAX`, 128 values — `strtol` accepts `+`. | [x] |
| 11 | `driver` | leading whitespace (random mix of spaces, `\t`, `\n`, `\v`, `\f`, `\r`, length 1..8) before an optionally signed random value, 256 cases — `strtol` skips `isspace`. | [x] |
| 12 | `driver` | leading zeros: random value rendered with 1..20 leading `0`s, 128 cases — must stay base 10, no octal reinterpretation. | [x] |
| 13 | `driver` | trailing garbage: random valid value followed by a random non-digit suffix (`"12abc"`, `"7 8"`, `"3.9"`, `"5,"`, …), 256 cases — the C only tests `endp != str`, so this **must be accepted** and the prefix used. | [x] |
| 14 | `driver` | `0x`/`0X` hex-looking input (`"0x1f"`, `"0X0"`, random hex digits) — with base 10 `strtol` stops after the leading `0`, so the value is `0` and the input is **accepted**. 64 cases. | [x] |
| 15 | `driver` | boundary magnitudes exactly at the accepted limits: `"0"`, `"-0"`, `"+0"`, `"1"`, `"-1"`, `"2147483647"` (`INT_MAX`), `"-2147483648"` (`INT_MIN`), `"2147483646"`, `"-2147483647"`, each also with whitespace and `+` decorations. | [x] |
| 16 | `driver` | embedded `NUL`: `"123\0456"` passed as a pointer — parsing stops at the `NUL`, value `123`. 8 cases. | [x] |
| 17 | `driver` | long-but-valid buffers: random valid value padded with 1..4000 leading zeros and/or whitespace (total length up to ~4 KiB) — no length limit exists, must still parse. 32 cases. | [x] |
| 18 | `driver` + `run` interleaved | random alternation of `driver("<random>")` and `run(<random>)`, 256 steps — state hand-off between the convenience wrapper and the low-level entry point, the composed pipeline no per-function test can see. | [x] |
| 19 | `driver` + `run` interleaved | alternation where the `driver` inputs are a random mix of **valid and invalid** strings (so `run` is sometimes called twice and sometimes not at all), 256 steps — verifies the *state divergence* an incorrect error path would cause is detected. | [x] |
| 20 | `run` | `extra_bedrooms` chosen so that `bedrooms` lands exactly on `INT_MAX`, then `INT_MIN`, then `0` (computed from the live state read back out of the printed line), 3 targeted steps × 4 rounds — exact-boundary state configuration. | [x] |
| 21 | `driver` | repeated identical valid input (`"7"`) 64× — idempotence of the *input* against non-idempotence of the *state*; catches a translation that reset `the_house`. | [x] |
| 22 | `driver` | the full mixed corpus: 1024 randomly-generated strings drawn from a generator that mixes every shape above (valid, invalid, whitespace, signs, garbage, huge, empty) — property-style fuzz over the whole entry point. | [x] |

All 22 rows are implemented in `translation/tests/configs.rs` and pass; see
`VERIFICATION.md` for the run log.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the feature power set is
`{ default }` = `{ ∅ }`. `scripts/check_features.sh` derives this mechanically
from `Cargo.toml` and re-runs the full suite for the default build and for
`--no-default-features`; both are the same code path, and both pass.
