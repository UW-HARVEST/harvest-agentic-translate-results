# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`: every *valid* configuration the C actually branches on.

## Axes, derived from the `if`/`switch`/comparison sites in `c_src/src/lib.c`

This library has **no runtime option struct, no flags, no modes, no `#ifdef`s**.
Grepping the source for conditionals yields exactly these branch axes:

* **A1 — `get_operation_name` `switch (op_code)`** (line 85): 5 arms —
  `0`→`"add"`, `1`→`"subtract"`, `2`→`"multiply"`, `3`→`"divide"`, `default`→`"unknown"`.
* **A2 — `perform_operation` string dispatch** (lines 95–107): 6 outcomes —
  `add`, `subtract`, `multiply`, `divide`/`b!=0`, `divide`/`b==0`, no-match.
  The selector is a *string*, so it can be driven independently of A1.
* **A3 — `append_to_buffer` growth branch** (line 57
  `required_capacity > buffer->capacity`): grow vs. no-grow.
* **A4 — `append_to_buffer` string shape**: empty string, 1 byte, many bytes,
  exactly-fits, one-past-fits; and `buffer->length` 0 vs. >0 (append into an
  empty vs. a partially filled buffer) — the `strcpy(data + length, str)` offset.
* **A5 — `create_buffer` capacity shape**: `0`, `1`, small (`< 1` string),
  `32` (what `buffapp` uses), large, `INT_MAX`-ish.
* **A6 — `buffapp` `param1 % 4`**: C `%` truncates toward zero, so the operand
  sign matters: `{0,1,2,3}` select real ops, `{-1,-2,-3}` fall through to
  `"unknown"`. 7 distinct classes.
* **A7 — `buffapp` `param3 % 4`**: same 7 classes, independent of A6.
* **A8 — `buffapp` `intermediate3 != 0`** (line 141): divide-result branch vs.
  `p1+p2+p3+p4` fallback branch.
* **A9 — element/byte shape of the observable output**: the `int` return value,
  the `StringBuffer` fields (`capacity`, `length`), the NUL-terminated `data`
  bytes, and the `printf` byte stream on stdout.

Entry points covered: **all six** — the lowest-level `create_buffer`,
`append_to_buffer`, `destroy_buffer`, `get_operation_name`, `perform_operation`,
*and* the one-shot wrapper `buffapp` (the only symbol in the public header).
Rows 1–24 drive the low-level API directly; rows 25–40 drive `buffapp`.

Each row is run with **many randomized inputs** from a fixed-seed
(`SEED = 0x5EED_1234_ABCD_EF01`) xorshift64* PRNG plus the hand-picked boundary
values named in the row, and is checked off only when every draw matches
byte-for-byte between the two `.so`s.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `get_operation_name` | A1 arm `0` (`"add"`) — compare returned C string bytes | [x] |
| 2 | `get_operation_name` | A1 arm `1` (`"subtract"`) | [x] |
| 3 | `get_operation_name` | A1 arm `2` (`"multiply"`) | [x] |
| 4 | `get_operation_name` | A1 arm `3` (`"divide"`) | [x] |
| 5 | `get_operation_name` | A1 `default` — 4, 100, `INT_MAX`, negatives, `INT_MIN`, 4096 random ints | [x] |
| 6 | `get_operation_name` | pointer *stability*: same op_code twice returns the same pointer; distinct arms return distinct pointers (static-storage semantics) | [x] |
| 7 | `perform_operation` | A2 `"add"` × random `(a,b)` incl. `INT_MAX/INT_MIN` boundaries (A2×overflow) | [x] |
| 8 | `perform_operation` | A2 `"subtract"` × random `(a,b)` incl. boundaries | [x] |
| 9 | `perform_operation` | A2 `"multiply"` × random `(a,b)` incl. boundaries | [x] |
| 10 | `perform_operation` | A2 `"divide"`, `b != 0`, random `(a,b)` — truncation toward zero for all 4 sign combinations | [x] |
| 11 | `perform_operation` | A2 `"divide"`, `b != 0`, `a` or `b` at `INT_MIN`/`INT_MAX`/`±1` (excluding the `INT_MIN/-1` trap, which is `ERRORS.md` #16) | [x] |
| 12 | `perform_operation` | A2 `"divide"` with `b == 0` → 0 (also in `ERRORS.md` #15; kept here as a valid-input row) | [x] |
| 13 | `perform_operation` | operation string taken from `get_operation_name`'s own return pointer (composed pipeline, not a literal) — all 5 arms | [x] |
| 14 | `create_buffer` + `destroy_buffer` | A5 `capacity == 1`; check `capacity`/`length`/`data[0]=='\0'`, then destroy | [x] |
| 15 | `create_buffer` + `destroy_buffer` | A5 `capacity == 32` (the `buffapp` value) | [x] |
| 16 | `create_buffer` + `destroy_buffer` | A5 large capacity (`1<<20`) and random capacities in `1..=65536` | [x] |
| 17 | `create_buffer` + `append_to_buffer` | A3 no-grow: `capacity` large, single short append (`required <= capacity`) — assert `capacity` unchanged, `length`, `data` bytes | [x] |
| 18 | `create_buffer` + `append_to_buffer` | A3 grow: `capacity` small, single long append — assert the *exact* `new_capacity == required*2` and `data` bytes | [x] |
| 19 | `create_buffer` + `append_to_buffer` | A3/A4 exact boundary: `str_len == capacity - 1` (fits, no grow) vs `str_len == capacity` (grows) — swept over capacities `1..=64` | [x] |
| 20 | `create_buffer` + `append_to_buffer` | A4 empty string `""` — `str_len == 0`, `required == length+1`; from `length==0` and from `length>0` | [x] |
| 21 | `create_buffer` + `append_to_buffer` | A4 many appends (100+) of random-length random-byte strings into one buffer: diffs the whole `capacity`/`length`/`data` trajectory step by step (the composed pipeline) | [x] |
| 22 | `create_buffer` + `append_to_buffer` | A4 append into a buffer whose `length` was externally rewound to 0 (what `buffapp` line 116 does) — overwrite semantics | [x] |
| 23 | `create_buffer` + `append_to_buffer` | A4 append into a buffer whose `length` was externally set to a value `< capacity` but `> 0` (mid-buffer `strcpy` offset, leaves stale bytes before it) | [x] |
| 24 | `create_buffer` + `append_to_buffer` + `destroy_buffer` | full low-level lifecycle, randomized capacity + randomized append script, ending in `destroy_buffer`; heap pointer from C freed by C, from Rust freed by Rust | [x] |
| 25 | `buffapp` | A6=0 × A7=0 (`add`,`add`), random `p2`,`p4` | [x] |
| 26 | `buffapp` | A6=1 × A7=1 (`subtract`,`subtract`) | [x] |
| 27 | `buffapp` | A6=2 × A7=2 (`multiply`,`multiply`) | [x] |
| 28 | `buffapp` | A6=3 × A7=3 (`divide`,`divide`), incl. `p2==0` / `p4==0` zero-divisor sub-case | [x] |
| 29 | `buffapp` | full A6×A7 cross-product of the 4 non-negative classes (16 combos), randomized operands | [x] |
| 30 | `buffapp` | A6 negative (`p1%4 ∈ {-1,-2,-3}`) × A7 non-negative — `"unknown"` for op1 | [x] |
| 31 | `buffapp` | A6 non-negative × A7 negative — `"unknown"` for op2 | [x] |
| 32 | `buffapp` | A6 negative × A7 negative — both `"unknown"`, so `i1==i2==0`, `i3==0` → A8 fallback | [x] |
| 33 | `buffapp` | full A6×A7 cross-product over all 7×7 = 49 residue classes, randomized operands per class | [x] |
| 34 | `buffapp` | A8 divide branch: `intermediate3 != 0` | [x] |
| 35 | `buffapp` | A8 fallback branch: `intermediate3 == 0` reached via `i1 == 0` | [x] |
| 36 | `buffapp` | A8 fallback branch reached via `i2 == 0` | [x] |
| 37 | `buffapp` | A9 stdout: the full `"Computation Log:\n%s\n"` byte stream captured via `dup2` and diffed byte-for-byte, across all the above classes | [x] |
| 38 | `buffapp` | A9 widest `sprintf` output: all four params at `INT_MIN`/`INT_MAX` (longest `%d` renderings, largest `temp[64]` fill) | [x] |
| 39 | `buffapp` | boundary params: every combination drawn from `{INT_MIN, INT_MIN+1, -4,-3,-2,-1, 0, 1,2,3,4, INT_MAX-1, INT_MAX}` (4-way sweep, sampled) | [x] |
| 40 | `buffapp` | 4096 fully-random `(p1,p2,p3,p4)` quadruples — return value *and* stdout bytes | [x] |
| 41 | `create_buffer` | A5 large *positive* capacity where `malloc` still succeeds: `1<<30`, `INT_MAX-1`, `INT_MAX` (the largest non-negative `int`) | [x] |
| 42 | `create_buffer` + `append_to_buffer` | A3 grow branch where `new_capacity` is large but still **positive**, so `realloc` succeeds instead of failing (the mirror of `ERRORS.md` #6/#7): `length` around `1e8`, `required*2` ≈ 2·10⁸ | [x] |
| 43 | all three buffer entry points | many buffers alive **simultaneously**, interleaved create / append / destroy across both libraries (allocator interop, no shared global state) | [x] |
| 44 | `create_buffer` (lib X) + `append_to_buffer` / `destroy_buffer` (lib Y) | **cross-library handoff**: a buffer created by C is grown and freed by Rust and vice versa. Both must use the very same libc allocator for this to work, which is what the translation claims | [x] |
| 45 | `buffapp` | called repeatedly in a long sequence (no residual state between calls; return value and stdout identical on every repetition) | [x] |

## Optimisation levels

The C is ground truth as built by `c_src/CMakeLists.txt`, which sets no
`CMAKE_BUILD_TYPE` and therefore compiles at `-O0`. The UB-dependent outcomes
(`INT_MIN / -1` → SIGFPE, wrapping signed overflow) were additionally confirmed
identical for `gcc -O0`, `-O2` and `-O3`, so the translation's choices are not
tied to one optimisation level.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo test --no-default-features` is
equivalent to `cargo test`. Both are run by `./run_all.sh` for completeness.

## Row → test mapping (auditable)

Every row above is checked off because a named test exercises it. Test names
carry their row number, so the mapping can be re-derived mechanically:

```
grep -h '^fn row' tests/phase_b_*.rs
```

| rows | test file | test functions |
|------|-----------|----------------|
| 1–6   | `tests/phase_b_lowlevel.rs` | `row01_op_name_add` … `row06_op_name_pointer_stability` |
| 7–13  | `tests/phase_b_lowlevel.rs` | `row07_perform_add` … `row13_perform_with_name_from_get_operation_name` (+ `row13b_perform_unmatched_operation_strings`) |
| 14–24 | `tests/phase_b_lowlevel.rs` | `row14_create_capacity_one` … `row24_full_lowlevel_lifecycle_randomized` |
| 25–40 | `tests/phase_b_buffapp.rs`   | `row25_buffapp_add_add` … `row40_buffapp_bulk_random` |
| 41–45 | `tests/phase_b_extra.rs`     | `row41_create_buffer_large_positive_capacities` … `row45_buffapp_repeated_calls_are_stateless` |

All randomised rows draw from the fixed seed `SEED = 0x5EED_1234_ABCD_EF01`
(xorshift64\*, `tests/common/mod.rs`), so every run is reproducible.
