# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

Derived **mechanically** from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually distinguishes

### 1. Public entry points (complete set)

`nm -D --defined-only` on the C `.so` yields exactly one:

| entry point | signature | notes |
|---|---|---|
| `driver` | `void driver(int floors)` | the only exported symbol |
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | **lowest-level** function; `static`, so unexported. Reachable only via `driver`, always with `p = &raw` and `len = sizeof(house_t) = 16`. Covered indirectly (rows C18–C20). |

### 2. Runtime options / modes / flags

**None.** Grep finds no globals, no setters, no `#ifdef`-selected behaviour (only
the header include guard), no `switch`, and no `if`. The library is a pure
function of its single argument plus the compile-time constants below.

### 3. Compile-time constants the code embeds (fixed, not selectable)

| constant | value | observable effect |
|---|---|---|
| `house.bedrooms` | `3` | bytes 4..8 of output are always `03000000` |
| `house.bathrooms` | `2.` (`double`) | bytes 8..16 are always `0000000000000040` (IEEE-754 `2.0`, little-endian) |
| `sizeof(house_t)` | 16 (`_Alignof` 8, offsets 0/4/8 — **no padding**, verified by probe) | output is always 32 hex digits + `\n` = 33 bytes |
| `"%02x"` format | — | every byte is 2 lowercase hex digits, zero-padded |

### 4. Input shapes the observable behaviour depends on

`driver` takes one `int`. It performs no branching on the value, **but** its
bytes are hex-printed, so the value determines the output. The shapes below are
the classes where a translation can realistically diverge (byte order,
signedness of `char raw[]`, `%02x` zero-padding, NUL truncation, sign extension,
overflow at the `int` boundaries).

## Configuration-surface table

Each row is a meaningful combination of *entry point × input shape × pipeline
shape*. Every row is asserted against the C `.so` with **many randomized inputs
drawn from that row's class** (fixed-seed SplitMix64, so runs are reproducible),
not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| C1 | `driver` | `floors = 0` — all four bytes zero; exercises `%02x` zero-padding on every byte | [x] |
| C2 | `driver` | `floors` random in `1..=15` — only the low nibble set; each byte needs a leading `0` from `%02x` | [x] |
| C3 | `driver` | `floors` random in `16..=127` — one significant byte, high bit clear | [x] |
| C4 | `driver` | `floors` random in `128..=255` — one significant byte with the **high bit set**; distinguishes `unsigned char` reinterpretation from signed `char` sign-extension | [x] |
| C5 | `driver` | `floors` random in `256..=65535` — two significant bytes; pins little-endian **byte order** | [x] |
| C6 | `driver` | `floors` random in `65536..=16777215` — three significant bytes; byte order + one zero high byte | [x] |
| C7 | `driver` | `floors` with four pairwise-distinct nonzero bytes (`0xAABBCCDD` shapes, randomized) — full byte-order discrimination | [x] |
| C8 | `driver` | `floors = -1` — all bytes `0xff` | [x] |
| C9 | `driver` | `floors` random in `-256..=-1` — small negatives; two's-complement high bytes all `0xff` | [x] |
| C10 | `driver` | `floors` random in `-65536..=-257` | [x] |
| C11 | `driver` | `floors = INT_MAX` (`0x7fffffff`) — positive boundary | [x] |
| C12 | `driver` | `floors = INT_MIN` (`0x80000000`) — negative boundary, sign bit only | [x] |
| C13 | `driver` | `floors` bit patterns with **embedded NUL bytes** (`0x00ff00ff`, `0xff00ff00`, `0x00000001`, `0x01000000`, randomized 2-of-4-zero-byte masks) — no C-string truncation | [x] |
| C14 | `driver` | `floors` bit patterns containing byte `0x0a` (`'\n'`) and `0x25` (`'%'`) at each of the 4 byte positions — payload bytes that could break output framing | [x] |
| C15 | `driver` | `floors` uniform over the **full `i32` range** (property test, N = 4000, fixed seed) — value-dependent catch-all | [x] |
| C16 | `driver` | `floors` = every value in `-512..=512` (exhaustive small-magnitude sweep, both signs, crossing zero) | [x] |
| C17 | `driver` | `floors` = each single-bit value `1 << k` for `k = 0..=31` (covers the sign bit and every byte lane boundary) | [x] |
| C18 | `driver` → `print_hex` | invariant tail: for arbitrary randomized `floors`, bytes 4..16 of the output (the `bedrooms`/`bathrooms` words produced by the fixed constants) must be identical between C and Rust and independent of the input | [x] |
| C19 | `driver` → `print_hex` | output framing: exactly `2 * sizeof(house_t)` = 32 hex digits, all in `[0-9a-f]`, followed by exactly one `\n`, total 33 bytes, for randomized inputs | [x] |
| C20 | `driver` → `print_hex` | loop bound: the only reachable `len` is `sizeof(raw)` = 16, so exactly 16 byte-pairs are emitted — no over-read past `raw`, no early termination | [x] |
| C21 | `driver` | **batched pipeline**: many calls in one stdout-buffering window (no flush in between), randomized argument sequence — concatenated output must match byte-for-byte | [x] |
| C22 | `driver` | **repeat / statelessness**: the same argument called N times in a row yields N identical 33-byte records (local `house` is re-zeroed each call) | [x] |
| C23 | `driver` | **cross-library interleaving**: alternating C, Rust, C, Rust with different arguments in one process — neither library perturbs the other's state or stdout | [x] |
| C24 | `driver` | **stdout sink / buffering mode** axis, 3 sinks: in-memory stream (`open_memstream`), a real on-disk file fully buffered (`_IOFBF`, batched `write(2)`), and the same file forced unbuffered (`setvbuf(…, _IONBF, 0)`, one `write(2)` per byte) — identical bytes from all three, for both libraries | [x] |
| C25 | `driver` | **model cross-check**: output equals the independently computed little-endian encoding of `{i32 floors, i32 3, f64 2.0}`, confirming the differential test actually observes value-dependent bytes rather than a constant | [x] |

### Feature / build-configuration axes

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` defines
no options, no `target_compile_definitions`, and no conditional sources (its only
flag is `-fno-strict-aliasing`, which does not change observable behaviour here).

Therefore the complete set of valid feature combinations is exactly one:

| # | feature combination | cargo invocation |
|---|---------------------|------------------|
| F1 | *(empty — no features exist)* | `cargo test --no-default-features` (identical to `cargo test`) |

Both `--no-default-features` and the default build are exercised in Phase D.

## Verification status

All 25 rows pass in both debug and release, under the single valid feature
combination (F1). Reproduce with `./verify_all.sh`.

`tests/phase_b_configs.rs` contains 26 `#[test]`s: one per row C1–C25, plus
`c00_harness_loads_two_distinct_libraries`, which asserts the harness really
dlopens two *different* files (a guard against accidentally comparing one
library against itself).

### Harness note — why fd 1 is not redirected

An earlier version of the harness redirected file descriptor 1 around each call.
That produced 15 spurious "divergences" in which libtest's own progress text
(`test c19_output_framing ... FAILED`) was interleaved into the captured bytes by
*other* test threads. The C bytes were correct throughout. The harness now
temporarily reassigns glibc's writable `stdout` `FILE *` global instead, which
`printf`/`putchar` read on every call in every shared object, while leaving fd 1
(and therefore Rust's `std::io::stdout()` and libtest) untouched.

### Harness sensitivity (mutation testing)

The suite was validated by injecting deliberate bugs into `src/lib.rs` and
confirming each is caught, so a pass is meaningful rather than vacuous:

| mutant | tests failed |
|---|---|
| byte order of `floors` reversed | 23 |
| `unsigned char` -> signed `char` sign-extension | 22 |
| `%02x` -> `%02X` (uppercase hex) | 23 |
| `%02x` -> `%x` (zero-padding dropped) | 25 |
| loop bound off by one | 25 |
| `bedrooms = 3` -> `4` | 25 |
| `bathrooms = 2.` -> `2.5` | 25 |
| trailing `printf("\n")` removed | 25 |
| `HouseT` fields reordered | 25 |
| `#[unsafe(no_mangle)]` removed | 26 (incl. Phase D symbol tests) |
| `#[repr(C)]` -> `#[repr(C, packed(4))]` | 0 — **equivalent mutant**: verified that on this target both give size 16 / offsets 0,4,8 and byte-identical output, so surviving is correct |

### Independent real-stdout cross-check

Outside the test harness, each `.so` was `dlopen`ed from a separate Python
process with **no `stdout` interception at all**, writing to real piped process
stdout, over 50 008 values (fixed seed 20260818, full `i32` range plus
boundaries):

```
1650264 bytes each (50008 x 33)
389007fefbe586b98197de7e955197f1  C
389007fefbe586b98197de7e955197f1  Rust (debug)
389007fefbe586b98197de7e955197f1  Rust (release)
```
