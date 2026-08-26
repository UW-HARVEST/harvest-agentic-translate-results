# VERIFICATION.md — completion gate

Differential verification of the Rust translation in `src/` against the C ground
truth in `c_src/src/main.c`.

## Nature of the target (why tests are process-level)

`c_src/CMakeLists.txt` declares `add_executable(driver src/main.c)`. The C target
is a **program**, not a shared library: it exports **zero** dynamic symbols and
its only worker function is `static void foo(int, int)`. There is no `.so` and no
FFI entry point to `dlopen`, so the equivalent of "load both artifacts and compare
through the boundary" is to run both artifacts as processes across the boundary
they actually expose — stdin, stdout, stderr, exit status and fatal signal — and
compare byte for byte. No Rust function is ever called directly from a test;
every comparison goes through the built `driver` artifacts. `libloading` is a
dev-dependency as required and is used by `tests/symbols.rs` to assert that
neither artifact exposes a loadable symbol surface.

## Build configurations (Phase A)

* `Cargo.toml` has **no `[features]` table** → the power set of features is the
  single empty combination. `run_all_configs.sh` enumerates it mechanically from
  the file rather than assuming, so it keeps working if features are added.
* `c_src/` has no `option()`, no `add_definitions`, and `grep -c '#if' → 0` in
  `main.c` → exactly one C build configuration.
* `[profile.release] panic = "abort"` makes the release binary a genuinely
  different artifact, so the whole suite is run for **dev and release**.

## Artifacts

| file | contents |
|---|---|
| `SYMBOLS.md` | every public symbol from `nm -D` on both artifacts, plus the diff |
| `ERRORS.md` | error-surface table: 23 rows, one per distinct rejection in the C |
| `CONFIGS.md` | configuration-surface table: 29 rows of option × input-shape combinations |
| `tests/common/mod.rs` | differential harness (process runner, prefix comparison, seeded PRNG) |
| `tests/differential.rs` | Phase B — one test per `CONFIGS.md` row |
| `tests/errors.rs` | Phase C — one test per `ERRORS.md` row + generic boundaries |
| `tests/symbols.rs` | Phase D — symbol parity |
| `run_all_configs.sh` | runs `cargo check` + the full suite for every configuration |

## Divergences found and fixed

All three were real translation bugs; the C was never modified.

1. **`SIGPIPE` disposition** (`ERRORS.md` E19). The Rust runtime installs
   `SIG_IGN` for `SIGPIPE` before `main`, so when the reader of stdout went away
   the Rust program ignored `EPIPE` and exited **0** while the C died from
   **signal 13** (wait status 141). Fixed by restoring the default disposition at
   the top of `main`.
2. **stdin was slurped** (`ERRORS.md` E22). The translation used `read_to_end`,
   so `/dev/zero` took ~2.2 s and gigabytes of memory where the C exits in 4 ms.
   Fixed by reading on demand with a `peek`/`bump` scanner that never consumes a
   byte it does not convert.
3. **stdin over-consumption on a shared descriptor** (`ERRORS.md` E22). Even after
   fix 2, `std::io::stdin()`'s internal 8 KiB `BufReader` left a shared seekable
   stdin drained 8192 bytes deep, where the C leaves the offset at exactly the
   last byte a conversion needed (glibc's exit-time stream cleanup seeks the unused
   tail back). Measured with `{ ./driver; cat; } < file`: C consumed 3 bytes, Rust
   consumed 8192. Fixed by reading fd 0 directly in 4096-byte chunks and giving
   the unused tail back with `lseek(SEEK_CUR)`.

## Test-suite validity (mutation testing)

Passing tests only mean something if they can fail. Six deliberate mutations were
injected into `src/main.rs` one at a time; **all six were caught**:

| mutation | caught by |
|---|---|
| `if x < 3` → `if x <= 3` (back-edge boundary) | Phase B |
| `x == 1 && y == 4` special case removed (`goto label2`) | Phase B |
| `%d` conversion wraps instead of saturating at the `long` limits | Phase B/C |
| `SIGPIPE` left ignored | `e19_sigpipe_kills_writer` |
| stdin read through std's 8 KiB `BufReader` | `e22_unbounded_stdin_not_drained` |
| `lseek` give-back at exit removed | `e22_unbounded_stdin_not_drained` |

## Completion gate

- [x] **`SYMBOLS.md`**: `nm -D` diff (C defined − Rust defined) is empty, and the
      Rust artifact has **0** unresolved non-libc symbols. The C exports no
      dynamic symbols at all, and the one C translation unit is fully translated —
      no stubs, no `unimplemented!()`, no `todo!()` in `src/`.
- [x] **Phase B**: all **29** `CONFIGS.md` rows pass across randomized inputs
      (fixed-seed xorshift64\*, one seed per row).
- [x] **Phase C**: all **23** `ERRORS.md` rows have a passing error-path
      differential test asserting the *same* rejection (same exit status/signal,
      same bytes), plus 4 generic boundary tests covering absent/empty inputs,
      oversized inputs, and every possible byte value `0x00`–`0xff` in both the
      leading and the separator position.
- [x] **Every configuration**: the single feature combination × {dev, release}
      profiles, all green (`./run_all_configs.sh` → `RESULT: all configurations
      passed`).

```
$ ./run_all_configs.sh
Feature combinations discovered: 1
  - '<none>'
### cargo test  features='<none>' profile=dev      -> 29 + 27 + 4 passed, 0 failed
### cargo test  features='<none>' profile=release  -> 29 + 27 + 4 passed, 0 failed
OK: target/debug/driver   exports every symbol the C .so exports (diff empty)
OK: target/release/driver exports every symbol the C .so exports (diff empty)
OK: 0 unresolved non-libc symbols
RESULT: all configurations passed
```

## Known limit of finite testing

Distinguishing `y--`'s wrap at `y == INT_MIN` from a saturating decrement would
require observing more than 2^32 output lines (~8 GiB): both produce the identical
unbounded `"y\n"` stream before that point. The Rust uses `wrapping_sub`, which is
what the C compiles to at `-O0`, and every finite prefix of the two agrees. This
is the only equivalence not reachable in finite test time; it is stated here
rather than left as an implicit gap.
