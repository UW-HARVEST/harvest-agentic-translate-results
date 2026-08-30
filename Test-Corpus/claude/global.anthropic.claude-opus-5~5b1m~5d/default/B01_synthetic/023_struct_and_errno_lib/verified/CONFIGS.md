# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from `c_src/include/driver.h` (the public header) and the
branch structure of `c_src/src/driver.c`.

## Public entry points (complete set)

`nm -D --defined-only` on the C `.so` yields exactly two global functions, and
both are tested directly through the `.so` exports:

| entry point | signature | level |
|-------------|-----------|-------|
| `driver` | `void driver(const char *in)` | high-level one-shot wrapper (declared in `driver.h`) |
| `run`    | `void run(house_t *the_house, int extra_bedrooms)` | **low-level** entry point — exported (`T`) but *not* declared in the public header. Must be driven directly, since `driver` can only ever reach it with `floors == 2, bedrooms == 5, bathrooms == 2.5` on the first call. |

`add_floor`, `add_bedrooms`, `print_house`, `parse_val` are `static` (`t`, not
exported) and are reachable only through the two entry points above.

## Axes the C actually branches on

There are **no** runtime option/mode/flag setters, no global state, no
`#ifdef` configuration blocks, and no `[features]` in `Cargo.toml`. The
configuration surface is therefore entirely the **shape of the input data**:

* **A1 — `parse_val` outcome** (`driver.c:64`): the 4-conjunct condition
  `endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX`. Accepting
  branch → 8 `print_house` lines; rejecting branch → `ERRORS.md`.
* **A2 — `strtol` lexical shape** of the input string (base 10, fixed):
  leading whitespace / sign / digit run / trailing unconsumed suffix /
  embedded NUL / string length (0, 1, many).
* **A3 — parsed magnitude** of `extra_bedrooms`: `0`, small ±, `INT_MAX`,
  `INT_MIN`, and values whose accumulation across the calls wraps `int`.
* **A4 — `house_t.floors` (`int`)**: `0`, ±small, `INT_MIN`, `INT_MAX`
  (`floors++` at `INT_MAX` wraps — `%d` width also changes).
* **A5 — `house_t.bedrooms` (`int`)**: `0`, ±small, `INT_MIN`, `INT_MAX`
  (`bedrooms += extra_bedrooms` wraps).
* **A6 — `house_t.bathrooms` (`double`)** — the widest axis, because it feeds
  both an arithmetic op (`+= 1.0`) and `%.1f` formatting, which
  special-cases: normal values, negative zero, half-way rounding ties
  (round-half-to-even in glibc), subnormals, values so large that `+= 1.0` is a
  no-op, `±inf`, and NaN (incl. sign and payload).
* **A7 — call multiplicity / state accumulation**: `driver` invokes `run`
  **twice** on the *same* `house_t`, so the second call starts from the mutated
  state. Driving `run` directly for 0, 1, 2, and many successive calls
  exercises the composed pipeline that per-wrapper tests would miss.
* **A8 — observable channels**: stdout bytes **and** the caller-visible
  mutation of `*the_house` (checked field-by-field, `bathrooms` compared by
  raw IEEE-754 bit pattern so `-0.0` and NaN payloads cannot hide).

## Configuration table

Every row is exercised with **many randomized inputs** (deterministic
`splitmix64`, fixed seed `0x5eed_1234_dead_beef`) unless the row is inherently
a single boundary value, in which case the row's *other* axes are randomized.
Every row compares C vs Rust via the `.so` exports byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C1 | `driver` | accepting path, bare small positive decimal (`"0"`, `"1"`, `"7"`, random `0..=9999`), length 1 and many | [x] |
| C2 | `driver` | accepting path, explicit `+` sign (`"+0"`, `"+42"`, random `+N`) | [x] |
| C3 | `driver` | accepting path, `-` sign (`"-0"`, `"-1"`, random `-N`) | [x] |
| C4 | `driver` | accepting path, every leading-whitespace class `strtol` skips (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) singly and in random mixtures, then sign+digits | [x] |
| C5 | `driver` | accepting path, leading zeros: `"0000000042"`, 64 zeros + digits, `"-0000001"` | [x] |
| C6 | `driver` | accepting path with **trailing garbage** (never checked by C): `"12abc"`, `"5 "`, `"5\n"`, `"7,9"`, random digits + random suffix | [x] |
| C7 | `driver` | accepting path, hex-looking input under base 10: `"0x10"`, `"0X1f"`, `"0b1"` → parses `0`, `endp` advanced by 1 | [x] |
| C8 | `driver` | accepting path, decimal-point / exponent forms truncated at the `.`/`e`: `"7.9"`, `"-3.5"`, `"2e5"`, `"1_000"` | [x] |
| C9 | `driver` | accepting path, **embedded NUL** in the buffer (`"5\0abc"`) — `strtol` stops at the terminator | [x] |
| C10 | `driver` | accepting path, exact valid boundaries `INT_MAX` (`"2147483647"`) and `INT_MIN` (`"-2147483648"`) | [x] |
| C11 | `driver` | accepting path, magnitudes that make `bedrooms` wrap across the two internal `run` calls: `x` near `±INT_MAX/2`, `±INT_MAX`, `INT_MIN` | [x] |
| C12 | `driver` | accepting path, randomized full-`i32`-range decimal strings (1000 iterations, fixed seed) | [x] |
| C13 | `driver` | randomized composite strings: random whitespace prefix × random sign × random full-range magnitude × random trailing suffix (1000 iterations) | [x] |
| C14 | `run` (low-level, direct) | `floors=2, bedrooms=5, bathrooms=2.5` (the state `driver` uses) × `extra_bedrooms ∈ {0, 1, -1}` | [x] |
| C15 | `run` (low-level, direct) | `floors = INT_MAX` → `floors++` **wraps** to `INT_MIN`; also `floors = INT_MIN`, `-1`, `0` | [x] |
| C16 | `run` (low-level, direct) | `bedrooms = INT_MAX` × `extra_bedrooms > 0` (wrap up) and `bedrooms = INT_MIN` × `extra_bedrooms < 0` (wrap down) | [x] |
| C17 | `run` (low-level, direct) | `extra_bedrooms ∈ {INT_MIN, INT_MAX}` — the "arbitrary int with no valid variant" case across the FFI boundary | [x] |
| C18 | `run` (low-level, direct) | negative `floors` and `bedrooms` (`%d` sign width) × random `extra_bedrooms` | [x] |
| C19 | `run` (low-level, direct) | `bathrooms = 0.0` | [x] |
| C20 | `run` (low-level, direct) | `bathrooms = -0.0` (sign must survive `%.1f` and `+= 1.0`) | [x] |
| C21 | `run` (low-level, direct) | `bathrooms` = `%.1f` **rounding ties**: `0.05, 0.15, 0.25, 0.35, 2.45, -0.25, -1.05, 0.949999…, 0.95` (glibc round-half-to-even on the exact binary value) | [x] |
| C22 | `run` (low-level, direct) | `bathrooms` = **subnormals**: `5e-324` (min subnormal), `f64::MIN_POSITIVE`, `-5e-324` | [x] |
| C23 | `run` (low-level, direct) | `bathrooms` = huge, where `+= 1.0` is a no-op: `1e300`, `f64::MAX`, `2^53`, `2^53+1`, and negatives thereof (≈310-digit `%.1f` output) | [x] |
| C24 | `run` (low-level, direct) | `bathrooms` = `+inf`, `-inf` (`+= 1.0` keeps inf; `%.1f` → `inf`/`-inf`) | [x] |
| C25 | `run` (low-level, direct) | `bathrooms` = NaN: default quiet NaN, negative quiet NaN, NaN with a custom payload, signalling-NaN bit pattern (payload/sign propagation through `+= 1.0` and `%.1f`) | [x] |
| C26 | `run` (low-level, direct) | fully randomized `house_t`: `floors`/`bedrooms` uniform over `i32`, `bathrooms` from a uniform random **raw u64 bit pattern** (hits normals, subnormals, inf, NaN, ±0), × random `extra_bedrooms` (2000 iterations) | [x] |
| C27 | `run` (low-level, direct) | fully randomized `house_t` with `bathrooms` drawn from *finite* random exponents (`10^{-320..308}` × random mantissa) × random `extra_bedrooms` (1000 iterations) | [x] |
| C28 | `run` (low-level, direct) | **state accumulation / composed pipeline**: the *same* `house_t` fed through `run` 1, 2, 3 … 16 times in a row with a per-call random `extra_bedrooms`, comparing stdout and the struct after **every** call (mirrors and extends `driver`'s double invocation) | [x] |
| C29 | `driver` + `run` | **interleaved** sequence: random `driver` calls and random `run` calls in one process, in the same order for C and Rust, to catch hidden global/`errno` state coupling between the two entry points | [x] |
| C30 | `driver` | input length axis: `""`(→error row E1), `"1"` (1 byte), 2 bytes, 4096-byte digit string of a valid magnitude (leading zeros), 4096-byte suffix after a valid digit | [x] |

## Feature combinations

`Cargo.toml` has no `[features]` section → the only combination is the default
(identical to `--no-default-features`). Verified by `check_features.sh`, which
enumerates features from `Cargo.toml` and runs the full suite for every
combination it finds (plus the empty one).
