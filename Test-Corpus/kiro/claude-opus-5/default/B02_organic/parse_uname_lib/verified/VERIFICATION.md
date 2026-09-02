# VERIFICATION.md — completion gate

Reproduce everything with one command:

```
cd translation && bash scripts/verify_all.sh
```

Last run: **GATE: PASS**.

## What is under test

`c_src` is a single translation unit (`src/lib.c`, 147 lines) exporting three
functions. `translation/src/lib.rs` is its Rust counterpart. Both are built as
shared libraries and loaded side by side with `libloading`; **no Rust function
is ever called directly**, so the `#[no_mangle]` / `extern "C"` export wrappers
are themselves part of what is verified (`tests/common/mod.rs::Lib::open`).

The C is the ground truth. Where it is odd, the Rust reproduces the oddity:

* `parse_uname_string` mutates the caller's buffer in place, so the harness
  compares the **whole buffer including 32 bytes of slack on each side**, not
  just the output struct.
* `*(p + strlen(p) - 1) = '\0'` writes one byte *before* the string when the
  string is empty. Rows E25–E28 cover all four sites.
* The Windows branch never probes for the architecture (row E24), and the Unix
  branch runs the probe on the already-truncated buffer (row E23).
* `get_os_arch` returns the first match in **`ARCHS[]` order**, not the
  left-most match in the string (rows C2, C31).
* `w_regexec` does not `regfree` when `regcomp` fails; it does when it succeeds.

## Gate

| # | requirement | evidence | result |
|---|-------------|----------|--------|
| 1 | `SYMBOLS.md`: `nm -D` shows 0 missing / 0 unresolved non-libc symbols | `scripts/symbol_diff.sh` | 3/3 exports match; `ldd -r` reports nothing unresolved |
| 2 | Phase B: every `CONFIGS.md` row passes across randomized inputs | `tests/phase_b_valid.rs`, 40 tests | 40 passed |
| 3 | Phase C: every `ERRORS.md` row has a passing error-path differential test | `tests/phase_c_errors.rs`, 36 tests | 36 passed |
| 4 | every row is really checked off and really backed by a passing test | `scripts/audit_artifacts.py` | 32 + 40 rows audited, 0 problems, 0 orphaned tests |
| 5 | holds under every feature combination | `scripts/feature_matrix.sh` | 1 combination (no `[features]` table, no `cfg` in the source) |
| 6 | the suite can actually detect divergence | `scripts/mutation_check.py` | **24 / 24 mutants killed** |

79 tests total: 40 (Phase B) + 36 (Phase C) + 3 (Phase D resource/ABI).

## Why the mutation check is here

Passing differential tests prove nothing on their own — a suite that never
compares the right thing also passes. `scripts/mutation_check.py` injects 24
concrete bugs into `src/lib.rs` (wrong `ARCHS` order, inverted `regexec`
polarity, `strdup` omitted, `" [Ver: "` offset off by one, `os_build` skipped,
each of the three regexes altered, `REG_EXTENDED` dropped, the `osd == NULL`
guard removed, an off-by-one in the `malloc` size, `regfree` removed, …),
rebuilds, and requires `cargo test` to fail each time.

One of those bugs — removing `regfree` — produces **byte-identical output
forever**, so output comparison alone cannot see it. That is what
`tests/phase_d_resources.rs` is for: it measures `mallinfo2().uordblks` around
20 000 `w_regexec` calls and requires near-zero retention (measured: **0 bytes**
for both libraries). Without that test the mutant survived; with it, all 24 die.

## Platform ABI assumptions (measured, not assumed)

`src/lib.rs` mirrors glibc's opaque regex types. A throwaway C probe confirmed:

```
sizeof(regex_t)    = 64    _Alignof(regex_t) = 8     -> Rust: [u64; 8]
sizeof(regoff_t)   = 4                               -> Rust: c_int
sizeof(regmatch_t) = 8     offsetof(rm_eo)  = 4      -> Rust: { c_int, c_int }
REG_EXTENDED       = 1     REG_NOMATCH      = 1
```

`d3_regex_abi_assumptions` pins the Rust side of this; `d1` exercises the
`regex_t` reservation destructively (20 000 `regcomp`/`regfree` round-trips
would corrupt the stack if 64 bytes were not enough).

## Known limits

* `malloc` returning NULL is unchecked by the C at five sites. Forcing it would
  make both libraries dereference NULL identically and kill the test process, so
  it is documented in `ERRORS.md` rather than tested.
* `parse_uname_string(NULL, osd)` segfaults in both (the C dereferences `uname`
  inside `strstr` *after* the `!osd` check). Not a rejection the C implements, so
  it is documented rather than asserted in-process.
* `c_src/` was not modified. Only `c_src/build/` (a fresh CMake output directory)
  was created; the `.c`, `.h` and `CMakeLists.txt` files retain their original
  timestamps.
