# ERRORS.md — differential verification of the C → Rust translation

Scope: `c_src/` (`driver`, built with CMake/gcc) compared against
`translation/` (`driver`, built with `cargo build --release`) by running both
as subprocesses over identical stdin and diffing stdout, stderr and exit
status. Harness: `translation/tests/differential.rs` (33 tests).

## Result

**No mismatches were found.** Every input class enumerated below produced
byte-identical stdout, byte-identical stderr (empty in both) and an identical
exit status (`exit(0)` in every reachable case).

That includes ~40 hand-written cases plus 20 deterministic fuzz corpora
(token-level, raw random bytes, and numeric-alphabet bytes — 3–4 KB each,
seeded via SplitMix64 so they are reproducible).

## Harness validity (negative control)

A test suite that passes vacuously proves nothing, so the harness was
deliberately broken three times and confirmed to fail, once per compared
channel:

| Injected defect in `translation/src` | Result |
|---|---|
| `"Price: ${:.2}"` → `"${:.3}"` | 13 / 33 tests fail with `STDOUT mismatch` |
| `std::process::exit(0)` → `exit(1)` on menu choice 7 | 17 / 33 fail with `EXIT STATUS mismatch (C=exit(0) Rust=exit(1))` |
| `eprintln!("noise")` at the top of `main` | 32 / 33 fail with `STDERR mismatch` |

All three injections were reverted and the sources verified identical to their
pre-injection state before the final run.

## Input classes enumerated from the C source

`main()` is a menu loop: `fgets(input, 256, stdin)` → `sscanf(input, "%d",
&choice)` → `switch (choice)`. The branch points and their coverage:

| C branch | Input class | Test |
|---|---|---|
| `if (!fgets(...)) break;` | empty stdin; EOF after a complete line; EOF mid-line | `empty_input_...`, `missing_trailing_newline_at_eof` |
| `if (sscanf(...) != 1)` → `"Invalid input"` | blank line, letters, sign only, `.5`, whitespace only, `\r\n`, punctuation, leading NUL | `sscanf_matching_failure_invalid_input`, `embedded_nul_bytes_...` |
| `case 1` … `case 5` | `1\n` … `5\n` | `menu_choice_1..5_*` |
| `case 6` (runs all five demos) | `6\n` | `menu_choice_6_run_all_demos` |
| `case 7` — the only early `return` | `7\n`, and `7\n` followed by more input | `menu_choice_7_exits_with_goodbye`, `menu_choice_7_stops_reading_remaining_input` |
| `default` → `"Invalid choice"` | `0`, `8`, `9`, `-1`, `-7`, `-0`, `123456` | `default_case_invalid_choice` |
| loop `continue` after an error | invalid line followed by a valid one | `invalid_input_then_valid_choice_continues_loop` |
| array growth (`size >= capacity` → `capacity * 2`) | `2\n` — `array_double_create(5)` then 7 pushes | `menu_choice_2_double_containers` |

## Behaviours that were specifically probed as likely divergence points

These are the places a translation usually drifts. Each was tested and each
already matched — they are recorded so a later reader can re-check them rather
than rediscover them.

1. **`fgets` truncation at 255 bytes, remainder retained.** `char input[256]`
   means an over-long line is *split*, not discarded. The sharpest case:
   254 spaces + `"77\n"` — `fgets` returns 254 spaces plus the *first* `7`, so
   the choice is 7 and the program exits without ever seeing the second `7`.
   At 253 spaces the same bytes parse as `77` → `"Invalid choice"`. Also
   covered: a 255-byte payload leaving the `\n` behind as an extra empty line,
   and 300 zeros followed by `7` (first line → choice 0, remainder → choice 7).
   `stdio::fgets` reproduces all of this.

2. **`sscanf("%d")` overflow.** glibc converts via `strtol` and, on overflow,
   saturates to `LONG_MAX`/`LONG_MIN` (setting `ERANGE`) before the result is
   truncated to `int`. It does **not** wrap modulo 2^64. Discriminating cases:
   `18446744073709551617` (2^64+1) and `18446744073709551621` (2^64+5) —
   wrapping would select demo 1 and demo 5, saturation yields `(int)-1` →
   `"Invalid choice"`. Both programs print `"Invalid choice"`.
   `stdio::sscanf_int` matches by saturating to `i64::MIN/MAX` and then
   casting `as i32`.

3. **`int` truncation of in-range `long` values.** `4294967303` (2^32+7)
   truncates to 7 and *exits*; `8589934598` (2·2^32+6) truncates to 6 and runs
   every demo; `2147483648` truncates to `INT_MIN`. All match.

4. **`sscanf` lexical details.** Leading whitespace is skipped (including
   `\v` and `\f`), an optional `+`/`-` is accepted, leading zeros are ignored,
   and conversion stops at the first non-digit (`3abc` → 3, `6.9` → 6,
   `0x10` → 0, `1 2` → 1). All match.

5. **Embedded NUL bytes.** `fgets` copies a NUL through as an ordinary byte,
   after which `sscanf` sees a C string that ends there. `"\0" "7\n"` is an
   input failure; `"7\0\n"` still converts to 7. Both match.

6. **Invalid UTF-8 on stdin and non-ASCII in output.** Input is treated as raw
   bytes on both sides (`0xff 0xfe`, lone high bytes, full-width `７`).
   Output-side: the `°` in demo 2 and the box-drawing banner in `main` are
   multi-byte UTF-8 in the C source and are emitted as the same bytes by the
   Rust build. Item names are printed through `out_raw` on the NUL-terminated
   byte buffer, so no transcoding can occur.

7. **`printf("%.2f")` vs Rust `{:.2}` rounding.** Verified independently
   against tie-prone values (`0.125`, `2.675`, `1.005`, `0.5`, `1.5`, `2.5`,
   `8.995`, …) at both `%.2f` and `%.1f`: glibc and Rust produce identical
   digits, because both round the exact binary value half-to-even. No
   divergence risk here.

8. **The `max_price` quirk in `calculate_inventory_stats`.** C seeds
   `max_price = 0.0` while seeding `min_price` from `items->data[0].price`.
   This is asymmetric and would report `0.00` as the maximum for an
   all-negative-price inventory. The Rust version reproduces it verbatim
   (`inventory.rs`) rather than "fixing" it to seed from element 0.

## Paths that no input can reach through the executable

Several `inventory.c` branches are dead with respect to `main()`, so a
black-box differential run cannot cover them. They were checked by reading the
two sources side by side instead; each is a faithful translation.

- `calculate_inventory_stats` `size == 0` → `"No items in inventory"`, and
  `calculate_order_stats` `size == 0` → `"No orders to analyze"`. `main()`
  always populates 10 items / 8 orders first.
- `find_items_by_category` `found == 0` → `"No items found in this category"`.
  `main()` only ever queries `"Electronics"` and `"Furniture"`, which both
  match.
- `find_expensive_items` is never called from `main()` at all — neither
  program can print `"=== Items priced above $…"` or `"No items found above
  this price"`.
- `list_*_prepend`, `list_*_clear`, `array_*_clear`, `array_*_get` (beyond
  index 0), and `array_*_create(0)`'s capacity-16 fallback are never invoked.
- `strncpy` truncation in `create_item` / `create_order` never triggers: every
  literal name is well under `MAX_NAME_LENGTH - 1` / `MAX_CATEGORY_LENGTH - 1`.

## Known non-divergences by construction (not defects)

- **Null-pointer guards.** C checks `if (!items || !category) return;` and
  `if (!arr) return -1;`. Rust uses `&`/`&mut` references, which cannot be
  null, so those guards are absent. Unreachable in C too, since `main()` never
  passes a null pointer.
- **Allocation failure.** C's `array_*_create`/`push` return `NULL`/`-1` on
  `malloc` failure (and `main()` ignores the result, which would then
  segfault). Rust aborts on allocation failure. Only observable under OOM,
  which is not an input class.
- **stdout buffering.** C's stdout is block-buffered when piped; the Rust
  translation accumulates output in a `Vec<u8>` and flushes before each
  `fgets` and at exit. The interleaving with stdin is therefore the same, and
  the concatenated stdout byte stream is identical. stderr is unused by both.
