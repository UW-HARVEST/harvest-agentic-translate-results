# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically from `grep -n 'return\|assert\|exit(' c_src/src/*.c` plus
every explicit range/NULL check and every min/max constant in the C sources.
There are **no `assert`s** and **no `exit()` calls** in the C code; rejections
are expressed as `return <code>`, `return false`, a substituted default value,
or a message on `stderr`.

Test column: name of the differential test in `tests/` that constructs exactly
that condition and asserts C and Rust return the *same* code / sentinel.

## engine.c — `run_engine` numeric error codes

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| 1 | `run_engine` | op `0` (push) is the last word: `prog_fetch(&p,&imm)` fails | `return 1` | [x] `err_engine_codes` |
| 2 | `run_engine` | op `1` (add) with `stack.len == 0` (first `iv_pop` fails) | `return 2` | [x] `err_engine_codes` |
| 3 | `run_engine` | op `1` (add) with `stack.len == 1` (second `iv_pop` fails) | `return 2` | [x] `err_engine_codes` |
| 4 | `run_engine` | op `2` (mul) with `stack.len == 0` | `return 3` | [x] `err_engine_codes` |
| 5 | `run_engine` | op `2` (mul) with `stack.len == 1` | `return 3` | [x] `err_engine_codes` |
| 6 | `run_engine` | op `4` (drop) with `stack.len == 0` | `return 4` | [x] `err_engine_codes` |
| 7 | `run_engine` | op `6` (jump) is the last word: `prog_fetch(&p,&k)` fails | `return 5` | [x] `err_engine_codes` |
| 8 | `run_engine` | op `6` with `k` present but `stack.len == 0` (no condition) | `return 6` | [x] `err_engine_codes` |
| 9 | `run_engine` | op `6`, `cond != 0`, `(size_t)k > p.n - p.ip` (jump past end) | `return 7` | [x] `err_engine_codes` |
| 10 | `run_engine` | op `6`, `cond != 0`, **negative** `k` → `(size_t)k` is huge → same check | `return 7` | [x] `err_engine_codes`, `err_jump_negative` |
| 11 | `run_engine` | op `7` (repeat) is the last word: `prog_fetch(&p,&times)` fails | `return 8` | [x] `err_engine_codes` |
| 12 | `run_engine` | op `7` with `times` present but nothing after it (`p.ip >= p.n`) | `return 9` | [x] `err_engine_codes` |
| 13 | `run_engine` | op `9` (stream) is the last word: `prog_fetch(&p,&m)` fails | `return 10` | [x] `err_engine_codes` |
| 14 | `run_engine` | op `9` with `m < 0` | `return 11` | [x] `err_engine_codes`, `err_stream_m_range` |
| 15 | `run_engine` | op `9` with `(size_t)m > vm->stack.len` | `return 11` | [x] `err_engine_codes`, `err_stream_m_range` |
| 16 | `run_engine` | opcode not in `0..=10` (e.g. `11`, `12`, `INT_MAX`) | `return 99` | [x] `err_engine_codes`, `err_opcode_out_of_range` |
| 17 | `run_engine` | **negative** opcode (e.g. `-1`, `INT_MIN`) — `switch` default | `return 99` | [x] `err_engine_codes`, `err_opcode_out_of_range` |
| 18 | `run_engine` | op `10` (halt) — early, non-error termination | `return 0` (rest of program ignored) | [x] `err_engine_codes` |
| 19 | `run_engine` | `n == 0` (empty program, incl. `code == NULL`) — loop never runs | `return 0`, VM untouched | [x] `err_zero_len`, `err_null_code_zero_len` |
| 20 | `run_engine` | op `7`, inner 1-instruction `run_engine` returns non-zero → *not* propagated: `p.ip = saved_ip+1`, `vm_trace(vm,12)`, `break` | outer keeps running, `return 0` at end; trace gains `12` | [x] `err_repeat_inner_failure` |

## util.c — boolean / sentinel rejections

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| 21 | `iv_reserve` | `need <= v->cap` (nothing to do, incl. `need == 0`) | `true`, `data`/`cap` unchanged | [x] `err_iv_reserve` |
| 22 | `iv_reserve` | `need` so large that doubling reaches `nc > SIZE_MAX/2` (e.g. `SIZE_MAX`, `SIZE_MAX/2+2`) | `false`, vector unchanged, **no `realloc` call** | [x] `err_iv_reserve` |
| 22b | `iv_reserve` | `nc * sizeof(int)` itself wraps `size_t` to 0 (`need ≥ 2^62`) → the C calls `realloc(ptr, 0)`: with `data == NULL` glibc returns a 0-byte block and `iv_reserve` reports **success with a bogus `cap`**; with a live buffer glibc **frees it** and returns NULL → `false` + dangling `data` (latent C bug; buffer contents afterwards are indeterminate, so only the reported result is compared) | as described, identical on both sides | [x] `err_iv_reserve_size_wrap` |
| 23 | `iv_reserve` | `realloc` fails (`need = 1<<61` → 8 EiB request) | `false`, vector unchanged | [x] `err_iv_reserve` |
| 24 | `iv_push` | `v->len == v->cap` and `iv_reserve` fails (`cap*2` overflows / allocation fails) | `false`, `len` unchanged | [x] `err_iv_push_reserve_fail` |
| 25 | `iv_pop` | `v->len == 0` (empty) | `false`, `out` **not written** | [x] `err_iv_pop` |
| 26 | `iv_pop` | `out == NULL` on a non-empty vector | `true`, `len` decremented, no store | [x] `err_iv_pop` |
| 27 | `iv_peek` | `v->len == 0` | returns `def` verbatim (any `int`, incl. `INT_MIN`, `-777`) | [x] `err_iv_peek_default` |
| 28 | `prog_fetch` | `p->ip >= p->n` (exhausted, incl. `n == 0`) | `false`, `*out` **not written**, `ip` unchanged | [x] `err_prog_fetch` |
| 29 | `prog_fetch` | `p->ip > p->n` (corrupt/overshot ip set by the caller) | `false` | [x] `err_prog_fetch` |

## a.c / b.c / lib.c — value-domain special cases (the "negative input" rejections)

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| 30 | `target` (lib.c) | `code < 0` (incl. `INT_MIN`) | `return 7` | [x] `err_target_negative` |
| 31 | `target` (a.c, static) | `code < 0` | `(state_a & 1) ? 6 : 5` — depends on accumulated state, `state_a` **not** updated | [x] `err_a_negative_state` (via `call_a_once`/`process_a_stream`) |
| 32 | `target` (b.c, static) | `code < 0` | `flipflop ? 2 : 6` — `flipflop` **is** toggled first | [x] `err_b_negative_state` (via `call_b_once`/`process_b_stream`) |
| 33 | `process_a_stream` | `n == 0` (and `xs == NULL`) — loop skipped, clamps still applied | `INT_MIN` (`-2147483648`) | [x] `err_zero_len`, `err_null_ptr_zero_len` |
| 34 | `process_a_stream` | any input at all — `acc < -0x80000000LL` compares as `unsigned long long`, always true | always `INT_MIN` | [x] `cfg_process_a_stream` |
| 35 | `process_b_stream` | `n == 0` (and `xs == NULL`) | `1` (initial `acc`) | [x] `err_zero_len`, `err_null_ptr_zero_len` |
| 36 | `classify`/`process_stream` (via `run_engine`) | `impl_id` outside `{0,1}` — any other `int` (`2`, `-1`, `7`, `INT_MIN`, `INT_MAX`) selects the lib.c `target` path | identical behaviour for every such value | [x] `err_impl_id_out_of_range` |

## main.c — CLI level rejections

| # | function | trigger (exact invalid input/condition) | expected C result | ✔ |
|---|----------|------------------------------------------|-------------------|---|
| 37 | `main` | `--help` anywhere in argv | `usage()` on stderr (`Usage: %s …` with `argv[0]`), `return 0`, remaining args ignored | [x] `scripts/cli_diff.sh` |
| 38 | `main` | argument where `strtol` leaves `*e != '\0'` (`abc`, `12abc`, `0x10`, `+-5`, `"  12  "`, `\xff\xfe`) | `skip '<arg>'` on stderr, not pushed | [x] `scripts/cli_diff.sh` |
| 39 | `main` | argument that is the **empty string** — `strtol` performs no conversion so `e == nptr` and `*e == '\0'` is *true* | pushes `0` (quirk, reproduced) | [x] `scripts/cli_diff.sh` |
| 40 | `main` | no numeric argument survives (`code.len == 0`), with or without `--stdin` | `no program` on stderr, `return 2` | [x] `scripts/cli_diff.sh` |
| 41 | `main` | out-of-`int` numeric argument (`99999999999999999999`, `-99999999999999999999`, `2147483648`, `4294967296`) | `strtol` saturates to `LONG_MAX`/`LONG_MIN`, then `(int)` truncation (`-1`, `0`, `-2147483648`, `0`) | [x] `scripts/cli_diff.sh` |
| 42 | `read_stdin` | token that `strtol` rejects → silently dropped (no `skip` message on this path) | not pushed, `count` not incremented | [x] `scripts/cli_diff.sh` (`in.junk`) |
| 43 | `read_stdin` | line longer than `sizeof buf == 4096` → `fgets` splits it, a number may be cut in half | both halves parsed independently | [x] `scripts/cli_diff.sh` (`in.long`, `in.splittoken`) |
| 44 | `read_stdin` | embedded `NUL` byte → the `while (*p)` walker stops, rest of that `fgets` chunk is dropped | remainder of the line ignored, next line still parsed | [x] `scripts/cli_diff.sh` (`in.nul`, `in.nulmid`) |
| 45 | `main`/`read_stdin` | empty stdin / stdin without trailing newline | no tokens / last token still parsed | [x] `scripts/cli_diff.sh` (`in.empty`, `in.noeol`) |

## Generic FFI boundary cases (covered even though not in the C's own checks)

| # | case | C behaviour | ✔ |
|---|------|-------------|---|
| G1 | `run_engine(impl, NULL, 0, &vm)` | `n == 0` → returns `0` without dereferencing | [x] `err_null_code_zero_len` |
| G2 | `process_a_stream(NULL, 0)` / `process_b_stream(NULL, 0)` | loop skipped → `INT_MIN` / `1` | [x] `err_null_ptr_zero_len` |
| G3 | `iv_pop(v, NULL)` | explicit `if (out)` guard → no store | [x] `err_iv_pop` |
| G4 | `iv_free` twice / on a zeroed vector | `free(NULL)` is a no-op, fields re-zeroed | [x] `err_double_free_and_reinit` |
| G5 | `iv_peek` / `iv_pop` / `prog_fetch` on a **zero-initialised** struct (`data == NULL`, `len == 0`) | default / `false`, no dereference | [x] `err_iv_peek_default`, `err_iv_pop`, `err_prog_fetch` |
| G6 | out-of-range "enum-like" ints across FFI: `impl_id` (see #36), opcode (see #16/#17), `vm_trace(vm, t)` with arbitrary `t` (`t & 25` indexes the 26-letter table, negative `t` included) | never out of bounds; letter = `"abc…z"[t & 25]` | [x] `err_impl_id_out_of_range`, `err_opcode_out_of_range`, `cfg_vm_print_trace_alphabet` |
| G7 | `vm_print` with an empty label (`""`), a 300-byte label, a label containing `%d`/`%s`, and a **NULL** label | `%s` prints the bytes verbatim; glibc prints `(null)` for NULL | [x] `cfg_vm_print_labels`, `err_vm_print_null_label` |
| G7b | `run_engine` on a VM whose `steps` the caller pre-set to `INT_MAX` → `vm->steps++` overflows (UB in C, wraps in practice) | wraps to `INT_MIN`, run continues | [x] `cfg_engine_steps_overflow` |
| G7c | `main` called through `dlopen`/`dlsym` on the `.so` with `argc == 0, argv == NULL`, and with each argv shape | same exit code, stdout and stderr | [x] `err_main_symbol_via_dlopen` |
| G8 | over-sized `run_engine` length: `n` larger than the real buffer | **UB in C (out-of-bounds read)** — not differentially testable, deliberately not tested | n/a |
| G9 | `NULL` struct pointers to `iv_*`, `prog_*`, `vm_*`, `vm_print` | **UB in C — unconditional dereference, crashes the process**; there is no C check to mirror, so no test (documented, not tested) | n/a |

Rows G8/G9 are the only entries with no test: the C code performs no check
there, so any "expected result" would be undefined behaviour rather than a
rejection. Everything the C actually *checks* has a test above.
