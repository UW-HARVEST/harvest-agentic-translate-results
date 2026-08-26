# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## Mechanical derivation of the axes

The C library is `int main(void)` containing `printf("Hello World!\n"); return 0;`.
Grepping for every branch/option construct (see `ERRORS.md` for the counts):
`if`/`switch`/`for`/`while`/`goto` = **0**, `#if`/`#ifdef` = **0**,
`argc`/`argv` = **0**, `getenv` = **0**, `setvbuf`/`signal` = **0**, stdin reads
= **0**. There is **one** code path and **no** runtime options, so the axes are
*not* internal flags — the C code branches on nothing.

What genuinely varies, and what the C's observable behavior therefore depends
on, is the **environment `main` executes in**. These are the real axes:

| axis | values the C distinguishes |
|---|---|
| **A. entry point / linkage** | executable entry (`c_driver` / `driver`); `.so` export called via `dlopen`+`dlsym` (`libc_driver.so` / `librust_driver.so`) |
| **B. stdout kind** | tty, pipe (reader draining), pipe (reader closed → E1), regular file, file opened `O_APPEND`, `/dev/null`, `/dev/full` (E3), closed fd (E4), read-only fd (E6), directory fd (E7) — *the buffering mode glibc picks depends on this (line- vs block-buffered), which is why it is an axis* |
| **C. stdin kind** | inherited tty, `/dev/null`, regular file with data (offset must be untouched), pipe with data, closed fd (E8) |
| **D. stderr kind** | inherited, captured pipe (must stay empty), closed (E5) |
| **E. argv shape** | none; 1; 4096 args; ~100 KiB single arg; `argv[0]` = `""`; `argv[0]` non-UTF-8 |
| **F. environment** | inherited; empty (`env -i`); oversized (256 KiB); `LC_ALL`/`LANG` = `C`, `en_US.UTF-8`, `tr_TR.UTF-8` (locale-sensitive case rules), invalid locale name |
| **G. call multiplicity** | 1 call; randomized N<100 sequential `.so` calls; C and Rust calls **interleaved** on one fd; 32 concurrent processes |
| **H. observable** | stdout bytes, stderr bytes, exit code, **terminating signal**, stdin offset after exit, return value of `main` |

Rows below are the cross-product pruned to the combinations that are actually
distinguishable. Every row is exercised with **randomized inputs** (fixed seed
`0x5EED_1EC5`, deterministic xorshift PRNG) wherever a row has a value to
randomize (argv contents/counts/lengths, env contents, file contents preceding
the write, stdin payloads, call counts, interleaving order); rows whose inputs
are structurally fixed are repeated across randomized *fd arrangements* and
call counts instead.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|---|---|---|---|
| C1 | exe | stdout = pipe (drained), stdin = inherited, stderr = pipe, no args, inherited env — the baseline | `b1_baseline_pipe` | [x] |
| C2 | exe | stdout = **regular file**, compare file bytes + exit code | `b2_stdout_regular_file` | [x] |
| C3 | exe | stdout = regular file opened **`O_APPEND` with pre-existing random content** (randomized prefix bytes/length) | `b3_stdout_append_prefixed` | [x] |
| C4 | exe | stdout = **`/dev/null`**, stderr captured | `b4_stdout_devnull` | [x] |
| C5 | exe | stdout = **tty** (allocated pty) → glibc switches to line buffering | `b5_stdout_tty` | [x] |
| C6 | exe | stdout = pipe read **only after child exit** (buffer-drain ordering) | `b6_pipe_read_after_exit` | [x] |
| C7 | exe | randomized **argv**: count ∈ [0,64], random lengths/bytes incl. NUL-free non-UTF-8 | `b7_random_argv` | [x] |
| C8 | exe | randomized **environment**: count ∈ [0,64] random KEY=VALUE, plus `env_clear` | `b8_random_env` | [x] |
| C9 | exe | `argv[0]` = `""`, and `argv[0]` = non-UTF-8 bytes | `g3_arg0_and_env_edges` | [x] |
| C10 | exe | locale matrix `LC_ALL`/`LANG` ∈ {C, POSIX, en_US.UTF-8, tr_TR.UTF-8, de_DE.ISO-8859-1, invalid name, empty} × stdout=pipe | `b9_locale_matrix` | [x] |
| C11 | exe | stdin = **regular file with random data**; assert stdin **offset unchanged** after exit (C never reads it) | `b10_stdin_offset_preserved` | [x] |
| C12 | exe | stdin = pipe with random data (unread), `/dev/null`, and closed | `b11_stdin_kinds` + `e8_closed_stdin` | [x] |
| C13 | exe | **32 concurrent** invocations, each with piped stdout+stderr | `b12_concurrent` | [x] |
| C14 | exe | 100 sequential invocations — determinism/idempotence (every run compared to run 0) | `b13_repeat_determinism` | [x] |
| C15 | exe | oversized env (256 KiB) + argv counts {0, 1, 4096} + one ~100 KiB arg | `g2_argv_counts_and_huge_arg` | [x] |
| C16 | **.so** | `dlopen` → `dlsym("main")` → call **once**, fd 1 → temp file; compare bytes **and** `int` return | `b14_ffi_single_call` | [x] |
| C17 | **.so** | N=100 **sequential** calls, randomized N per iteration; output must be N identical lines | `b_repeated_and_interleaved` | [x] |
| C18 | **.so** | C and Rust `main` calls **interleaved in randomized order** on the *same* fd — pins down per-call flush behavior | `b_repeated_and_interleaved` | [x] |
| C19 | **.so** | fd 1 → pipe (drained) instead of file — different buffering class through FFI | `b15_ffi_pipe` | [x] |
| C20 | **.so** | fd 1 → `/dev/null`; return value only | `b16_ffi_devnull` | [x] |
| C21 | **.so** | called with **junk in argument registers** (`main(void)` must ignore them) | `g1_junk_arguments_ignored` | [x] |
| C22 | **.so** | `dlopen` twice / both libraries resident simultaneously; symbol addresses distinct and non-null | `c_and_rust_so_export_only_main` | [x] |
| C23 | exe + .so | **cross-check**: exe stdout bytes ≡ `.so` single-call bytes, for both C and Rust | `b17_exe_so_cross_check` | [x] |

## Notes on axes deliberately *not* turned into rows

* **Cargo features:** `Cargo.toml` has **no `[features]` table** and
  `CMakeLists.txt` has **no `option()`/`add_definitions`/`#ifdef`**, so the
  feature powerset is the single empty set. `--no-default-features` and the
  default build are the same configuration; both are still run (see
  `run_all_configs.sh`).
* **Buffering mode** is not independently settable (the C never calls
  `setvbuf`); it is a *consequence* of axis B, so it is covered by C1–C6/C19
  rather than being its own axis.
* **`/dev/full`, closed/read-only/directory fds, closed-reader pipes** are error
  conditions and live in `ERRORS.md` (E1–E8) instead of being duplicated here.
