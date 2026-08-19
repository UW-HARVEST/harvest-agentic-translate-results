# CONFIGS.md — Phase B configuration-surface table

## Axes mechanically derived from the C source

`c_src/include/driver.h` declares the **complete** public API:

```c
void driver(int x);          /* the one and only public entry point */
```

`c_src/src/driver.c` contains the only other function, the lowest-level one:

```c
static void print_hex(unsigned char *p, int len);   /* internal, not in the ABI */
```

Branch inventory (`if` / `switch` / `#ifdef` / ternary in the C): **none**, apart
from the single `for (i = 0; i < len; i++)` loop in `print_hex`. There are no
runtime option setters, no modes, no flags, no global state, no compile-time
`#ifdef` configuration, and `Cargo.toml` declares **no `[features]`** — so the
only feature combination is the empty/default one (Phase D).

The axes the code therefore actually distinguishes are:

| axis | values the C code treats differently |
|------|--------------------------------------|
| **A. entry point** | `driver` (public); `print_hex` (lowest-level, reached only via `driver` with `p = &house`, `len = sizeof(house_t) = 16`) |
| **B. `floors` bit pattern** | the `%02x` conversion is per-byte and value-dependent: bytes `0x00`, `0x01..0x0f` (zero-pad path), `0x10..0x7f`, `0x80..0xfe` (high-bit/sign-extension path), `0xff` |
| **C. byte position** | `p[i]` for `i = 0..15`: the 4 `floors` bytes, the 4 `bedrooms` bytes (constant `3`), the 8 `bathrooms` bytes (constant `2.0`) — offset arithmetic must be index-exact |
| **D. sign of `floors`** | positive / zero / negative (two's-complement high bytes `0x00` vs `0xff`) |
| **E. range extremes** | `INT_MIN`, `INT_MAX`, and one step inside each |
| **F. `stdout` stream configuration** | the library's only output channel is glibc `printf`, whose behaviour is parameterised by the stream: fully buffered (file), fully buffered (pipe), line buffered (`_IOLBF`), unbuffered (`_IONBF`) |
| **G. call multiplicity / interleaving** | 1 call; N calls same input; N calls differing inputs; C and Rust calls interleaved in one buffer window; enough calls to overflow the 4 KiB stdio buffer and force a real `write(2)` mid-stream |

## Rows (pruned cross-product of A–G)

Every row is exercised against **both** `.so`s through their exported `driver`
symbol and compared byte-for-byte. Rows marked *randomized* use many inputs from
a fixed-seed (0x2026_0818) xorshift generator, not a single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` → `print_hex` | axis B/D: `floors = 0`; every `floors` byte `0x00` (zero-pad path on all 4 bytes) | [x] |
| C2 | `driver` → `print_hex` | axis B/D: `floors = -1`; every `floors` byte `0xff` (high-bit path on all 4 bytes) | [x] |
| C3 | `driver` → `print_hex` | axis B/D: small positive, `1..=255` — byte 0 varies, bytes 1–3 are `0x00`; *randomized* | [x] |
| C4 | `driver` → `print_hex` | axis B/D: small negative, `-256..=-1` — byte 0 varies, bytes 1–3 are `0xff`; *randomized* | [x] |
| C5 | `driver` → `print_hex` | axis B: all four bytes drawn from `0x00..=0x0f` (forces the leading-zero `%02x` pad on every byte); *randomized* | [x] |
| C6 | `driver` → `print_hex` | axis B: all four bytes drawn from `0x80..=0xff` (high bit set on every byte — catches signed-`char` promotion bugs); *randomized* | [x] |
| C7 | `driver` → `print_hex` | axis E: boundary set `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` | [x] |
| C8 | `driver` → `print_hex` | axis B: patterns embedding `0x0a`, `0x0d`, `0x00`, `0x25` (`%`) bytes — newline/CR/NUL/format bytes inside the dumped data must be hex-escaped, never interpreted; *randomized placement* | [x] |
| C9 | `driver` → `print_hex` | axis B (full domain): uniform random over all 2³² `int` values, 512 iterations, fixed seed | [x] |
| C10 | `driver` → `print_hex` | axis C: byte-position sweep — a single `0xff` byte at each position 0–3, others `0x00` | [x] |
| C11 | `driver` → `print_hex` | axis C: byte-position sweep — a single `0x01` byte at each position 0–3, others `0xff` | [x] |
| C12 | `print_hex` (via `driver`) | axis A/C: loop-bound exactness — output must be exactly 33 bytes (32 lowercase hex digits + one `\n`), no over/under-run of `len = 16`; checked for every randomized input | [x] |
| C13 | `print_hex` (via `driver`) | axis C: constant-field bytes — offsets 4–7 must dump `03000000` (`bedrooms = 3`) and offsets 8–15 `0000000000000040` (`bathrooms = 2.0`, little-endian IEEE-754), independent of `floors` | [x] |
| C14 | `driver` | axis F: `stdout` fully buffered, redirected to a **regular file** (the default configuration); randomized inputs | [x] |
| C15 | `driver` | axis F: `stdout` fully buffered, redirected to a **pipe**; randomized inputs | [x] |
| C16 | `driver` | axis F: `stdout` **unbuffered** (`setvbuf(_IONBF)`) — per-`printf` write path; randomized inputs | [x] |
| C17 | `driver` | axis F: `stdout` **line buffered** (`setvbuf(_IOLBF)`) — flush-on-`\n` path; randomized inputs | [x] |
| C18 | `driver` | axis G: 8 consecutive calls with the *same* input in one buffer window → 8 identical lines (statelessness) | [x] |
| C19 | `driver` | axis G: 64 consecutive calls with *differing* randomized inputs in one buffer window (no cross-call state, no reused static buffer) | [x] |
| C20 | `driver` (C ↔ Rust) | axis G: C and Rust calls **interleaved** in a single capture window (shared glibc `stdout` `FILE*`) — asserts ordering and buffering are indistinguishable | [x] |
| C21 | `driver` | axis G: 400 calls ≈ 13 KiB > 4 KiB stdio buffer, forcing real `write(2)` syscalls mid-stream; whole stream compared | [x] |
| C22 | `driver` | axis F×G: unbuffered stream × 64 interleaved randomized calls (worst case for flush-order divergence) | [x] |

## Feature combinations (Phase D)

`Cargo.toml` declares **no `[features]`** table and `c_src/CMakeLists.txt` has no
`option()` / `add_definitions` / `#ifdef`-driven variants (a single
`add_library(driver SHARED src/driver.c)`). The powerset of features therefore
has exactly **one** member: the no-feature build.

`./verify.sh` enumerates that powerset mechanically from `Cargo.toml` (so it
scales automatically if features are ever added) and, for each combination,
runs `cargo check --all-targets`, `cargo test` in **both** the `dev` and
`release` profiles (`release` additionally exercises `panic = "abort"`), and
diffs `nm -D --defined-only` between the two `.so`s.

## How to run

```sh
cd translated_rust
(mkdir -p c_src/build && cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .)
cargo test            # 31 differential tests, strictly sequential
./verify.sh           # every feature combo × profile + symbol-table diff
```

Two properties of the test binary are load-bearing:

* `harness = false` — the tests capture file descriptor 1 process-globally
  (that *is* the library's observable behaviour), so they must never run
  concurrently. Owning `main` makes that true regardless of `--test-threads`.
* the test rebuilds the cdylib itself and asserts the `.so` is not older than
  `src/*.rs`. `cargo test` does **not** rebuild a `crate-type = ["cdylib"]`
  library, so without this guard the suite would silently compare against a
  stale Rust `.so` and report false passes.
