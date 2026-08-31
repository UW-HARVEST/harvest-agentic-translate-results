# Differential verification log — `c_src/src/main.c` → `translation/`

## What the two programs are

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) printf("%d %d\n", i, j);
}
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

Exactly two input-dependent behaviours exist:

1. whether `scanf("%d", &x)` assigns (its return value is discarded, so a failed
   conversion silently leaves `x == 0`);
2. the sign and magnitude of the resulting `int`, which `driver` tests once via
   `i < x` before the first iteration.

`main` always `return 0`, and neither function writes to stderr — so for every
input except a closed stdout, the expected result is empty stderr and exit 0.

## How it is verified

`translation/tests/differential.rs` builds/locates both executables and runs
them as subprocesses over the same stdin bytes, comparing **stdout, stderr and
exit status**. Nothing is linked as a library.

- C binary: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver` (the test builds it automatically if absent)
- Rust binary: `cd translation && cargo build --release`
  → `translation/target/release/driver`
- Test suite: `cd translation && cargo test` — 24 tests, no `#[ignore]`, no skips.

`c_src/` was only read and built out-of-tree; no file under it was modified.

---

## Mismatches found

### 1. `SIGPIPE`: Rust exited 0 where C died by signal 13 — FIXED

The only genuine behavioural divergence found.

The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs, so a write
to a closed stdout returns `EPIPE`. The original translation discarded write
errors (`let _ = writeln!(...)`), so it ran the loop to completion and exited 0.
The C program inherits the default disposition and is killed by the signal.

Observed before the fix, with a reader that closes after a few bytes:

```
$ printf 2000000 | ./c_src/build/driver            | head -c 5   # → status 141
$ printf 2000000 | ./translation/target/release/driver | head -c 5   # → status 0
```

**Cause:** Rust-runtime-specific signal setup that has no counterpart in the C
program, not a logic error in the translated loop or parser.

**Fix:** `restore_default_sigpipe()` in `translation/src/main.rs` resets
`SIGPIPE` to `SIG_DFL` as the first statement of `main`, via a bare
`extern "C" { fn signal(...) }` declaration so no dependency is added. Both
programs now report `(code: None, signal: Some(13))`.

**Regression test:** `closed_stdout_kills_both_the_same_way`.

### 2. `values_at_the_int_boundaries` was untestable as written — TEST RESTRUCTURED

Not a translation defect, but it made the suite unusable. The input
`-2147483649` converts (as a `long`) to `-2147483649` and *then* narrows to
`int`, landing on `INT_MAX`. Both programs therefore emit 2 147 483 647 lines
(≈47 GiB). Capturing that into memory made `cargo test` take **529 seconds**.

The `INT_MAX` inputs (`2147483647`, `-2147483649`) moved to
`int_max_is_the_maximum_the_code_handles`, which compares the two stdout streams
in **lockstep, byte for byte, over a bounded 8 MiB prefix** (O(1) memory) and
additionally asserts neither program terminated early. Full runtime is now ~2 s.
This is the one place the comparison is a prefix rather than the whole stream;
every other case is compared in full.

---

## Behaviours confirmed identical (no fix needed)

These were checked because they are the places a translation usually drifts.

| Behaviour | C semantics | Status |
| --- | --- | --- |
| Failed conversion | return value ignored ⇒ `x` stays `0` ⇒ no output | matches |
| EOF / whitespace-only input | `scanf` returns `EOF`, `x` stays `0` | matches |
| Whitespace skipping | `scanf` crosses newlines (unlike `fgets`); set is `" \t\n\r\v\f"` | matches |
| `x <= 0` | `i < x` false on entry ⇒ zero lines, still exit 0 | matches |
| Explicit `+` sign | accepted by `%d` | matches |
| Leading zeros | decimal, never octal (`007` → 7) | matches |
| `0x10`, `1e3`, `2.9` | conversion stops at the first non-digit (`0`, `1`, `2`) | matches |
| Trailing input | only one conversion; the rest of stdin is never read | matches |
| Overflow clamping | glibc `%d` converts via `long`, saturating at `LONG_MAX`/`LONG_MIN` | matches |
| Narrowing to `int` | clamped `long` truncated mod 2³²: `4294967297`→1, `2147483648`→`INT_MIN`, `-4294967295`→1, 300-digit input→`LONG_MAX`→`-1` | matches |
| Non-UTF-8 stdin | byte-oriented; `\xff`, `\x00`, lone continuation bytes | matches |
| `printf("%d %d\n")` | single space separator, trailing `\n`, no padding across digit widths | matches |
| stderr | never written | matches (empty for both, all cases) |
| Exit status | always 0 unless signalled | matches |

### Unreachable in test time: `j` overflow

`j == 2*i`, so `j` only overflows `int` once `i` passes 1 073 741 824 — over a
billion `printf` calls, which cannot be run in a test. The Rust code uses
`wrapping_add(2)`, which matches what the compiled C actually does at both
optimisation levels inspected:

- `cmake ..` (no build type, `-O0`): `addl $0x2, -0x8(%rbp)` — 32-bit slot, wraps
- `-DCMAKE_BUILD_TYPE=Release`: gcc eliminates `j` and recomputes it as
  `lea (%rbx,%rbx,1),%edx`, i.e. 32-bit `2*i`, which also wraps

Both are equal to `2*i mod 2³²` reinterpreted as `int`, which is what the Rust
accumulator produces. Signed overflow is UB in C, so this rests on the observed
codegen rather than the standard.

## Input classes covered

empty · whitespace only (space, tab, newline, CR, VT, FF) · `0` · `-0` · `+0` ·
`1` · every `x` in `-5..=40` · `2..=12` · negatives down to `INT_MIN` ·
explicit `+` · leading whitespace incl. newlines · leading zeros · digit-width
crossings (6, 51, 501, 5001) · 100 000 lines compared in full · `INT_MAX`
(bounded prefix) · `2^31`, `2^31+1`, `-2^31`, `2^32`, `2^32±k`, `2*2^32+2` ·
`LONG_MAX`, `LONG_MAX+1`, `LONG_MIN`, `LONG_MIN-1`, `10^19`, `2^64`, `2^64+1`,
26- and 300-digit magnitudes (both signs) · 1000 leading zeros · non-numeric
(`abc`, `.5`, `!!!`, `,5`, `#5`, Arabic-Indic `٥`) · bare `-`, bare `+`, `-a`,
`+-3`, `- 3` · trailing garbage (`3abc`, `3 99`, `0x10`, `1e3`, `2.9`, `4-5`) ·
non-UTF-8 bytes (`\xff`, `\x00`, `\x80\x81\x82`, truncated UTF-8) · closed
stdout mid-stream · 300-case deterministic randomised sweep.
