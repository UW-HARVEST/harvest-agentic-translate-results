# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/luggage.c`

## Build-time configurations

* `Cargo.toml` has **no `[features]` section** → exactly ONE feature
  combination exists: the default (empty) set.  It is exercised as
  `--no-default-features`, `--all-features` and plain (identical builds);
  `check_all.sh` runs `cargo check`/`cargo test` for all three spellings.
* `c_src/CMakeLists.txt` has no options, no `#ifdef`s anywhere in the C source
  (`grep -c '#if' c_src/src/luggage.c` → 0) → one C configuration.
  Both the default cmake build (`add_executable`) and the
  `-shared -fPIC -Dmain=luggage_main` shared-library build of the same
  translation unit are used (see `SYMBOLS.md`).

## Runtime configuration axes (derived from the C source)

| axis | values the C code distinguishes | source |
|------|--------------------------------|--------|
| A. filter argv[i] | wildcard (`expected[0]=='-'`), exact literal, non-matching literal, empty string `""`, `-`+suffix | `matches` :57 |
| B. record count | 0, 1, 2, many (100+) | loop :93 |
| C. timestamp shape | 0, small, ties, ascending, descending, random, leading `+`/`-`, leading zeros, `INT_MAX`, `INT_MAX+1`, `UINT_MAX`, `UINT_MAX+1`, `LONG_MAX(+1)`, `LONG_MIN(-1)`, 40-digit | `%d` :102, `%010u` :73 |
| D. field width | shorter than max, exactly max (8/6/3/3/80), longer than max | :105/:109/:112 |
| E. separators | one space, many spaces, `\t`, `\n`, `\v`, `\f`, `\r`, none (`JFKLAX`), high byte (`\xa0`, not `isspace` in the C locale) | whitespace directives |
| F. comment shape | absent (`\n` right after arrival), one space, leading tabs, 80 chars, >80 chars, embedded NUL, bytes ≥ 0x80, `\r\n` line ends | :100/:112 |
| G. supersede structure | unique luggage ids; same id + same departure (superseded); same id + different departure (search stops, row 33 of ERRORS.md); 3-node chains; interleaved ids; exact duplicates | `supersedes`/`superseded` |
| H. list insertion position | into empty list, at head, in the middle, at the tail, among equal timestamps (ties keep insertion order) | `addRoutingDirectiveToList` |
| I. stream termination | EOF after each of the 6 conversions, with/without trailing `\n`, with/without trailing whitespace | :102–:114 |
| J. entry point | process (`main` via argv/stdin) **and** each exported function called directly through the `.so`: `addRoutingDirectiveToList`, `supersedes`, `superseded`, `matches`, `printMatchingDirectives`, `luggage_main` | `nm` |

## Combination table

Each row is exercised with MANY randomized inputs (fixed seed, deterministic
xorshift PRNG in `tests/support/mod.rs`) unless it is a fixed-shape row.
`P*` = `tests/differential_exec.rs`, `F*` = `tests/differential_ffi.rs`.

| #  | entry point(s) | configuration (options + input shape) | test | ok |
|----|----------------|----------------------------------------|------|----|
| 1  | process | 0 records (empty stdin) × all-wildcard filters | `p01_empty_input` | [x] |
| 2  | process | 1 well-formed record × all-wildcard filters, randomized fields | `p02_single_record_random` | [x] |
| 3  | process | 2 records, second timestamp < first (insert at head) | `p03_two_records_orders` | [x] |
| 4  | process | 2 records, second timestamp > first (insert at tail) | `p03_two_records_orders` | [x] |
| 5  | process | 3 records, middle insertion (axis H) | `p04_middle_insertion` | [x] |
| 6  | process | N∈[0,40] random records, random timestamps, unique luggage ids | `p05_many_random_records` | [x] |
| 7  | process | N∈[0,40] random records, timestamps drawn from a tiny pool → many ties (stable order) | `p06_tie_stability` | [x] |
| 8  | process | ascending timestamps (100 records) / descending / shuffled | `p07_sorted_streams` | [x] |
| 9  | process | timestamp shapes from axis C (each shape randomized) | `p08_timestamp_shapes` | [x] |
| 10 | process | field widths: all fields shorter than max, randomized | `p09_widths_short` | [x] |
| 11 | process | field widths: every field exactly at max (8/6/3/3/80) | `p10_widths_exact` | [x] |
| 12 | process | field widths: every field over max → truncation + re-parse of the remainder | `p11_widths_over` | [x] |
| 13 | process | separators: randomized mix of ` `, `\t`, `\n`, `\v`, `\f`, `\r`, multiple ws | `p12_separator_shapes` | [x] |
| 14 | process | no separator between departure and arrival (`JFKLAX`), randomized | `p13_no_separator` | [x] |
| 15 | process | comment absent / single space / tab-led / 80 / >80 / NUL / high bytes / CRLF | `p14_comment_shapes` | [x] |
| 16 | process | supersede structure: pool of 1–3 luggage ids × 1–3 departures, randomized (G) | `p15_supersede_pool` | [x] |
| 17 | process | supersede structure: same id, same departure, 2..5 node chain | `p16_supersede_chain` | [x] |
| 18 | process | supersede structure: same id, different departures (search stops early) | `p17_supersede_stops` | [x] |
| 19 | process | exact duplicate records ×2..4 | `p18_duplicates` | [x] |
| 20 | process | filters: wildcard `-` in each of the 4 positions, others exact (cross-product) | `p19_filter_cross_product` | [x] |
| 21 | process | filters: all 4 exact from an existing record | `p19_filter_cross_product` | [x] |
| 22 | process | filters: literal that matches nothing / empty string / `-`+suffix | `p20_filter_special` | [x] |
| 23 | process | filters drawn randomly from {wildcard, field value, random word, "", "-x"} × random record set | `p21_filter_random` | [x] |
| 24 | process | EOF at each of the 6 conversion points (axis I), randomized prefix records | `p22_eof_positions` | [x] |
| 25 | process | trailing newline present / absent / trailing spaces / trailing `\n\n\n` | `p23_stream_termination` | [x] |
| 26 | process | fully random byte streams (0–200 bytes) — mixes valid and invalid shapes | `p24_random_bytes` | [x] |
| 27 | process | random tokens from a small alphabet (`0-9A-Z a-z \n\t-+[]`) — stale-buffer paths | `p25_random_tokens` | [x] |
| 28 | process | 100–300 record streams built from a small pool (superseding + ties + filters at scale) | `p26_scale_pool` | [x] |
| 29 | process | high byte (`\xa0`) used as a separator (not `isspace`) | `p27_high_byte_separator` | [x] |
| 30 | `matches` (`.so`) | wildcard / exact / mismatching / empty expected / `-`+suffix / empty actual, randomized strings | `f10_matches_random` | [x] |
| 31 | `supersedes` (`.so`) | empty tail (NULL), 1..8 node chains, id present/absent, departure equal/different, first match early/late | `f11_supersedes_random` | [x] |
| 32 | `superseded` (`.so`) | single node, 2..8 node chains, superseded/not superseded | `f12_superseded_random` | [x] |
| 33 | `addRoutingDirectiveToList` (`.so`) | insert into empty list / at head / middle / tail / among ties, 0..8 existing nodes, randomized timestamps | `f13_add_random` | [x] |
| 34 | `addRoutingDirectiveToList` (`.so`) | full build-up: insert N randomized nodes one by one and compare the final chain order | `f14_add_sequence` | [x] |
| 35 | `printMatchingDirectives` (`.so`) | NULL list, 1..8 node lists × randomized filters × superseded nodes; stdout captured through fd 1 | `f15_print_random` | [x] |
| 36 | `luggage_main` (`.so`) | exported symbol exists in both `.so`s with the same name (behaviour covered by the process tests, since it consumes stdin and `exit()`s) | `symbol_parity_c_so_vs_rust_so` | [x] |
| 37 | process | 1..3 records where a `%d`/scan-set matching failure creates a stale-field record (axis I + row 10/13/17/19 of ERRORS.md), randomized | `p28_stale_records` | [x] |
| 38 | process | multi-line input where a >80-char comment leaks into the next iteration | `p29_comment_leak` | [x] |
| 39 | process | **cross-product fuzz**: all generators above mixed (pool records / random records / over-long fields / small-alphabet tokens / raw bytes) × random truncation × random filters, 1200 seeded cases | `p30_heavy_mixed_fuzz` | [x] |
| 40 | process | extreme shapes: 100/1000/5000-digit timestamps (glibc's `%d` workspace), 10k-char comment, 100k single token, 50k letters, 10k space run, 5k empty lines, exotic whitespace, 2000 ascending / 2000 descending records | `p31_extreme_sizes` | [x] |
| 41 | process | stdout is a pipe whose reader exits early (SIGPIPE), stdout closed (`>&-`), stdout on a full device, stdout on `/dev/null` | `p32_stdout_failure_modes` | [x] |
| 42 | process | filter arguments that are not valid UTF-8 (`\xff`, `-\xff`, `\x80\x80`, `\t`, …) in each of the 4 positions, and comments containing bytes ≥ 0x80 | `p33_non_utf8_filters` | [x] |
