# CONFIGS.md — Phase B configuration-surface table

Axes the C actually branches on (derived from `c_src/src/lib.c`):

* **A1 — `charinbuf` `mode`**: `switch (mode)` with cases `0,1,2,3,4` + `default`
  (line 103). This is the library's only "option" selector.
* **A2 — `value` shape for mode 0 / `validate_uint16_range`**: `< 0`, `0`,
  `1..65534`, `65535`, `> 65535` (the two `if`s at lines 81–82).
* **A3 — `value/opt1/opt2` shape for mode 3**: sign, zero, and
  overflow-inducing magnitudes for `reset → increment → multiply → decrement 5`.
* **A4 — counter state**: fresh vs. accumulated (the `static int counter`);
  `charinbuf` forces `counter = 0` (line 101) but direct calls to the four
  `*_counter` exports do not.
* **A5 — `is_string_empty` input shape**: NULL, `""`, first byte non-NUL,
  first byte NUL but longer buffer, high-bit first byte.
* **A6 — `find_char_in_buffer` input shape**: NULL vs non-NULL buffer;
  `size` = 0 / 1 / many / shorter-than-match / longer-than-first-match;
  `target` = present-first / present-last / duplicated / absent / `'\0'` /
  high-bit (negative `char`).
* **A7 — `create_buffer` input shape**: NULL, `""`, short, long, embedded
  high-bit bytes (length taken with `strlen`).
* **A8 — `apply_operation` `op`**: NULL, and each of the four counter ops.
* **A9 — stdout**: every mode prints; the exact byte stream is part of the
  contract (compared byte-for-byte via fd-1 capture).
* **Cargo features**: `translation/Cargo.toml` declares **no `[features]`
  table**, so the only feature combination is the default (empty) one. Verified
  by `scripts/check_features.sh`, which enumerates features from Cargo.toml and
  runs the whole suite for each combination (default, `--no-default-features`).

Cross product, pruned to combinations the C distinguishes. Every row is driven
against BOTH `.so`s through `libloading`, with many randomized inputs
(fixed-seed xorshift PRNG, seed `0x2024_C0FFEE`), and both the return value and
the captured stdout bytes are compared.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `validate_uint16_range` | full sweep of boundaries `{INT_MIN,-2,-1,0,1,2,65534,65535,65536,65537,INT_MAX}` + 4096 random `i32` | [x] |
| 2 | `is_string_empty` | NULL | [x] |
| 3 | `is_string_empty` | `""` (single NUL) | [x] |
| 4 | `is_string_empty` | non-empty ASCII, first byte non-NUL | [x] |
| 5 | `is_string_empty` | first byte NUL but trailing garbage after it | [x] |
| 6 | `is_string_empty` | first byte high-bit (`0x80..0xFF`, negative `char`) — 256 random strings | [x] |
| 7 | `create_buffer` | NULL | [x] |
| 8 | `create_buffer` | `""` → 1-byte allocation, empty result | [x] |
| 9 | `create_buffer` | short/long random NUL-free byte strings (len 1..512, incl. high-bit bytes) — 512 random cases; result compared byte-for-byte and `free`d | [x] |
| 10 | `create_buffer` | string with bytes after the terminating NUL (only prefix copied) | [x] |
| 11 | `find_char_in_buffer` | NULL buffer, `size` 0 and > 0, any target | [x] |
| 12 | `find_char_in_buffer` | non-NULL buffer, `size == 0` | [x] |
| 13 | `find_char_in_buffer` | `size == 1`, target == byte 0 / != byte 0 | [x] |
| 14 | `find_char_in_buffer` | target present at index 0 (first byte) | [x] |
| 15 | `find_char_in_buffer` | target present at index `size-1` (last byte) | [x] |
| 16 | `find_char_in_buffer` | target duplicated → first occurrence must win | [x] |
| 17 | `find_char_in_buffer` | target absent from the whole buffer | [x] |
| 18 | `find_char_in_buffer` | target present only at index `>= size` (truncated search must miss) | [x] |
| 19 | `find_char_in_buffer` | `target == '\0'`, buffer containing embedded NULs | [x] |
| 20 | `find_char_in_buffer` | `target` high-bit / negative `char` (`0x80..0xFF`) incl. sign-extension check | [x] |
| 21 | `find_char_in_buffer` | 2048 randomized (buffer bytes, size, target) triples; returned offset compared | [x] |
| 22 | `increment_counter` | fresh state, then accumulated sequence — 2048 random deltas, running value compared at every step | [x] |
| 23 | `decrement_counter` | fresh + accumulated, 2048 random deltas | [x] |
| 24 | `multiply_counter` | fresh + accumulated, 2048 random factors (incl. 0, 1, -1, INT_MIN) | [x] |
| 25 | `reset_counter` | full `i32` range sample, 2048 randoms + `{INT_MIN,-1,0,1,INT_MAX}` | [x] |
| 26 | all four `*_counter` | interleaved random operation sequence (2048 ops), asserting equal running state — covers overflow wraparound of `+`, `-`, `*` | [x] |
| 27 | `apply_operation` | `op = NULL`, random `value` | [x] |
| 28 | `apply_operation` | `op` = each of the four counter exports **of the same library**, random values, accumulated state | [x] |
| 29 | `apply_operation` | `op` = counter export resolved from the *other* library's handle is deliberately NOT crossed (would share the wrong `counter`); instead each library's own symbol address is used — documented | [x] |
| 30 | `charinbuf` | mode 0, `value` valid (`0`, `1`, `65535`, randoms in range) — stdout + return | [x] |
| 31 | `charinbuf` | mode 0, `value` invalid (`-1`, `INT_MIN`, `65536`, `INT_MAX`, randoms out of range) | [x] |
| 32 | `charinbuf` | mode 1 (fixed strings; `value/opt1/opt2` must be ignored — randomized to prove it) | [x] |
| 33 | `charinbuf` | mode 2 (malloc/free path; `value/opt1/opt2` ignored — randomized) | [x] |
| 34 | `charinbuf` | mode 3, all of `value/opt1/opt2` zero | [x] |
| 35 | `charinbuf` | mode 3, positive small values | [x] |
| 36 | `charinbuf` | mode 3, negatives | [x] |
| 37 | `charinbuf` | mode 3, `opt2 == 0` (multiply collapses to 0) | [x] |
| 38 | `charinbuf` | mode 3, `opt2 == -1` / `INT_MIN` (overflow wraparound of `*`) | [x] |
| 39 | `charinbuf` | mode 3, `value == INT_MAX`/`INT_MIN` with `opt1 != 0` (overflow of `+`/`-`) | [x] |
| 40 | `charinbuf` | mode 3, 2048 fully randomized `(value,opt1,opt2)` triples | [x] |
| 41 | `charinbuf` | mode 4 (memchr path; args ignored — randomized) | [x] |
| 42 | `charinbuf` | `default` branch: `mode` ∈ `{-1,5,6,100,INT_MIN,INT_MAX}` + randoms outside `0..=4` | [x] |
| 43 | `charinbuf` | repeated invocation of every mode in sequence on one handle (counter reset at entry must make results idempotent) | [x] |
| 44 | `charinbuf` | mode sweep `0..=4` interleaved with direct `*_counter` calls (proves the `counter = 0` reset at entry) | [x] |
| 45 | binary/driver | none: the project builds only a shared library (`add_library(... SHARED)`, `crate-type = ["cdylib"]`), so there is no executable stdout to diff. Instead stdout is diffed at the FFI level for rows 30–44. | [x] |
