# CONFIGS.md — Configuration-surface table (Phase A) / valid-path tests (Phase B)

## Mechanical derivation of the axes

The library is two functions in one translation unit:

```c
void printHexCharLine (char charHex) { printf("%02x\n", charHex); }
void driver           (char data)    { char result = data + 1; printHexCharLine(result); }
```

There are **no runtime option/mode/flag setters** (no `set_*`, no context
struct, no globals, no `enum`s — `driver.h` declares exactly one prototype and
`driver.c` adds one more external symbol). So the usual "options set" axis is
empty *by derivation*. The axes the C actually distinguishes are:

**Axis A — entry point.** Two public entry points, and they are *not*
independent: `driver` is the wrapper, `printHexCharLine` is the low-level
primitive that `driver` calls through the PLT/GOT. Both are exported from the
`.so`, so both are tested directly, and the composed path (`driver` ->
`printHexCharLine`) is tested as its own thing.

**Axis B — value class of the argument.** Straight-line code, but the *data* is
branched on downstream, inside `printf`, and by the implicit conversions:

1. `char` -> `int` default argument promotion. `char` is **signed** on this
   target (confirmed in the disassembly: `movsbl`), so the promoted value is
   negative for `0x80..=0xFF`.
2. `%x` reinterprets the promoted `int` as `unsigned int`. A negative value
   therefore prints as **8** hex digits (`ffffff80`), a non-negative one as 1–2.
3. `%02x`'s minimum field width **2** only takes effect when the value needs
   fewer than 2 digits, i.e. `0x00..=0x0F` (zero-padding path).
4. In `driver`, `data + 1` is evaluated in `int` and converted back to `char`,
   so `0x7F` wraps to `-128` and `0xFF` wraps to `0x00`. The disassembly shows
   `movzbl` (zero-extend) for the addition and `movsbl` for the pass-down, i.e.
   the wrap is a plain low-byte truncation.

   => value classes: `[0x00,0x0F]`, `[0x10,0x7F]`, `[0x80,0xFF]`, plus the
   boundaries `0x0F/0x10`, `0x7F/0x80`, `0xFF/0x00` shifted by one for `driver`.

**Axis C — argument form at the ABI.** The parameter is a narrow integer in a
32-bit register, so a caller may present a value with dirty high bits. Both
`.so`s must use only the low byte. (Valid-input mirror of ERRORS.md E9/E10.)

**Axis D — call count / sequencing.** `printf` writes into the shared `stdout`
`FILE` buffer, so behaviour composes across calls: one call, many calls,
`driver` and `printHexCharLine` interleaved, and enough output to overflow
`BUFSIZ` and force several underlying `write()`s.

**Axis E — `stdout` buffering mode and fd type.** `_IOFBF` / `_IOLBF` /
`_IONBF`, and regular file vs pipe. This changes when and how `printf` emits
bytes; the emitted byte stream must be identical for C and Rust.

**Axis F — library load / call order.** C-first vs Rust-first, to rule out
order dependence (lazy PLT binding, first-call stdio initialisation).

There are **no** size/width/count/element-type/byte-order/format axes: nothing
is indexed, allocated, or serialised.

## Build configurations

`Cargo.toml` has no `[features]`; `CMakeLists.txt` has no options or
conditional compilation; the C has no `#ifdef` beyond the header guard. So there
is **exactly one feature combination** (the empty set), invoked explicitly as
`--no-default-features` to prove it.

`Cargo.toml` *does* define a `[profile.release]`, and the cargo profile turned
out to be a load-bearing axis: an unsound ABI assumption produced identical
output in `debug` and divergent output in `release` (see SYMBOLS.md Finding 1).
So every row below is run under **2 build configurations**:
`{no features} x {debug, release}`, via `./run_diff_tests.sh`.

Additionally, the whole table was re-run against C libraries built at `-O0`,
`-O1`, `-O2`, `-O3` and `-Os` (`DRIVER_C_SO=<path> cargo test`) — 35/35 tests
pass against every one, in both Rust profiles.

## Configuration table

Every row is exercised with **many** inputs (exhaustive over the 256-value
domain where the row's class allows, otherwise seeded-random, seed
`0x5DEECE66D0001234` via a deterministic SplitMix64 generator) and compared
**byte-for-byte** between the two `.so`s.

All of a row's inputs are driven inside a single captured child process, so a
row's assertion compares the two libraries' **entire concatenated output
streams**. On a mismatch, `assert_same_over_values` maps the first differing
output line back to the exact input that produced it, so batching costs nothing
in diagnostic precision (demonstrated against a mutation that diverges at only
1 of 256 inputs).

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|-------------------------------------------|-----|
| C1  | `printHexCharLine` | value class `0x00..=0x0F` — `%02x` zero-padding path; **exhaustive** (16 values) | [x] |
| C2  | `printHexCharLine` | value class `0x10..=0x7F` — positive, exactly 2 digits, no padding; **exhaustive** (112 values) | [x] |
| C3  | `printHexCharLine` | value class `0x80..=0xFF` — negative `char`, sign-extended to 8 digits, field width inert; **exhaustive** (128 values) | [x] |
| C4  | `printHexCharLine` | **exhaustive over the entire domain**, all 256 bit patterns | [x] |
| C5  | `printHexCharLine` | boundary values only: `0x00, 0x0F, 0x10, 0x7E, 0x7F, 0x80, 0x81, 0xFE, 0xFF` | [x] |
| C6  | `driver` | `data ∈ 0x00..=0x0E` -> result `0x01..=0x0F`, padding path; **exhaustive** | [x] |
| C7  | `driver` | `data == 0x0F` -> result `0x10`, crosses the padding boundary | [x] |
| C8  | `driver` | `data ∈ 0x10..=0x7E` -> positive 2-digit result; **exhaustive** | [x] |
| C9  | `driver` | `data == 0x7F` -> `char` overflow wrap to `-128`, result prints 8 digits | [x] |
| C10 | `driver` | `data ∈ 0x80..=0xFE` -> negative result, 8 digits; **exhaustive** | [x] |
| C11 | `driver` | `data == 0xFF` -> wrap to `0x00`, back into the padding path | [x] |
| C12 | `driver` | **exhaustive over the entire domain**, all 256 bit patterns | [x] |
| C13 | both, directly | 4096 seeded-random values, each fed to **both** entry points in the same capture, verifying the low-level primitive and the wrapper agree per value | [x] |
| C14 | `printHexCharLine` | **Axis C**: raw `int` argument form with dirty high bits (`0xDEADBE00 \| b`, `0x1FF`, `i32::MIN`, `i32::MAX`, + 512 random `i32`s) | [x] |
| C15 | `driver` | **Axis C**: raw `int` argument form with dirty high bits, same input set | [x] |
| C16 | `printHexCharLine` | **Axis D**: 1000 seeded-random calls accumulated in a single capture (buffer accumulation, output ordering) | [x] |
| C17 | `driver` | **Axis D**: 1000 seeded-random calls accumulated in a single capture | [x] |
| C18 | `driver` + `printHexCharLine` interleaved | **Axis D**: 1000 calls alternating between the wrapper and the primitive in one capture — exercises the composed pipeline and shared buffer | [x] |
| C19 | both | **Axis D**: 20000 calls in one capture (≈150 KiB ≫ `BUFSIZ`), forcing many underlying `write()`s and partial-buffer boundaries | [x] |
| C20 | both | **Axis E**: `stdout` set `_IONBF` (unbuffered) in a forked child, 256 exhaustive values | [x] |
| C21 | both | **Axis E**: `stdout` set `_IOLBF` (line buffered, 1-byte buf) in a forked child, 256 exhaustive values | [x] |
| C22 | both | **Axis E**: `stdout` set `_IOFBF` (fully buffered, small 8-byte buf to force mid-record flushes) in a forked child, 256 exhaustive values | [x] |
| C23 | both | **Axis E**: `stdout` redirected to a **pipe** rather than a regular file, exhaustive 256 values | [x] |
| C24 | both | **Axis F**: call order Rust-first-then-C, and C-first-then-Rust, over the exhaustive domain (both orders asserted equal) | [x] |

All rows C1–C24 are implemented in `tests/differential_valid.rs` and pass.

## Verification status

```
./run_diff_tests.sh
### feature combinations to verify: 1
### feature combination: <none>   profile: debug     -> 35 passed, 0 failed
### feature combination: <none>   profile: release   -> 35 passed, 0 failed
### ALL FEATURE COMBINATIONS x PROFILES PASSED
```

Rows C14/C15 (Axis C, the out-of-range `int` argument form) are the rows that
exposed the one real defect found during verification; it was visible only in
the `release` profile. See SYMBOLS.md "Findings" and ERRORS.md rows E9/E10.

The suite's sensitivity was validated by injecting 11 mutations into
`src/lib.rs`: every behaviour-changing mutation was detected in both profiles,
including mutations whose output differs for only a single input value out of
256. See the table in SYMBOLS.md.
