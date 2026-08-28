# CONFIGS.md — Phase A configuration surface (valid inputs)

Axes derived mechanically from the branches the C actually takes
(`switch (mode)` in `charinbuf`, the `if`s in the nine lower-level exported
functions, and the file-scope `static int counter` state machine). There are no
`#ifdef`s and no compile-time options in `c_src`, and the Rust crate declares no
cargo `[features]`, so the only configuration axes are runtime ones:

* **A1 — entry point**: `charinbuf` (the one-shot wrapper) *and* each of the nine
  low-level exports: `increment_counter`, `decrement_counter`,
  `multiply_counter`, `reset_counter`, `is_string_empty`,
  `find_char_in_buffer`, `create_buffer`, `validate_uint16_range`,
  `apply_operation`.
* **A2 — `charinbuf` mode**: `0 | 1 | 2 | 3 | 4 | default`.
* **A3 — `value` shape**: `INT_MIN`, `< 0`, `0`, `1..=65534`, `65535`, `65536`,
  `INT_MAX`.
* **A4 — `opt1` / `opt2` shapes** (mode 3 only): `0`, `> 0`, `< 0`, `INT_MAX`,
  `INT_MIN` (⇒ signed wrap-around in `+`, `-`, `*`).
* **A5 — static-counter state**: fresh, seeded by `reset_counter`, accumulated
  across a randomized sequence of the four ops, persistence across `.so` calls,
  and the reset-to-0 that `charinbuf` performs on entry.
* **A6 — string shape**: `NULL`, `""`, 1 byte, plain ASCII, high-bit (≥ 0x80)
  bytes, embedded NUL, leading NUL, long (≥ 4 KiB).
* **A7 — buffer/size/target shape** for `find_char_in_buffer`: `size` 0 / 1 /
  n / `> strlen` / `< strlen`; `target` at first / middle / last / absent /
  after `size`; `target == '\0'`; `target` with the high bit set (negative
  `char`).
* **A8 — function pointer** passed to `apply_operation`: each of the four
  counter ops, and (cross-library) the *same library's* op only, so that each
  library mutates its own `static counter`.

All randomized rows use a fixed-seed xorshift64* PRNG (`seed = 0x2026_0827_C0FF_EE01`)
so runs are reproducible; each row runs **many** inputs (256 – 4096 iterations,
noted per row). Every row compares the C `.so` against the Rust `.so` through
`libloading`: return value **and**, for the printing entry point, the exact
`stdout` byte stream (captured by `dup2`-ing fd 1 to a file around each call).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|------------------------------------------|-----|
| C1  | `validate_uint16_range` | all of `-1,0,1,2,254,255,256,32767,32768,65534,65535,65536,65537,INT_MAX,INT_MIN,INT_MIN+1,INT_MAX-1` + 4096 random `i32` | [x] |
| C2  | `is_string_empty` | `""` (A6 empty) | [x] |
| C3  | `is_string_empty` | 1-byte strings for every byte value `0x01..=0xFF` (incl. high-bit / negative `char`) | [x] |
| C4  | `is_string_empty` | random ASCII strings, len 1..64 (256 iters) | [x] |
| C5  | `is_string_empty` | leading-NUL string (`"\0abc"`) and long string (4 KiB) | [x] |
| C6  | `create_buffer` | `""` ⇒ 1-byte allocation, result is `""` | [x] |
| C7  | `create_buffer` | random byte strings (bytes `0x01..=0xFF`, len 1..128, 256 iters) — content + `strlen` compared, both pointers `free`d by the caller | [x] |
| C8  | `create_buffer` | long input (4 KiB, 8 KiB) | [x] |
| C9  | `create_buffer` | input with embedded NUL — only the prefix is copied | [x] |
| C10 | `find_char_in_buffer` | `size == strlen`, target at index 0 (first byte) | [x] |
| C11 | `find_char_in_buffer` | `size == strlen`, target in the middle, several occurrences ⇒ first match returned | [x] |
| C12 | `find_char_in_buffer` | `size == strlen`, target = last byte | [x] |
| C13 | `find_char_in_buffer` | `size == 1` (single-byte buffer), hit and miss | [x] |
| C14 | `find_char_in_buffer` | `size < strlen` and the only occurrence lies at/after `size` ⇒ miss | [x] |
| C15 | `find_char_in_buffer` | `size > strlen` (scans past the NUL, buffer padded) ⇒ can hit a byte after the NUL | [x] |
| C16 | `find_char_in_buffer` | `target == '\0'` with the NUL inside the range | [x] |
| C17 | `find_char_in_buffer` | `target` = every byte `0x00..=0xFF` against a 256-byte buffer holding all byte values (A7 sign edges) | [x] |
| C18 | `find_char_in_buffer` | randomized: random buffer (len 1..256), random `size` in `0..=len`, random `target` (1024 iters) — offset of the returned pointer compared | [x] |
| C19 | `reset_counter` | `0`, `1`, `-1`, `INT_MAX`, `INT_MIN` + random (256 iters) | [x] |
| C20 | `increment_counter` | from a seeded state, random deltas, including wrap past `INT_MAX` (512 iters) | [x] |
| C21 | `decrement_counter` | from a seeded state, random deltas, including wrap past `INT_MIN` (512 iters) | [x] |
| C22 | `multiply_counter` | from a seeded state, `×0`, `×1`, `×-1`, random factors, including overflow (512 iters) | [x] |
| C23 | all four counter ops | randomized *sequences* (length 1..32) of interleaved ops with random operands, comparing the counter after every step — A5 accumulated state (256 sequences) | [x] |
| C24 | `apply_operation` | each of the four ops (same-library pointer) × values `{0,1,-1,INT_MAX,INT_MIN}` + random (256 iters) | [x] |
| C25 | `apply_operation` | randomized sequences of ops applied through `apply_operation` (A5 + A8), counter compared after every step (256 sequences) | [x] |
| C26 | `charinbuf` | mode 0, `value` ∈ A3 (incl. `0`, `1`, `65535`, `65536`, `INT_MIN`, `INT_MAX`) + 512 random values; `opt1`/`opt2` random (unused) | [x] |
| C27 | `charinbuf` | mode 1, `value`/`opt1`/`opt2` random (all unused ⇒ must not affect output) (256 iters) | [x] |
| C28 | `charinbuf` | mode 2, `value`/`opt1`/`opt2` random (unused) — allocation + `strlen` + `free` path (256 iters) | [x] |
| C29 | `charinbuf` | mode 3, `value`/`opt1`/`opt2` random incl. `0`, `±1`, `INT_MAX`, `INT_MIN` ⇒ reset/increment/multiply/decrement with signed wrap (1024 iters) | [x] |
| C30 | `charinbuf` | mode 3 with `opt2 == 0` (counter zeroed by multiply) and `opt2 < 0` | [x] |
| C31 | `charinbuf` | mode 4, `value`/`opt1`/`opt2` random (unused) — `memchr` hit at index 21 (256 iters) | [x] |
| C32 | `charinbuf` | mode 3 then a direct `increment_counter` call ⇒ observes the `counter = 0` reset done on entry to `charinbuf`, and the final counter left behind (A5) | [x] |
| C33 | `charinbuf` | random *sequences* of modes interleaved with direct counter-op calls (128 sequences of length 8) — cross-call state, the composed pipeline | [x] |
| C34 | `charinbuf` | every mode `0..=4` × `value` ∈ {`INT_MIN`,`-1`,`0`,`1`,`65535`,`65536`,`INT_MAX`} × `opt1`,`opt2` ∈ {`INT_MIN`,`-1`,`0`,`1`,`INT_MAX`} (full cross-product, 875 combinations) | [x] |
| C35 | `charinbuf` | mode 0, **every** `value` in `-70000..=70000` (140 001 inputs — exhaustive across both range boundaries) | [x] |
| C36 | `charinbuf` | mode 0, strided sweep (prime stride 104 729) over the **whole `i32` domain**, ~41 000 inputs | [x] |
| C37 | `charinbuf` | mode 3, 50 000 randomized `(value, opt1, opt2)` triples biased towards overflow-inducing operands | [x] |
| C38 | `charinbuf` | all modes (valid *and* invalid) × 50 000 randomized argument triples | [x] |
| C39 | `charinbuf` + counter ops | 20 000-call mixed-mode stream followed by 1 000 direct counter ops — the composed pipeline as one continuous state history | [x] |
| C40 | `validate_uint16_range` | exhaustive `-80000..=80000` plus a ~1.2 M-sample strided sweep of the whole `i32` domain | [x] |
| C41 | `find_char_in_buffer` | exhaustive matrix: target byte `0x00..=0xFF` × match position `0..63` × `size` ∈ {0, 1, p, p+1, 64} (≈ 82 000 combinations) | [x] |
| C42 | counter ops | operand grid `-600..=600` (and ×7) × 4 ops × 9 seed states (≈ 43 000 steps) | [x] |

Row `Cn` corresponds to the test function named `cn_*` in
`tests/phase_b_valid.rs` (C1–C34) / `tests/phase_b_bulk.rs` (C35–C42).
