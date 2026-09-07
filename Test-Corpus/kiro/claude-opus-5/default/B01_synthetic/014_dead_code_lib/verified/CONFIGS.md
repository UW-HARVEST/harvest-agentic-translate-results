# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs. Axes derived mechanically from
the branches the C actually takes, not from a guess about what matters.

## Axis enumeration

**Runtime options / modes / flags.** None. The library has no init function, no
context struct, no global state, no setters, no `#ifdef`-gated behaviour, and no
environment lookups. `include/driver.h` declares one function and no types or
constants. `CMakeLists.txt` sets no `target_compile_definitions`. `Cargo.toml`
has **no `[features]` section**, so `--no-default-features` and the default build
are the same single configuration (verified in Phase D).

**Public entry points**, lowest level first — the full set from the C `.so`'s
dynamic symbol table, not just the one declared in the header:

| level | entry point | calls |
|---|---|---|
| 0 (lowest) | `printLine(const char*)` | `puts` only |
| 1 | `bad()` | `printLine` ×1 |
| 1 | `good()` | `printLine` ×1, then `static helperGood()` → `printLine` ×1 |
| 2 (composed) | `driver()` | `printLine` ×4 interleaved with `good()` and `bad()` |

**Input shapes the code distinguishes.** `printLine`'s only branch is
`line != NULL`, so the pointer axis is `{NULL, non-NULL}` (NULL is covered in
`ERRORS.md`). Below that, `puts` walks the buffer to its terminator, so the
distinguishing shapes are: length (0, 1, small, exactly at/around stdio's 4096-byte
buffer, ≫buffer), byte values (ASCII printable, control bytes, `\n`, `\t`,
high bytes / invalid UTF-8, all 255 non-NUL values), and content that looks like
a format string.

**Stream-state axis.** Output is appended to the process's shared `stdout`
`FILE*`, so ordering, buffering and flush boundaries are observable: a fresh
stream vs. a stream that already holds caller-written bytes, and single vs.
repeated vs. long call sequences.

## Rows (cross-product, pruned to what the C distinguishes)

Every row is exercised against **both** `.so`s through `libloading`, with
stdout captured on fd 1 and compared byte-for-byte. Rows marked *randomized* use
many generated inputs from a fixed-seed PRNG, not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printLine` | 1-byte string, every value `0x01..=0xFF` (exhaustive, 255 inputs) | `cfg_row1_single_byte_exhaustive` | [x] |
| 2 | `printLine` | random ASCII-printable strings, len 1..=64 — *randomized*, 512 inputs | `cfg_row2_random_ascii_short` | [x] |
| 3 | `printLine` | random arbitrary non-NUL bytes `0x01..=0xFF`, len 1..=256 (invalid UTF-8 included) — *randomized*, 512 inputs | `cfg_row3_random_arbitrary_bytes` | [x] |
| 4 | `printLine` | strings whose length straddles stdio's 4096-byte buffer: len ∈ {4094,4095,4096,4097,4098} plus random content — *randomized* | `cfg_row4_buffer_boundary_lengths` | [x] |
| 5 | `printLine` | very long strings, len 64 KiB…1 MiB, forcing multiple internal flushes — *randomized* | `cfg_row5_long_strings` | [x] |
| 6 | `printLine` | strings containing embedded newlines and other control bytes (`\n \r \t \v \f \b \x1b`) — *randomized* | `cfg_row6_control_bytes` | [x] |
| 7 | `printLine` | strings that are valid `printf` format strings (`%s`, `%d`, `%n`, `%%`, `%1000000d`) — must be emitted literally | `cfg_row7_format_string_content` | [x] |
| 8 | `printLine` | long random call **sequence** (256 calls, mixed shapes incl. NULL) against one accumulating stream — order and buffering must match | `cfg_row8_random_call_sequence` | [x] |
| 9 | `printLine` | called when the stream already holds caller-written bytes (interleaving with the consumer's own `write(2)`) | `cfg_row9_interleaved_with_caller_output` | [x] |
| 10 | `bad` | no arguments, fresh stream (level-1 entry point called directly) | `cfg_row10_bad` | [x] |
| 11 | `good` | no arguments, fresh stream — exercises the `static helperGood()` call the C makes | `cfg_row11_good` | [x] |
| 12 | `driver` | no arguments, fresh stream — the fully composed pipeline | `cfg_row12_driver` | [x] |
| 13 | `bad`, `good`, `driver` | each invoked repeatedly (16×) — idempotence / no hidden state | `cfg_row13_repeated_invocations` | [x] |
| 14 | `printLine`, `bad`, `good`, `driver` | randomly interleaved sequence of **all four** entry points, 256 calls — the composed pipeline as a real consumer drives it — *randomized* | `cfg_row14_all_entry_points_interleaved` | [x] |
| 15 | `driver` | `driver()` interleaved with direct `printLine`/`good`/`bad` calls, so the level-2 wrapper and the level-0 primitive share one stream | `cfg_row15_driver_mixed_with_primitives` | [x] |
| 16 | all four | called from a non-main thread (the C uses no TLS; must be identical) | `cfg_row16_called_from_secondary_thread` | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)` —
there is no `add_executable`, and `translation/Cargo.toml` declares only `[lib]`
with `crate-type = ["cdylib"]`. **The project builds no binary**, so the
"compare C and Rust stdout on the same inputs" gate has no executable to run. It
is instead satisfied at the library level: every row above compares the exact
stdout byte stream produced through each `.so`.
