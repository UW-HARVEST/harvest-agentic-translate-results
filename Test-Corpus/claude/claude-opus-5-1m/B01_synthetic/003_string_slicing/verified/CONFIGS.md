# CONFIGS.md — configuration-surface table (Phase A / Phase B)

Axes derived mechanically from `c_src/src/main.c` (there is no header, no option
struct, no `#ifdef`: the *entire* runtime configuration of this library is the
`(argc, argv)` pair, so the axes are the branches the C takes on it).

**Axis E — entry point**
* `EP1` — the lowest-level entry point: the `main` symbol exported by the shared
  object, called through `libloading` with a caller-built `argv`. Gives full
  control over `argc`, over each `argv[i]`, and over the *memory layout* of the
  vector (which `main` observes with `end == argv[3]`).
* `EP2` — the composed pipeline: the executable, started with `execve`, compared
  on stdout **and** stderr **and** exit status.

**Axis A — `argc`** (`if ((argc > 4) || (argc == 1))`, `if (argc >= 3)`,
`if (argc == 4)`): `2`, `3`, `4` are the valid shapes; `0`, `1`, `>4`, negative
are in `ERRORS.md` (E1, E2, B1, B2).

**Axis S — shape of `argv[1]`** (only `strlen` and the `%.*s` slice depend on
it): empty (`len == 0`), one byte, short ASCII, long (4 KiB), bytes ≥ 0x80 /
non-UTF-8, embedded whitespace and newlines, digits-only.

**Axis N — lexical form of `argv[2]` / `argv[3]`** (what `strtol(_,_,10)`
branches on): plain digits, leading C-locale whitespace (`" \t\n\v\f\r"`),
explicit `+`, explicit `-`, leading zeros, digits followed by junk (`"3abc"`,
`"3 4"`), base-10 reading of `"0x3"` (→ `0`), no digits at all (→ E3),
`INT_MAX`/`INT_MIN`, values that only differ after the `long`→`int` truncation
(`2^31`, `2^32`, `2^32+k`), `LONG_MAX`/`LONG_MIN` and beyond (saturation).

**Axis V — value relationship** (`start > len`, `stop > len`, `stop <= start`,
`stop - start`): `start == 0`, `0 < start < len`, `start == len`, `start > len`;
`stop == len`, `stop == start + 1`, `start < stop < len`, `stop <= start`,
`stop > len`.

**Axis L — layout of the `argv` vector** (observable through
`if (end == argv[3])`): `Contiguous` (one block, exactly what `execve`
produces), `Separate` (each string its own allocation), `Alias`
(`argv[3] == argv[2] + k`, i.e. a pointer *into* `argv[2]`).

**Axis P — build profile of the two artifacts**: `dev` and
`release` (`panic = "abort"`), C at `-O0` (CMake default) and `-O2`.

Each row below is checked off only after it passes for **many randomized inputs
with a fixed seed** (`tests/differential_so.rs`, `tests/differential_cli.rs`);
the iteration count per row is in the test.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | EP1 | A=2, S=empty (`len == 0`), L=Contiguous | `cfg_c1_argc2_empty` | [x] |
| C2 | EP1 | A=2, S=1 byte (every one of the 255 possible non-NUL bytes) | `cfg_c2_argc2_single_byte` | [x] |
| C3 | EP1 | A=2, S=short random ASCII (1..32), L=Contiguous | `cfg_c3_argc2_short_ascii` | [x] |
| C4 | EP1 | A=2, S=random arbitrary bytes 0x01..0xFF incl. whitespace/newlines/high-bit | `cfg_c4_argc2_random_bytes` | [x] |
| C5 | EP1 | A=2, S=long (up to 4096 bytes) | `cfg_c5_argc2_long_string` | [x] |
| C6 | EP1 | A=3, V=`start == 0`, S=all shapes (incl. empty) | `cfg_c6_argc3_start_zero` | [x] |
| C7 | EP1 | A=3, V=`start == len` (boundary, prints just `"\n"`) | `cfg_c7_argc3_start_eq_len` | [x] |
| C8 | EP1 | A=3, V=`0 < start < len` random, S=random bytes | `cfg_c8_argc3_start_interior` | [x] |
| C9 | EP1 | A=3, V=`start > len` (one past, and far past) → E4 | `cfg_c9_argc3_start_past_end` | [x] |
| C10 | EP1 | A=3, N=decorated numeric forms (whitespace / `+` / `-` / leading zeros) around a valid `start` | `cfg_c10_argc3_numeric_forms` | [x] |
| C11 | EP1 | A=3, N=digits followed by junk (`"3abc"`, `"3 4"`, `"0x3"`, `"3."`) | `cfg_c11_argc3_trailing_junk` | [x] |
| C12 | EP1 | A=4, V=`0 <= start < stop <= len` random pairs, S=random bytes | `cfg_c12_argc4_valid_window` | [x] |
| C13 | EP1 | A=4, V=`start == 0 && stop == len` (whole string) | `cfg_c13_argc4_whole_string` | [x] |
| C14 | EP1 | A=4, V=`stop == start + 1` (single byte window, every offset) | `cfg_c14_argc4_single_byte_window` | [x] |
| C15 | EP1 | A=4, N=decorated forms on **both** numeric arguments (cross product of whitespace/sign/zeros/junk) | `cfg_c15_argc4_numeric_forms` | [x] |
| C16 | EP1 | A=4, N=`long`→`int` truncation & `strtol` saturation values (`2^31`, `2^32`, `2^32+k`, `LONG_MAX`, `LONG_MIN`, `±10^20`) on `start` and/or `stop` | `cfg_c16_argc4_truncation_values` | [x] |
| C17 | EP1 | L=Separate (each argument separately malloc'd), A∈{2,3,4}, random args | `cfg_c17_layout_separate` | [x] |
| C18 | EP1 | L=Contiguous (exec-like block), A∈{2,3,4}, random args | `cfg_c18_layout_contiguous` | [x] |
| C19 | EP1 | L=Alias: `argv[3] == argv[2] + k` for every `k`, A=4 — the only configuration in which `end == argv[3]` can be true | `cfg_c19_layout_alias` | [x] |
| C20 | EP1 | repeated invocation: many calls to `main` in one process, alternating configurations (checks there is no hidden per-process state) | `cfg_c20_repeated_calls` | [x] |
| C21 | EP2 | CLI, A∈{2,3,4}, random ASCII args, compare stdout+stderr+exit status | `cfg_c21_cli_random_ascii` | [x] |
| C22 | EP2 | CLI, S=non-UTF-8 `argv[1]` (raw high bytes through `OsString`) | `cfg_c22_cli_non_utf8` | [x] |
| C23 | EP2 | CLI, full fuzz across A (0..6 arguments), N (all forms incl. non-numeric), S (all shapes) — valid *and* invalid mixed | `cfg_c23_cli_fuzz` | [x] |
| C24 | EP1+EP2, P=release | the whole EP1 + EP2 suite re-run against the `release` (`panic = "abort"`) Rust artifacts, and against the C source compiled both at `-O0` (CMake default) and `-O2`/`-O3` | `scripts/verify.sh` (4-way matrix) | [x] |
| C25 | EP1 | N=fuzzed `argv[2]`/`argv[3]`: arbitrary bytes, digit-only, `strtol`-flavored mixes (whitespace+sign+digits+junk), non-converting strings; both layouts; argc 3 and 4 — 6000 iterations, asserted to reach the success path and every rejection reachable with argc 3/4 | `cfg_c25_fuzz_numeric_arguments` | [x] |
| C26 | EP1 | N=very long digit strings: 1..500 leading zeros, 19/20/21/40/200 significant digits, negative variants (the `strtol` overflow/saturation path) | `cfg_c26_long_digit_strings` | [x] |
| C27 | EP2 | CLI, S=100 000-byte argument (more output than a pipe buffer) with start/stop at both ends, at `len` and one past `len` | `cfg_c27_cli_huge_argument` | [x] |
| C28 | EP2 | CLI, stdout is a pipe whose reader closes early (broken pipe) — the C is killed by `SIGPIPE`, so the Rust must not inherit the runtime's `SIG_IGN` | `cfg_c28_cli_broken_pipe` | [x] |
| C29 | EP2 | CLI, stdout closed before `main` runs, on the success and on the error paths | `cfg_c29_cli_closed_stdout` | [x] |
| C30 | EP1 | `argv` storage must be treated as read-only: every `assert_same*` snapshots the vector and asserts neither implementation modified it | built into `assert_same`/`assert_same_argc` | [x] |
