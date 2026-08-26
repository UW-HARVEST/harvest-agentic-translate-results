# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
`c_src/src/lib.c` actually branches on.

## Build-time axes

There are **none**. `Cargo.toml` has no `[features]`; `lib.c`/`lib.h` have no
`#if`/`#ifdef`/`#define`; `CMakeLists.txt` has no `option()` or compile
definitions. The single build configuration is the default one, which is also
identical to `--no-default-features`. (See `SYMBOLS.md`.)

## Runtime axes actually branched on by the C

| axis | values the C distinguishes | where |
|---|---|---|
| A1 `initial_capacity` | `> 0` (normal) / `== 0` (`malloc(0)` + 1-byte OOB store) / `< 0` (huge `size_t`, fails) | `create_buffer` L40–48 |
| A2 `required_capacity` vs `capacity` | `<` , `==` (no realloc — boundary) / `>` (realloc to `required*2`) | `append_to_buffer` L57 |
| A3 `buffer->length` at entry | `0` / mid-buffer / **externally forced** (as `buffapp` itself does at L116) | `append_to_buffer` L55, L69, L70 |
| A4 `strlen(str)` | `0` (empty) / `1` / fits exactly / overflows capacity / long (multi-realloc) | `append_to_buffer` L54 |
| A5 append count | 0 / 1 / many (realloc chain, `data` pointer moves) | pipeline |
| A6 cross-library ownership | buffer created in C & destroyed by Rust, and vice-versa (both use the same glibc heap) | `create_buffer`/`destroy_buffer` |
| A7 `op_code` | `0` `1` `2` `3` (the four `case`s) / anything else (`default`) | `get_operation_name` L85–91 |
| A8 `operation` string | `"add"` / `"subtract"` / `"multiply"` / `"divide"` / no match — matched by `strcmp`, so **exact bytes** matter | `perform_operation` L95–107 |
| A9 `b` for `"divide"` | `!= 0` / `== 0` | `perform_operation` L102 |
| A10 operand magnitude | `0`, `±1`, small, `INT_MAX`, `INT_MIN`, values whose `+`/`-`/`*` **wrap** `int` | `perform_operation` L96–100 |
| A11 `param1 % 4` | `0,1,2,3` (positive params) **and** `-1,-2,-3` (C `%` truncates toward zero ⇒ `default` ⇒ `"unknown"`) | `buffapp` L121 |
| A12 `param3 % 4` | same 7 residue classes | `buffapp` L128 |
| A13 `intermediate3` | `!= 0` → `result / intermediate3` / `== 0` → 4-way sum fallback | `buffapp` L141–145 |
| A14 decimal width of the `%d`s | 1 digit … 11 chars (`-2147483648`) — changes every log line length, hence the realloc schedule of the log buffer and the exact `printf` bytes | `buffapp` L118–150 |
| A15 observable output | `int` return value **and** the bytes written to `stdout` by `printf("Computation Log:\n%s\n", …)` | `buffapp` L150 |

Everything below is the cross-product of those axes, pruned to the
combinations the code treats differently. All rows use **randomized inputs with
a fixed seed** (`SEED = 0x243F6A8885A308D3`, SplitMix64) unless the row names a
specific boundary value; every row compares C vs Rust through `dlopen`ed
exports only.

## Rows — `create_buffer` (+ struct-field inspection, + `destroy_buffer`)

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|------------------------------------------|-----|
| C1 | `create_buffer` → read `{data,capacity,length,data[0]}` → `destroy_buffer` | `initial_capacity == 0` (A1 zero; `malloc(0)` + OOB store) | [x] |
| C2 | same | `initial_capacity == 1` (exactly holds the NUL) | [x] |
| C3 | same | `initial_capacity == 2` | [x] |
| C4 | same | `initial_capacity == 31 / 32 / 33` (the value `buffapp` uses, ±1) | [x] |
| C5 | same | `initial_capacity` = 200 randomized values in `1..=4096` | [x] |
| C6 | same | `initial_capacity == 65536` / `1<<20` (large but allocatable) | [x] |
| C7 | `create_buffer` ×N then `destroy_buffer` ×N | 64 live buffers interleaved (allocator state / no cross-talk) | [x] |
| C8 | C `create_buffer` → Rust `append_to_buffer` → Rust `destroy_buffer` | A6 cross-library ownership, capacity 8 | [x] |
| C9 | Rust `create_buffer` → C `append_to_buffer` → C `destroy_buffer` | A6 reverse direction, capacity 8 | [x] |
| C10 | `destroy_buffer` | freshly created buffer, `data` non-NULL (normal happy path) | [x] |

## Rows — `append_to_buffer` (lowest-level entry point, driven directly)

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|------------------------------------------|-----|
| C11 | `create_buffer` + `append_to_buffer` | cap 16, `str == ""` (A4=0) from `length==0` ⇒ `required=1 < cap`, no realloc | [x] |
| C12 | same | cap 16, 1-char string, `length==0` | [x] |
| C13 | same | cap 16, `strlen == 15` ⇒ `required == 16 == capacity` (A2 `==`, **no** realloc — boundary) | [x] |
| C14 | same | cap 16, `strlen == 16` ⇒ `required == 17 > capacity` (A2 `>`, realloc to 34) | [x] |
| C15 | same | cap 1, `strlen == 0` ⇒ `required == 1 == cap`, no realloc | [x] |
| C16 | same | cap 0, `strlen == 0` ⇒ `required == 1 > 0`, realloc to 2 | [x] |
| C17 | same | cap 4, `strlen == 4096` (single huge grow) | [x] |
| C18 | `create_buffer` + `append_to_buffer` ×K | cap 1, 64 sequential appends of random 0..17-byte strings (A5 many; realloc chain, `data` moves, `capacity` schedule must match exactly) | [x] |
| C19 | same | cap 4096, 64 sequential appends that never realloc (A2 always `<`) | [x] |
| C20 | same | cap 32, 200 appends of random 0..40-byte strings — mixed realloc/no-realloc schedule | [x] |
| C21 | `create_buffer` + force `length` + `append_to_buffer` | A3 externally forced `length` (exactly what `buffapp` L116 does): `length` set to a random value in `0..capacity` before appending | [x] |
| C22 | same | A3 forced `length == capacity - 1`, then 0-byte append ⇒ `required == capacity`, no realloc | [x] |
| C23 | same | A3 forced `length == capacity`, then 0-byte append ⇒ `required == capacity+1`, realloc | [x] |
| C24 | `append_to_buffer` | strings containing high-bit / non-ASCII bytes (`0x80..0xFF`) and `%`-like bytes (`strcpy`/`strlen` are byte-exact, no formatting) | [x] |

## Rows — `get_operation_name` (A7)

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|------------------------------------------|-----|
| C25 | `get_operation_name` | `op_code == 0` ⇒ `"add"` | [x] |
| C26 | `get_operation_name` | `op_code == 1` ⇒ `"subtract"` | [x] |
| C27 | `get_operation_name` | `op_code == 2` ⇒ `"multiply"` | [x] |
| C28 | `get_operation_name` | `op_code == 3` ⇒ `"divide"` | [x] |
| C29 | `get_operation_name` | 4096 randomized `i32` values over the whole range (mixes `case`s and `default`) | [x] |

## Rows — `perform_operation` (A8 × A9 × A10)

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|------------------------------------------|-----|
| C30 | `perform_operation` | `"add"` × 2000 random `(a,b)` incl. wrap-around, plus `{0,±1,INT_MAX,INT_MIN}²` grid | [x] |
| C31 | `perform_operation` | `"subtract"` × same input set (incl. `INT_MIN - 1` wrap) | [x] |
| C32 | `perform_operation` | `"multiply"` × same input set (incl. `INT_MIN * -1`, `INT_MAX * INT_MAX` wrap) | [x] |
| C33 | `perform_operation` | `"divide"`, `b != 0` × same input set minus `(INT_MIN,-1)` (E14) | [x] |
| C34 | `perform_operation` | `"divide"`, `b == ±1` (identity / negation boundary) | [x] |
| C35 | `perform_operation` | `"divide"` with truncation-toward-zero cases: `(7,2) (-7,2) (7,-2) (-7,-2) (1,2) (-1,2)` | [x] |
| C36 | `perform_operation` | operation pointer taken from `get_operation_name(0..3)` of the **other** library (pointer provenance is irrelevant, bytes are what matter) | [x] |
| C37 | `perform_operation` | operation string built at runtime on the stack/heap rather than a literal (`"divide"` copied byte-wise) | [x] |

## Rows — `buffapp` (A11 × A12 × A13 × A14; return value **and** stdout bytes)

Residue classes: `0`→`add`, `1`→`subtract`, `2`→`multiply`, `3`→`divide`,
`-1`/`-2`/`-3`→`unknown` (C `%` truncates toward zero). Every row below is
exercised with randomized `param2`/`param4` (and randomized `param1`/`param3`
inside the residue class), and each row samples **both** `intermediate3 == 0`
and `intermediate3 != 0` (A13) wherever reachable. Both the returned `int` and
the exact bytes `printf`ed to `stdout` are compared.

| # | entry point(s) | configuration (options set + input shape) | pass |
|---|----------------|------------------------------------------|-----|
| C38 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C39 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C40 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C41 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C42 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C43 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C44 | `buffapp` | `param1 % 4 == 0` (op1=`add`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C45 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C46 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C47 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C48 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C49 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C50 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C51 | `buffapp` | `param1 % 4 == 1` (op1=`subtract`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C52 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C53 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C54 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C55 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C56 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C57 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C58 | `buffapp` | `param1 % 4 == 2` (op1=`multiply`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C59 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C60 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C61 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C62 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C63 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C64 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C65 | `buffapp` | `param1 % 4 == 3` (op1=`divide`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C66 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C67 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C68 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C69 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C70 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C71 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C72 | `buffapp` | `param1 % 4 == -1` (op1=`unknown`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C73 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C74 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C75 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C76 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C77 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C78 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C79 | `buffapp` | `param1 % 4 == -2` (op1=`unknown`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C80 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == 0` (op2=`add`); randomized params in class, A13 both branches | [x] |
| C81 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == 1` (op2=`subtract`); randomized params in class, A13 both branches | [x] |
| C82 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == 2` (op2=`multiply`); randomized params in class, A13 both branches | [x] |
| C83 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == 3` (op2=`divide`); randomized params in class, A13 both branches | [x] |
| C84 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == -1` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C85 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == -2` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C86 | `buffapp` | `param1 % 4 == -3` (op1=`unknown`) × `param3 % 4 == -3` (op2=`unknown`); randomized params in class, A13 both branches | [x] |
| C87 | `buffapp` | all-zero params `(0,0,0,0)` — op1=`add`, i1=0, i2=0, i3=0 ⇒ A13 fallback, 1-digit widths | [x] |
| C88 | `buffapp` | `(1,1,1,1)` — op1=`subtract` ⇒ i1=0, op2=`subtract` ⇒ i2=0, i3=0 ⇒ fallback | [x] |
| C89 | `buffapp` | `INT_MAX` in every position (A14 max decimal width 10, A10 wrap in `+`/`*`) | [x] |
| C90 | `buffapp` | `INT_MIN` in every position (A14 width 11 = `-2147483648`; `INT_MIN % 4 == 0` ⇒ op=`add`) | [x] |
| C91 | `buffapp` | `(INT_MAX, INT_MAX, INT_MAX, INT_MAX)` and `(INT_MIN, INT_MIN, INT_MIN, INT_MIN)` and all 16 `{INT_MIN,INT_MAX,0,-1}` mixes — log-line lengths at maximum, realloc schedule of the internal 32-byte buffer stressed | [x] |
| C92 | `buffapp` | params chosen so the 4-way fallback sum itself overflows `int` (e.g. `(1,INT_MAX,1,INT_MAX)`) | [x] |
| C93 | `buffapp` | params chosen so `result / intermediate3` truncates toward zero with a negative operand (`i1+i2 < 0`, `i1*i2 > 0`) | [x] |
| C94 | `buffapp` | params chosen so `intermediate3 == 1` and `== -1` (identity/negate of `result`) | [x] |
| C95 | `buffapp` | 20000 fully randomized `(param1,param2,param3,param4)` `i32` quadruples — return value only (fast path) | [x] |
| C96 | `buffapp` | 2000 fully randomized quadruples — return value **and** full stdout byte comparison | [x] |
| C97 | `buffapp` | 1000 quadruples drawn from a small-magnitude domain `-8..=8` (dense coverage of every residue/branch interaction incl. many `intermediate3 == 0`) | [x] |

**Total rows: C1 – C97.**

## Row → test mapping and results

Every row above is checked off because a named case in
`tests/phase_b_valid.rs` exercises it against BOTH `.so` files and asserts
byte-identical results:

| rows | test case(s) |
|---|---|
| C1–C7 | `c1_…` … `c7_create_buffer_many_live_buffers` |
| C8–C10 | `c8_c9_c10_cross_library_ownership` (all 5 create/append/destroy library mixes) |
| C11–C17 | `c11_…` … `c17_…` **plus** `c11_c17_boundary_sweep` = the full 41 × 43 (capacity, strlen) grid, i.e. every position the `required_capacity > capacity` test can straddle |
| C18–C20 | `c18_…`, `c19_…`, `c20_append_chain_mixed_schedule` |
| C21–C23 | `c21_forced_length_random` (300 randomized), `c22_…`, `c23_…` |
| C24 | `c24_append_high_bit_and_format_like_bytes` |
| C25–C29 | `c25_…` … `c29_get_operation_name_randomized` (4 096 randomized codes) |
| C30–C37 | `c30_…` … `c37_perform_operation_runtime_built_name` (each ≈2 100 pairs: a 10 × 10 boundary grid + 2 000 randomized) |
| C38–C86 | `c38_buffapp_r1_0_r3_0` … `c86_buffapp_r1_m3_r3_m3` — the 7 × 7 residue cross-product, 9 randomized samples each (6 mixed-magnitude + 3 that force `intermediate3 == 0`), return value **and** stdout bytes |
| C87–C94 | `c87_…` … `c94_buffapp_intermediate3_plus_minus_one` |
| C95–C97 | `c95_…` (20 000), `c96_…` (2 000 with stdout), `c97_…` (1 000 dense small-domain) |

`residue_helper_is_correct` additionally proves the generator really produces
all seven C-truncating `% 4` classes, so the C38–C86 grid is not silently
degenerate.

### Measured results

```
tests/phase_b_valid.rs :  98 passed;  0 failed
tests/phase_c_errors.rs:  19 passed;  0 failed
```

under **every** build configuration (`--no-default-features` — the only one —
in both the `dev` and `release` profiles; `release` additionally exercises
`panic = "abort"` and full optimisation), and against four independently built
C references (gcc `-O0`/`-O2`/`-O3`, clang `-O2`). See `ERRORS.md` for the
cross-compiler table and the mutation-testing evidence that these rows are not
vacuously green.

### Soak run

`HARVEST_SOAK=<n>` multiplies the C95–C97 sweeps. A `HARVEST_SOAK=50` run was
executed: **1 000 000** randomized `buffapp` return-value comparisons,
**100 000** full-stdout byte comparisons and **50 000** dense small-domain
quadruples — 0 divergences.

Reproduce everything with `./run_verification.sh`.
