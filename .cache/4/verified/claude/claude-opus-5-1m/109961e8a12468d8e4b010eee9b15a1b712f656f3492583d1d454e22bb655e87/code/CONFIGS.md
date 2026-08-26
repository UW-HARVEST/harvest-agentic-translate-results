# CONFIGS.md — Phase B configuration surface

Derived **mechanically** from `c_src/src/driver.c`, `c_src/include/driver.h` and
`c_src/CMakeLists.txt`, the same way `ERRORS.md` is derived.

## Axis enumeration (what the C actually branches on)

**Axis 1 — runtime options / modes / flags: NONE.**
The public header declares exactly one function and no options:

```c
void driver(float x);
```

There is no context/handle struct, no setter, no global, no mutable state, no
mode enum, and no `typedef`/`enum` at all in either C file. Nothing the caller
can toggle. (`grep -nE "#define|enum|typedef|extern|static [^v]" src/driver.c
include/driver.h` yields only the `DRIVER_H_` include guard and the `static`
helper.)

**Axis 2 — build-time options / `#ifdef`: NONE.**
`CMakeLists.txt` declares no `option()`, no `target_compile_definitions`, and no
conditional sources; its sole flag is `-fno-strict-aliasing`, which has no
observable API effect. `Cargo.toml` has no `[features]` section, so the only
feature combination is the empty one (see `SYMBOLS.md`).

**Axis 3 — public entry points (full set, including the lowest level).**

| entry point | linkage | externally callable? | how it is exercised |
|-------------|---------|----------------------|---------------------|
| `driver(float)` | extern, exported (`T driver`) | YES — the only ABI entry point | called directly through `dlsym` on both `.so`s |
| `print_hex(unsigned char*, int)` | `static` (internal, lowest level) | NO — not in `nm -D` | reached only via `driver`, always with `len == sizeof(float) == 4`; its non-export is itself asserted (`ERRORS.md` row 8) |

There is no convenience-wrapper/low-level split to worry about: `driver` *is* the
lowest-level entry point in the ABI, and the one internal helper below it has a
single call site with a compile-time-constant `len`.

**Axis 4 — input shapes the code distinguishes.**
`driver`'s body has no conditional; the only branch in the library is
`print_hex`'s loop guard `i < len`. The behaviour therefore varies **only** with
the 4 bytes of `x`'s object representation, through two value-dependent
mechanisms in `printf("%02x", p[i])`:

* the `%02x` **zero-padding** decision (byte `< 0x10` vs `>= 0x10`), and
* the **`unsigned char` -> `int` promotion** (byte `< 0x80` vs `>= 0x80`, i.e.
  zero-extension, never sign-extension),

applied independently at each of the 4 byte positions. Combined with the IEEE-754
classes that a translation could accidentally canonicalize (subnormal, ±0, ±Inf,
qNaN, sNaN and their payloads), this gives the shape axis:
`{value class} x {sign} x {per-byte padding/promotion pattern} x {byte position}`.

## Configuration table

One row per meaningful combination of the axes above. Every row is run through
BOTH `.so`s via `dlsym` and compared byte-for-byte on the captured `stdout`, with
many randomized inputs per row (fixed seed `0x5EED_C0FFEE_u64`, so reruns are
reproducible). Rows marked exhaustive enumerate their whole subdomain rather than
sampling it.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `driver` | no options (none exist) + `+0.0` (`0x00000000`): all four bytes `0x00`, i.e. zero-padding taken in every position | `cfg_01_positive_zero` | [x] |
| 2 | `driver` | none + `-0.0` (`0x80000000`): sign byte `>= 0x80` (promotion path) with three `0x00` bytes | `cfg_02_negative_zero` | [x] |
| 3 | `driver` | none + positive **normals**, randomized: exponent uniform in `1..=254`, mantissa uniform in `0..=0x7fffff` (10,000 samples) | `cfg_03_positive_normals_random` | [x] |
| 4 | `driver` | none + negative **normals**, randomized: sign `1`, exponent `1..=254`, random mantissa (10,000 samples) | `cfg_04_negative_normals_random` | [x] |
| 5 | `driver` | none + positive **subnormals**: exponent `0`, mantissa `1..=0x7fffff` randomized (10,000 samples) + the exhaustive smallest/largest subnormal | `cfg_05_positive_subnormals_random` | [x] |
| 6 | `driver` | none + negative **subnormals**: sign `1`, exponent `0`, random mantissa (10,000 samples) | `cfg_06_negative_subnormals_random` | [x] |
| 7 | `driver` | none + small **integral** values a real consumer passes: every integer `-2048..=2048` converted to `f32` (exhaustive) | `cfg_07_integral_values_exhaustive` | [x] |
| 8 | `driver` | none + **powers of two** of both signs across the entire exponent range: `±2^k` for `k = -149..=127` (exhaustive, incl. subnormal powers) | `cfg_08_powers_of_two_exhaustive` | [x] |
| 9 | `driver` | none + named IEEE **limits**, both signs: `FLT_MIN`, `FLT_MAX`, `FLT_EPSILON`, `FLT_TRUE_MIN`, `1.0`, `-1.0` (exhaustive list) | `cfg_09_named_float_limits` | [x] |
| 10 | `driver` | none + **±Inf** (`0x7f800000`, `0xff800000`) | `cfg_10_infinities` | [x] |
| 11 | `driver` | none + **qNaN / sNaN** across payloads and both signs: exponent `0xff`, mantissa sweeping `1`, `2`, `0x400000`, `0x3fffff`, plus 4,000 random payloads x 2 signs | `cfg_11_nan_payload_matrix` | [x] |
| 12 | `driver` | none + **all-bytes-low** patterns exercising `%02x` zero-padding in every position simultaneously: `0x00000001`, `0x01010101`, `0x0f0f0f0f`, `0x0f000f00`, and randoms with every byte `< 0x10` (2,000 samples) | `cfg_12_all_bytes_below_0x10` | [x] |
| 13 | `driver` | none + **all-bytes-high** patterns exercising `unsigned char`->`int` promotion in every position: `0xffffffff`, `0x80808080`, `0xf0f0f0f0`, and randoms with every byte `>= 0x80` (2,000 samples) | `cfg_13_all_bytes_at_or_above_0x80` | [x] |
| 14 | `driver` | none + **per-position byte sweep**: for each byte position `0..=3`, all 256 values of that byte with the others held at `0x00`, then again with the others at `0xff` (exhaustive, 2,048 cases) — isolates a position-dependent padding/indexing bug | `cfg_14_per_byte_position_sweep` | [x] |
| 15 | `driver` | none + **exhaustive low 16 bits**: all `0x0000..=0xffff` in the low half, high half fixed at `0x3f80` (i.e. mantissa sweep of a normal) — 65,536 cases | `cfg_15_exhaustive_low_16_bits` | [x] |
| 16 | `driver` | none + **exhaustive high 16 bits**: all `0x0000..=0xffff` in the high half, low half fixed at `0x0000` (sign + exponent + high mantissa sweep, crosses every class boundary) — 65,536 cases | `cfg_16_exhaustive_high_16_bits` | [x] |
| 17 | `driver` | none + **uniform random over the full 2^32 bit-pattern domain** (200,000 samples, fixed seed) — the property-style row covering arbitrary mixed-class inputs | `cfg_17_full_domain_random` | [x] |
| 18 | `driver` | none + **decimal literals** a consumer would actually write: `0.1`, `0.5`, `1.5`, `3.14159`, `2.71828`, `1e-30`, `1e30`, `1e-45`, `3.4e38`, `-0.1`, ... (exhaustive list) | `cfg_18_decimal_literals` | [x] |
| 19 | `driver` | none + **repeated identical call** (same value 1,000 times) — proves no per-call state and that call N == call 1 | `cfg_19_repeated_identical_calls` | [x] |
| 20 | `driver` | none + **interleaved C/Rust ordering** on the one shared glibc `stdout` (C,R,C,R,... over a mixed value list) — exercises the composed pipeline / shared-buffer interaction rather than one library in isolation | `cfg_20_interleaved_c_and_rust` | [x] |
| 21 | `driver` | none + **long run crossing libc's 4 KiB `stdout` buffer boundary** (>3,000 calls => >27 KiB, flush boundaries land mid-record) — confirms both libraries emit through the same buffered stream with identical framing | `cfg_21_stdout_buffer_boundary_crossing` | [x] |
| 22 | `driver` (=> internal `print_hex`, `len == 4`) | none + **output framing invariant** over a mixed sample of all rows above: exactly 9 bytes per call, 8 lowercase hex digits from `[0-9a-f]` + one `'\n'`, matching the little-endian byte order of the argument's bit pattern | `cfg_22_output_framing_invariant` | [x] |

| 23 | `driver` | none + **EXHAUSTIVE: every one of the 2^32 `float` bit patterns**, swept in order in 2 Mi-value chunks. Because `driver` has one 32-bit input, no options and no state, this row *is* the entire input domain — it subsumes rows 1–18 rather than sampling them. | `exhaustive_full_domain` (`tests/exhaustive.rs`, `#[ignore]`d) | [x] |

### Row 23 result

The whole input domain was enumerated, in both profiles, comparing the two
`.so`s' `stdout` byte-for-byte:

```
dev     : VERIFIED 4294967296 bit patterns in 3105s  (all 2^32, [0x0, 0x100000000))
release : VERIFIED 4294967296 bit patterns in 2758s  (all 2^32, [0x0, 0x100000000))
```

Since `driver` is a pure function of one 32-bit argument with no options and no
state, enumerating all 2^32 arguments is not a sample of the configuration space —
it *is* the configuration space. The Rust translation is therefore verified
equivalent to the C for **every possible input**, not merely for the randomized
rows above.

All 23 rows are run under the single (empty) feature combination, which is the
only one that exists; `run_all_features.sh` re-runs the whole table for each
combination it enumerates from `Cargo.toml`, in both the `dev` and `release`
profiles (the release profile matters here: it is the one where LLVM is free to
reassociate float moves, so it is the configuration in which a NaN-canonicalizing
translation bug would most plausibly appear).

## Running the tables

```sh
./run_all_features.sh          # Phases B+C over every feature combo x {dev, release}

# Row 23 (the exhaustive sweep) is opt-in because it takes about an hour:
cargo build --offline
cargo test --offline --test exhaustive -- --ignored --nocapture
```

**The differential tests must run single-threaded.** They compare the two
libraries by capturing file descriptor 1, which is process-wide, and libtest
writes its own `test <name> ... ok` progress lines there too; if tests ran
concurrently that text would land inside a capture. `.cargo/config.toml` sets
`RUST_TEST_THREADS=1` and `run_all_features.sh` also passes
`-- --test-threads=1` explicitly (cargo resolves `.cargo/config.toml` relative to
the current directory, so the flag covers invocations from elsewhere). If a
capture is ever polluted anyway, `assert_reference_stream_clean` fails loudly
with an explanatory message instead of silently comparing garbage.

**`cargo test` does not rebuild a `crate-type = ["cdylib"]` library.** Always
`cargo build` (same profile and features) first, or the tests will load a stale
`libdriver.so`. `assert_fresh` in the harness enforces this by comparing mtimes
and refuses to run against a `.so` older than `src/`, so a stale artifact cannot
produce a false pass.
