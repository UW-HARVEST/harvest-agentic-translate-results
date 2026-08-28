# CONFIGS.md — Configuration-surface table (Phase A)

Mirror of `ERRORS.md` for **valid** inputs. Rows are the pruned cross-product of
the axes the C source *actually* branches on.

## Axes derived from `c_src/src/lib.c`

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| **entry point** | `gotomach` (composed pipeline) · `process_value` · `double_value` · `triple_value` (the lowest-level exports, callable directly *and* used as the `operation_fn` callback) | `lib.h:1`, `lib.c:59/65/71/106` |
| **`mode`** (operation selector) | `0` → `process_value` · `1` → `double_value` · `2` → `triple_value` · anything else → `default:` = `process_value` + `[WARNING]` | `switch`, `lib.c:126-140` |
| **`iterations`** (= `capacity`, = loop trip count, = both `malloc` sizes) | `0` (empty / `malloc(0)`) · `1` (one) · `2..N` (many) · `65534` (one below saturation) · `65535` (`UINT16_MAX`, max valid, only value that can saturate `count`) | `lib.c:114,142,149,163` |
| **`seed`** (initial `current_value`) | `0` · `1` · `< 1000` (fold `% 1000` is identity for `process_value`) · `>= 1000` (first op output exceeds the fold range) · `65535` (max valid) | `lib.c:120,162` |
| **`threshold`** (append predicate `produced < threshold`) | `INT_MIN` / `<= 0` → append **none** · `INT_MAX` → append **all** · in-between → append **some** · exactly equal to a produced value → **not** appended (strict `<`) | `lib.c:172` |
| **`count` saturation** | `count < UINT16_MAX` (normal exit) · `count >= UINT16_MAX` (`[WARNING] Reached maximum count` + `break`, then *still* sums) | `lib.c:178-181` |
| **callback extra args** | `unused_param` (`0` from `gotomach`, arbitrary when called directly) · `unused_context` (`NULL` from `gotomach`, arbitrary when called directly) | `lib.c:60-61,170` |
| **observable channel** | return value · `stdout` log bytes (`LOG_MSG` → `printf("[LVL] msg\n")`, lowered to `puts`) | `lib.c:31` |

There are **no** `#ifdef`s, no compile-time options, no global/`static` mutable
state, and no runtime setters — the entire configuration surface is the four
`int` arguments plus the direct-call arguments of the three `operation_fn`s.

## Configuration rows

All rows are exercised with **many randomized inputs** (fixed seed
`0x5EED_1EAF_C0FFEE01`, xorshift64\* PRNG) unless the row pins a specific value.
Both `.so`s are loaded with `libloading`; the return value of every call is
compared byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `process_value` | direct call, `value` uniform over the whole `i32` range (incl. both overflow edges), `unused_param = 0`, `unused_context = NULL` | [x] |
| 2  | `double_value`  | direct call, `value` uniform over the whole `i32` range | [x] |
| 3  | `triple_value`  | direct call, `value` uniform over the whole `i32` range | [x] |
| 4  | `process_value`/`double_value`/`triple_value` | direct call, `value` random, **`unused_param` random `i32`** and **`unused_context` random non-null pointer** — proves args 2/3 are ignored identically | [x] |
| 5  | `process_value`/`double_value`/`triple_value` | direct call, `value` restricted to the range `gotomach` can actually feed them (`0..=65535` on iter 0, `0..=999` afterwards) | [x] |
| 6  | `gotomach` | `mode = 0`, `iterations = 0` (empty shape, `malloc(0)`), random `seed`/`threshold` | [x] |
| 7  | `gotomach` | `mode = 1`, `iterations = 0` | [x] |
| 8  | `gotomach` | `mode = 2`, `iterations = 0` | [x] |
| 9  | `gotomach` | `mode = default` (random out-of-range `int`), `iterations = 0` | [x] |
| 10 | `gotomach` | `mode = 0`, `iterations = 1` (one shape), random `seed ∈ 0..=65535`, random `threshold` | [x] |
| 11 | `gotomach` | `mode = 1`, `iterations = 1` | [x] |
| 12 | `gotomach` | `mode = 2`, `iterations = 1` | [x] |
| 13 | `gotomach` | `mode = default`, `iterations = 1` | [x] |
| 14 | `gotomach` | `mode = 0`, `iterations ∈ 2..=64` (many shape), random `seed`, random `threshold` | [x] |
| 15 | `gotomach` | `mode = 1`, `iterations ∈ 2..=64` | [x] |
| 16 | `gotomach` | `mode = 2`, `iterations ∈ 2..=64` | [x] |
| 17 | `gotomach` | `mode = default`, `iterations ∈ 2..=64` | [x] |
| 18 | `gotomach` | `mode = 0`, `threshold = INT_MIN` → append **none**, `count == 0`, random `iterations`/`seed` | [x] |
| 19 | `gotomach` | `mode = 1`, `threshold = INT_MIN` | [x] |
| 20 | `gotomach` | `mode = 2`, `threshold = INT_MIN` | [x] |
| 21 | `gotomach` | `mode = default`, `threshold = INT_MIN` | [x] |
| 22 | `gotomach` | `mode = 0`, `threshold = INT_MAX` → append **all**, `count == iterations`, random `iterations`/`seed` | [x] |
| 23 | `gotomach` | `mode = 1`, `threshold = INT_MAX` | [x] |
| 24 | `gotomach` | `mode = 2`, `threshold = INT_MAX` | [x] |
| 25 | `gotomach` | `mode = default`, `threshold = INT_MAX` | [x] |
| 26 | `gotomach` | **strict-`<` boundary**: `threshold` pinned to exactly a value the sequence produces (and ±1), all four modes, random `seed` | [x] |
| 27 | `gotomach` | `threshold` pinned into the *interesting* band `0..=3100` (where produced values live) → **partial** appends, all four modes, random `iterations ∈ 0..=300` | [x] |
| 28 | `gotomach` | `seed = 0` boundary (min valid), all four modes, random `iterations`/`threshold` | [x] |
| 29 | `gotomach` | `seed = 65535` boundary (`UINT16_MAX`, max valid), all four modes, random `iterations`/`threshold` | [x] |
| 30 | `gotomach` | `seed >= 1000` (so the first op output overflows the `% 1000` fold) vs `seed < 1000`, all four modes | [x] |
| 31 | `gotomach` | `iterations = 65535` (`UINT16_MAX`, max valid) + `threshold = INT_MAX` → **`count` saturation** path, `[WARNING] Reached maximum count`, all four modes, several seeds | [x] |
| 32 | `gotomach` | `iterations = 65534` (one below saturation) + `threshold = INT_MAX` → warning **not** emitted, all four modes | [x] |
| 33 | `gotomach` | `iterations = 65535` + `threshold` in the partial band → large `count` but no saturation, all four modes | [x] |
| 34 | `gotomach` | `iterations ∈ {65000..65535}` × `threshold ∈ {INT_MAX, 3000, 1000, 100, 0}` — the near-max × append-density cross-product | [x] |
| 35 | `gotomach` | **full random sweep**: all four arguments independently random over the whole `i32` range (mixes valid and invalid; 200 000 cases) | [x] |
| 36 | `gotomach` | **full random sweep, valid-only**: `iterations ∈ 0..=65535`, `seed ∈ 0..=65535`, `mode ∈ i32`, `threshold ∈ i32` (50 000 cases) | [x] |
| 37 | `gotomach` | **`stdout` byte comparison** for a representative config from every distinct log path: `[INFO]`×2, `[ERROR] Invalid iteration count`, `[ERROR] Invalid seed value`, `[WARNING] Invalid mode`, `[WARNING] Reached maximum count` | [x] |
| 38 | `gotomach` | **repeat invariance / no hidden state**: the same config called 50× in a row, and interleaved C/Rust calls, must give the identical value every time (the C has no globals; the Rust must not either) | [x] |
| 39 | `gotomach` + ops | **cross-implementation callback composition**: verify the Rust `gotomach` reproduces the value obtained by driving the *C* `process_value`/`double_value`/`triple_value` through the C algorithm by hand (independent oracle for the composed pipeline) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration. The default feature set is the only feature set:

```
$ grep -n 'feature' translation/Cargo.toml   # -> no match
```

`cargo test --no-default-features` and `cargo test --all-features` are therefore
identical to `cargo test`; all three are run by `run_all.sh` for completeness.

## Status

All 39 rows pass across their randomized inputs. See
`translation/tests/differential.rs`.
