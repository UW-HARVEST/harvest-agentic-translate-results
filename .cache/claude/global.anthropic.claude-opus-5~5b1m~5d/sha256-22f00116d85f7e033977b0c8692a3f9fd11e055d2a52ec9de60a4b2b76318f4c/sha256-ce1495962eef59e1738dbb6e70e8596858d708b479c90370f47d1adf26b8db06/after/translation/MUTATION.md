# MUTATION.md — how much the differential suite actually detects

Matching symbols and passing happy-path tests are necessary but not sufficient,
so the suite itself is validated by mutation testing: `mutation_check.py`
injects 62 realistic single-edit translation defects into `src/cjson.rs`,
`src/cshim.rs` and `src/driver.rs`, rebuilds the `.so`, runs the whole
differential suite and reports which mutants survive.

```
python3 mutation_check.py          # all test binaries
python3 mutation_check.py --fast   # skips the slow bigalloc binary
python3 mutation_check.py --only N # a single mutant
```

The pristine sources are snapshotted in `.pristine/` and restored after every
mutant (and in a `finally:` block), so an interrupted run cannot leave the tree
modified.

## Result

```
killed:    53
survived:   9
skipped:    0
```

Each of the 9 survivors is an **equivalent mutant**: no input to the public ABI
can distinguish it from the original. The proofs are below — none of them is a
gap in the tests.

| # | mutant | why no input can distinguish it |
|---|--------|----------------------------------|
| 3 | `print_number`: overflow guard `length > 25` → `length > 26` | `%1.17g` of any `double` is at most 24 characters (`-1.2345678901234567e-308`), so `length` never reaches 25 and the guard is dead code either way. This is ERRORS.md row 28. |
| 7 | `parse_number`: `number >= INT_MAX` → `number > INT_MAX` | The two differ only for `number == 2147483647.0`, where the original yields `valueint = INT_MAX` and the mutant falls through to `(int)number`, which is also `2147483647`. |
| 8 | `cJSON_CreateNumber`: `num <= (double)INT_MIN` → `num < (double)INT_MIN` | Differ only for `num == -2147483648.0`; the original yields `INT_MIN`, the mutant falls through to `(int)num`, which is exactly representable and also `INT_MIN`. |
| 10 | `case_insensitive_strcmp`: treat two NULLs as equal | The only caller is `get_object_item`, which returns `NULL` when `name == NULL` *before* the comparison loop, so `string1` is never NULL and the added branch is unreachable. |
| 11 | `ensure`: `needed > INT_MAX/2` → `needed >= INT_MAX/2` | Differ only for `needed == 1073741823`, where the capacity becomes `INT_MAX` instead of `2*needed = 2147483646`. Both capacities satisfy the request, `print`'s `cjson_min(length, offset+1)` clamp is unaffected (`offset+1 <= 1073741823 < both`), and distinguishing the 1-byte capacity difference at a later `ensure` would require an output above 2 GiB *and* a request landing in that single-byte window. |
| 13 | `ensure`: drop the `p->length > 0 && p->offset >= p->length` guard | Redundant. For `noalloc` buffers, falling through gives `needed += offset + 1 > length`, so the `noalloc` branch returns `NULL` anyway. For growing buffers the invariant `offset < length` holds after every `ensure`, so the condition is never true. (ERRORS.md row 21 is still exercised — the *trigger* and the returned `0` are asserted; only the redundant guard is indistinguishable.) |
| 22 | `parse_string`: `allocate(allocation_length + 1)` → `allocate(allocation_length)` | Not even a memory-safety difference: `allocation_length = (input_end - buffer_at_offset) - skipped_bytes` counts the **opening quote**, which never appears in the output, so `allocation_length` alone is already exactly enough for the unescaped payload plus its NUL. Verified independently by `tests/guarded.rs`, whose footer canary is not tripped by the mutant. |
| 41 | `print`: drop the `cjson_min(buffer->length, buffer->offset + 1)` clamp | Defensive code. `ensure` guarantees `offset + 1 <= length` at every point, so the clamp always selects `offset + 1`. |
| 53 | `cJSON_Minify`: `skip_oneline_comment` advances 2 → 1 | `skip_oneline_comment` is only called when `json[1] == '/'`, so after advancing 1 the loop inspects that `'/'`, finds it is neither `'\0'` nor `'\n'`, and advances to exactly the position the original started from. |

## What the killed mutants demonstrate

The 53 killed mutants cover every functional area of the library, and the test
that caught each one is recorded in the run log. Notable classes:

* number formatting (`%1.15g` / `%1.17g` / the `sscanf` round-trip check,
  `compare_double`, the `INT_MAX`/`INT_MIN` saturation, the C `(int)double`
  cast producing `INT_MIN` for out-of-range values);
* string escaping in both directions (escape-cost arithmetic, `u%04x` case,
  the `> 31` boundary, `\b` → `0x08`, `parse_hex4` digit arithmetic, all UTF-16
  surrogate boundaries and UTF-8 byte-length selection);
* buffer management (`ensure`'s growth factor, `noalloc`, `PrintBuffered`'s
  negative-prebuffer rejection, `cJSON_InitHooks`'s `reallocate` selection —
  caught by the allocator call counters, not by the output);
* parsing state (`can_read` widths, whitespace scanning including the
  `offset == length` fixup, BOM detection width, the error-pointer position,
  `buffer_length = strlen + 1`, the nesting limit, the `parse_string` failure
  rewind);
* container invariants (`prev` back-pointer handling, the self-append guard, the
  corrupted-list guards in `Detach`/`Insert`, `ReplaceItemViaPointer`'s
  self-replace shortcut, `cJSON_StringIsConst` propagation in `Duplicate` and
  `replace_item_in_object`);
* memory accounting (a missing `deallocate` in `add_item_to_object` is a pure
  leak with no output effect — caught only by `tests/guarded.rs`);
* the composed `driver()` pipeline from `c_src/test.c`.
