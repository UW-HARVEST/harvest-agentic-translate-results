# CONFIGS.md — configuration-surface table (Phase A → verified in Phase B)

## Build-time configurations (enumerated first)

| axis | values | evidence |
|------|--------|----------|
| Cargo features | **none** — `Cargo.toml` has no `[features]` section, no optional dependencies, no `cfg(feature=…)` anywhere in `src/` | `grep -n features Cargo.toml` → no match; `grep -rn 'feature =' src/` → no match |
| CMake options | **none** — no `option()`, no `target_compile_definitions`, and no `#if`/`#ifdef`/`#define` in `c_src/src/main.c` | `grep -nE '^\s*#\s*(if\|ifdef\|ifndef\|define)' c_src/src/main.c` → no match |
| Cargo profiles | `dev`, `release` (`panic = "abort"`) — both exercised | `Cargo.toml` |

⇒ The complete set of valid feature combinations is **the single empty
combination**. `run_all_configs.sh` still runs the whole suite four times
(`default`/`--no-default-features` × `debug`/`release`) and *fails loudly* if a
`[features]` section, a CMake `option()` or a `#ifdef` ever appears, so the
enumeration cannot silently go stale.

## Runtime configuration axes the C code actually branches on

`main` takes no arguments; there are no environment variables, no `setlocale`
(so the program stays in the `"C"` locale no matter what `LC_*` say), no options
and no config files. Everything is a function of the **stdin byte stream** plus
the **kind of the stdin/stdout descriptors**. The axes are exactly what the C
code — and the glibc `%d` directive it delegates to — branches on:

* **A. number of successful `%d` conversions** — 0/1/2/3. `scanf` aborts at the
  first failure, so this selects which of `x`, `y`, `z` keep their pre-call
  values `0`, `123`, `0`.
* **B. `x == 1` ?** (L31) · **C. `y == 2` ?** (L37) · **D. `z == 3` ?** (L43) —
  the three sequential stages; only the *first* failing one prints.
* **E. inter-token whitespace** — the literal spaces in `"%d %d %d"` and each
  `%d` skip `isspace()`: `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'`, runs of
  them, and *no* whitespace before the first token.
* **F. sign prefix** per token — none / `+` / `-` (and sign-without-digits).
* **G. digit shape / magnitude** per token — single digit, leading zeros,
  `INT_MAX`, `INT_MIN`, `INT_MAX+1`, `UINT_MAX`, values that narrow (mod 2^32)
  *onto* 1/2/3, the `LONG_MAX`/`LONG_MIN` boundary, past-`LONG` (strtol
  saturation), 100 000-digit monsters.
* **H. trailing content after the 3rd token** — nothing / whitespace / garbage /
  more numbers / NUL bytes / 64 KiB of junk (must never be read).
* **I. stdin descriptor kind** — pipe, regular file, `/dev/null`, closed,
  directory, streams larger than the stream buffer, byte-at-a-time short reads.
* **J. stdout descriptor kind** — pipe, regular file, `/dev/null`, `/dev/full`,
  broken pipe (glibc picks full vs line buffering from this).
* **K. argv** — none / extra args (never inspected).
* **L. descriptor state left behind** — how much of stdin the process consumes
  is observable by the *next* reader of the same descriptor
  (`{ driver; cat; } < file`): glibc's buffer is `st_blksize` and `exit()`
  returns the unused read-ahead of a *seekable* descriptor
  (`_IO_cleanup` → `_IO_SYNC`).

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row runs BOTH compiled programs as external processes and compares
stdout + stderr + wait status byte-for-byte, with many randomized inputs from a
fixed-seed PRNG (`tests/common/mod.rs::Rng`).

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|--------------------------------------------|---|
| C1 | `driver` → `scanf` → `multi_stage` L49 | A=3, `x==1 && y==2 && z==3` — the only success path → `Ok!\nResult: 0\n`; 6 spellings (`+1`, `01`, zero-padded, …) × 8 randomized whitespace layouts | [x] |
| C2 | `driver` → `multi_stage` L31 | A=3, `x != 1`, random `y`,`z` over the full `i32` range (300 triples) | [x] |
| C3 | `driver` → `multi_stage` L37 | A=3, `x == 1`, `y != 2`, random `z` (300 pairs) | [x] |
| C4 | `driver` → `multi_stage` L43 | A=3, `x == 1`, `y == 2`, `z != 3` (300 values) | [x] |
| C5 | `driver` → `multi_stage` | A=3, 1000 fully random `i32` triples + 600 triples biased onto `magic ± 1` and the boundary set | [x] |
| C6 | `driver` → `scanf` | A=3, all 8 combinations of B×C×D, then each slot driven with `{0,-1,INT_MIN,INT_MAX,123}` | [x] |
| C7 | `driver` → `scanf` `%d` | A=3, E=each of the 6 `isspace` bytes alone as separator, ± leading/trailing, + 200 randomized mixed whitespace runs | [x] |
| C8 | `driver` → `scanf` `%d` | A=3, F=all 27 sign combinations × 4 randomized magnitudes + the magic triple | [x] |
| C9 | `driver` → `scanf` `%d` | A=3, G=leading zeros, 120 random pad widths 1…40, plus all-zero tokens of width 1…200 | [x] |
| C10 | `driver` → `scanf` `%d` | A=3, G=`INT_MAX`, `INT_MIN`, `INT_MAX+1`, `INT_MIN-1`, `UINT_MAX`, `2^32`, `-2^32`, … in each of the 3 slots and in all 3 at once | [x] |
| C11 | `driver` → `scanf` `%d` | A=3, G=values that narrow mod 2^32 *onto* the passing constants (`4294967297`→1, `-4294967295`→1, `k·2^32+{1,2,3}`) — the stage passes although the text is not "1" | [x] |
| C12 | `driver` → `scanf` `%d` | A=3, G=`LONG_MAX`, `LONG_MIN`, `±(LONG_MAX+1)`, `2^64-1`, `2^64`, `10^40`, 100-digit and **100 000-digit** numbers, + 120 random 19–25-digit numbers straddling `LONG_MAX` | [x] |
| C13 | `driver` → `scanf` | A=2 (EOF after the 2nd token), 200 random pairs × 3 trailing-whitespace shapes; `z` keeps 0 | [x] |
| C14 | `driver` → `scanf` | A=1, 200 random values × 3 shapes; `y` keeps 123, `z` keeps 0 | [x] |
| C15 | `driver` → `scanf` | A=0: empty stdin, each single whitespace byte, 60 random whitespace-only streams | [x] |
| C16 | `driver` → `scanf` | A=3 + H=10 tail shapes (none, `\n`, spaces, ` 4 5 6`, garbage, NULs, 64 KiB junk, 70 000 digits) × 4 stage outcomes | [x] |
| C17 | `driver` → `scanf` | A=3 with tokens spread over lines, `\r\n` endings, no final newline, 200 randomized line-ending mixes | [x] |
| C18 | `driver` | I=stdin a **regular file** vs a **pipe**: identical to each other and across programs (+100 randomized) | [x] |
| C19 | `driver` | I=stdin `/dev/null` (character device, immediate EOF) | [x] |
| C20 | `driver` | I=stdin ≥ 4 KiB/8 KiB/64 KiB/200 KiB so the scan spans several buffer refills; tokens straddling a refill boundary; 40 randomized near-boundary sizes | [x] |
| C21 | `driver` | I=stdin delivered **one byte at a time** (short reads) | [x] |
| C22 | `driver` | J=stdout a regular file (fully buffered) vs a pipe vs `/dev/null` — identical bytes and ordering | [x] |
| C23 | `driver` | K=extra argv (`a b c`, `""`, `--flag=value`, args with spaces/newlines/emoji) × 11 stdin shapes | [x] |
| C24 | C `main` via **`dlopen`/`dlsym` (libloading)** | `c_src/src/main.c` compiled a 2nd time as a `-shared -fPIC` object, `main` called through FFI with redirected stdin/stdout, fresh library copy per case so `static int y` restarts at 123; 19 fixed + 40 random shapes | [x] |
| C25 | `driver` | both profiles (`dev`, `release` with `panic="abort"`) × both feature spellings, whole suite | [x] |
| C26 | `driver` | L=**seekable** stdin: file offset left for the next reader must match (12 shapes + sizes 4000/4095/4096/4097/8191/8192/8193/20 000/50 000, tokens before *and* after long runs) | [x] |
| C27 | `driver` | L=**pipe** stdin: bytes still queued in the pipe must match (same shapes) | [x] |
| C28 | `driver` | L randomized: 60 random streams up to ~9 KiB, both file and pipe | [x] |
| C29 | `driver` → `scanf` `%d` | **Value-semantics probes** — inputs engineered so that each *plausible wrong* conversion semantics (wrap instead of saturate, clamp to `int` instead of narrowing, wrong whitespace set, digit-count limits, token-boundary errors) lands on a magic constant while glibc does not; plus a 600-case cross-check against an independent model of glibc's `%d` | [x] |
| C30 | `driver` → `scanf` | **Grammar fuzzing**: 1500 random streams, 3000 streams prefixed to reach the y/z stages, 800 arbitrary-byte streams, 1500 streams over the `0-9+- \t\n\r\v\f.,x\0` alphabet | [x] |
| C31 | `driver` → `scanf` | **Exhaustive** sweep of every 1-byte and every 2-byte stream over that 20-byte alphabet (20 + 400 cases) — pins down EOF/short-stream handling | [x] |

Mapping to tests: C1–C17 → `tests/configs.rs`; C18–C23 → `tests/fds.rs`;
C24 → `tests/dlopen_c_lib.rs`; C25 → `run_all_configs.sh`;
C26–C28 → `tests/stdin_offset.rs`; C29 → `tests/scan_semantics.rs`;
C30–C31 → `tests/fuzz_stream.rs`.

## Why the extra rows C26–C31 exist (oracle resolution)

The program's stdout only reveals whether the scanned values *equal* 1, 2 and 3
— a 3-bit oracle per run. A random sweep of large values therefore cannot see a
wrong overflow rule, because both the right and the wrong value are simply
"not 1". Rows C29–C31 exist to raise that resolution (engineered values, an
independent model, exhaustive short streams), and rows C26–C28 add a second,
independent observable (the descriptor state left behind). This is validated by
the mutant battery in `run_all_configs.sh --mutants`: every behavioural mutant
of `src/main.rs` must be killed by these rows.
