# ERRORS.md — error-surface table for `c_src/`

Derived mechanically from the C sources: every `js_*error` / `js_throw` /
`jsY_error` / `jsP_error` / `jsC_error` / `die()` call site, every `assert()`,
every explicit range / bounds / limit check, every NULL check that changes
behaviour, and every `return -1` / `return NULL` / `return 0` failure signal.

**Total rows: 933**

## Phase C status

| | count |
|---|---|
| rows with a PASSING differential test (`[x]`) | **891** |
| rows proven UNREACHABLE or C undefined behaviour (`[-]`) | **42** |
| rows with no test (`[ ]`) | **0** |

The `cov` column names the Phase-C differential test that constructs the exact
invalid input, calls BOTH the C `.so` and the Rust `.so`, and asserts they
return the SAME error / sentinel (same message text and error constructor, not
merely "both failed"). `[-]` rows are each annotated in the owning test file
with a `///` comment naming the C `file:line` and the reason: either the site is
unreachable through any input (and a *pinning* test asserts that no input
reaches it), or the C's behaviour there is undefined (out-of-bounds write,
signed overflow, `assert()`/`abort()`), which cannot be compared.

## Error-surface constants (rejection thresholds)

| constant | value | used as |
|---|---|---|
| `JS_STACKSIZE` | 4096 | value-stack overflow -> bare string `"stack overflow"` |
| `JS_ENVLIMIT` | 1024 | env-stack + trace-stack overflow -> `"stack overflow"` |
| `JS_TRYLIMIT` | 64 | try-stack overflow -> bare string `"exception stack overflow"` |
| `JS_ARRAYLIMIT` | 1<<26 | `RangeError "array too large"` |
| `JS_STRLIMIT` | 1<<28 | max string length |
| `JS_ASTLIMIT` | 400 | `SyntaxError "too much recursion"` |
| `JS_GCFACTOR` | 5.0 | GC trigger threshold |
| `REG_MAXPROG` | 32768 | `regcomp` error `"program too large"` |
| `REG_MAXREC` | 4096 | `regexec` returns -1 -> `Error "regexec failed"` |
| `REG_MAXCLASS` | 128 | `"too many character classes"` |
| `REG_MAXSPAN` | 64 | `"too many character class ranges"` |
| `REG_MAXSUB` | 16 | `"too many captures"` |
| `REPINF` | 255 | `"numeric overflow"` in quantifiers |
| `maxExponent` (jsdtoa) | 511 | `js_strtod` ERANGE clamp -> inf / 0 |

## Rows

| # | file:line | function | trigger (the exact invalid input/condition) | expected C result | cov |
|---|-----------|----------|----------------------------------------------|-------------------|-----|
| 1 | jsrun.c:19 | js_trystackoverflow | try/exception stack full (reached from js_savetry, js_savetrypc, js_ptry when `J->trytop == JS_TRYLIMIT` = 64) | pushes JS_TLITSTR then `js_throw`; thrown value is the bare string `"exception stack overflow"` (not an Error object) | [x] `t_bare_string_throw_shapes` |
| 2 | jsrun.c:27 | js_stackoverflow | value stack or environment stack exhausted (reached from CHECKSTACK and jsR_savescope) | pushes JS_TLITSTR then `js_throw`; thrown value is the bare string `"stack overflow"` | [x] `t_bare_string_throw_shapes` |
| 3 | jsrun.c:35 | js_outofmemory | allocation failure or memlimit exceeded (reached from js_malloc / js_realloc) | pushes JS_TLITSTR then `js_throw`; thrown value is the bare string `"out of memory"` | [x] `t_bare_string_throw_shapes` |
| 4 | jsrun.c:43 | js_runlimit | instruction budget exhausted (reached from jsR_run) | pushes JS_TLITSTR then `js_throw`; thrown value is the bare string `"script ran too long"` | [x] `t_bare_string_throw_shapes` |
| 5 | jsrun.c:56 | js_malloc | `J->memlimit > 0 && size >= J->memlimit` | js_outofmemory → throws `"out of memory"` | [x] `t_malloc_realloc_limits` |
| 6 | jsrun.c:61 | js_malloc | host allocator `J->alloc(J->actx, NULL, size)` returns NULL | js_outofmemory → throws `"out of memory"`; would `return NULL` if throw ever returned | [x] `t_malloc_realloc_limits` |
| 7 | jsrun.c:70 | js_realloc | `J->memlimit > 0 && size >= J->memlimit` | js_outofmemory → throws `"out of memory"` | [x] `t_malloc_realloc_limits` |
| 8 | jsrun.c:75 | js_realloc | host allocator returns NULL on resize | js_outofmemory → throws `"out of memory"` | [x] `t_malloc_realloc_limits` |
| 9 | jsrun.c:106 | CHECKSTACK (macro) | `TOP + n >= JS_STACKSIZE` (JS_STACKSIZE == 4096) | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 10 | jsrun.c:110 | js_pushvalue | js_pushvalue when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 11 | jsrun.c:117 | js_pushundefined | js_pushundefined when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 12 | jsrun.c:124 | js_pushnull | js_pushnull when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 13 | jsrun.c:131 | js_pushboolean | js_pushboolean when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 14 | jsrun.c:139 | js_pushnumber | js_pushnumber when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 15 | jsrun.c:148 | js_pushstring | `strlen(v) > JS_STRLIMIT` (JS_STRLIMIT == 1<<28) | `js_rangeerror(J, "invalid string length")` → RangeError "invalid string length" | [x] `t_string_length_limit` |
| 16 | jsrun.c:150 | js_pushstring | js_pushstring when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 17 | jsrun.c:165 | js_pushlstring | explicit length `n > JS_STRLIMIT` | `js_rangeerror(J, "invalid string length")` → RangeError "invalid string length" | [x] `t_string_length_limit` |
| 18 | jsrun.c:167 | js_pushlstring | js_pushlstring when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 19 | jsrun.c:182 | js_pushliteral | js_pushliteral when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 20 | jsrun.c:190 | js_pushobject | js_pushobject when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 21 | jsrun.c:203 | js_currentfunction | js_currentfunction when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 22 | jsrun.c:204 | js_currentfunction | `BOT == 0` — no active call frame (called at top level) | pushes JS_TUNDEFINED instead of the current function; no error | [x] `t_currentfunction_no_frame` |
| 23 | jsrun.c:215 | js_currentfunctiondata | `BOT == 0` — no active call frame | `returns NULL` | [x] `t_currentfunction_no_frame` |
| 24 | jsrun.c:224 | stackidx | normalised `idx < 0 || idx >= TOP` (out-of-range stack index) | returns pointer to a static JS_TUNDEFINED value; silently reads as `undefined`, no error | [x] `t_stackidx_out_of_range` |
| 25 | jsrun.c:373 | js_toregexp | value at idx is not a JS_TOBJECT of class JS_CREGEXP | `js_typeerror(J, "not a regexp")` → TypeError "not a regexp" | [x] `t_toregexp_touserdata_tofunction` |
| 26 | jsrun.c:382 | js_touserdata | value is not JS_CUSERDATA, or `strcmp(tag, obj->u.user.tag) != 0` | `js_typeerror(J, "not a %s", tag)` → TypeError "not a \<tag\>" | [x] `t_toregexp_touserdata_tofunction` |
| 27 | jsrun.c:389 | jsR_tofunction | argument is JS_TUNDEFINED or JS_TNULL | `return NULL` (getter/setter slot left unset by caller) | [x] `t_toregexp_touserdata_tofunction` |
| 28 | jsrun.c:393 | jsR_tofunction | argument is neither undefined/null nor a JS_CFUNCTION/JS_CCFUNCTION object | `js_typeerror(J, "not a function")` → TypeError "not a function" | [x] `t_toregexp_touserdata_tofunction` |
| 29 | jsrun.c:406 | js_pop | `TOP - n < BOT` (popping more than the current frame holds) | clamps `TOP = BOT` then `js_error(J, "stack underflow!")` → Error "stack underflow!" | [x] `t_stack_manip_errors` |
| 30 | jsrun.c:415 | js_remove | normalised `idx < BOT || idx >= TOP` | `js_error(J, "stack error!")` → Error "stack error!" | [x] `t_stack_manip_errors` |
| 31 | jsrun.c:424 | js_insert | any call at all — function is unimplemented | `js_error(J, "not implemented yet")` → Error "not implemented yet" | [x] `t_stack_manip_errors` |
| 32 | jsrun.c:430 | js_replace | normalised `idx < BOT || idx >= TOP` | `js_error(J, "stack error!")` → Error "stack error!" | [x] `t_stack_manip_errors` |
| 33 | jsrun.c:437 | js_copy | js_copy when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 34 | jsrun.c:443 | js_dup | js_dup when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 35 | jsrun.c:451 | js_dup2 | js_dup2 when `TOP + 2 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 36 | jsrun.c:514 | js_isarrayindex | property name is the empty string `""` | `return 0` — not an array index, handled as a plain string key | [x] `t_isarrayindex` |
| 37 | jsrun.c:518 | js_isarrayindex | name starts with `'0'` and has more characters (e.g. `"01"`, `"0x1"`, `"00"`) | `return 0` — leading-zero forms rejected as indices (only `"0"` accepted) | [x] `t_isarrayindex` |
| 38 | jsrun.c:524 | js_isarrayindex | accumulated `n >= INT_MAX / 10` before consuming another digit (index ≥ ~10 digits) | `return 0` — overflowing index rejected, treated as a string key | [x] `t_isarrayindex` |
| 39 | jsrun.c:528 | js_isarrayindex | any character outside `'0'..'9'` | `return 0` — not an array index | [x] `t_isarrayindex` |
| 40 | jsrun.c:541 | js_pushrune | `rune < 0` (invalid/negative rune from js_runeat) | pushes JS_TUNDEFINED instead of a character string | [-] jsrun.c:541 js_pushrune(rune < 0) is unreachable. Its only caller (jsrun.c:596) is guarded by k >= 0 && k < obj->u.s.length, and u.s.length is js_utflen() (jsstring.c:49) which uses EXACTLY the same rune accounting as js_runeat (jsstring.c:20) -- both count a rune >= 0x10000 as two positions and both stop at the NUL -- so js_runeat never returns EOF for an index in [0, u.s.length). The whole in-range/out-of-range boundary (incl. astral runes) is still driven differentially by t_string_wrapper_indices. |
| 41 | jsrun.c:550 | jsR_unflattenarray | an exception is raised while migrating a flat array into the property tree (e.g. OOM in jsV_setproperty) | `obj->properties = NULL` then `js_throw(J)` — rethrows and leaves the object with a NULL property tree | [-] jsrun.c:550-552, the js_try handler in jsR_unflattenarray, sets obj->properties = NULL and rethrows. A NULL property tree is then dereferenced unconditionally by jsproperty.c:48 (lookup), jsgc.c:101 (jsG_scanobject) and jsgc.c:35 (jsG_freeobject), so reaching it makes every later property access / js_gc / js_freestate a NULL dereference. Undefined behaviour, not observable in-process. |
| 42 | jsrun.c:584 | jsR_hasproperty | simple (flat) JS_CARRAY, array index `k` outside `[0, flat_length)` | `return 0` — caller pushes undefined; prototype chain is NOT consulted | [x] `t_property_miss` |
| 43 | jsrun.c:594 | jsR_hasproperty | JS_CSTRING wrapper, array index `k` outside `[0, u.s.length)` | falls through to prototype lookup; `return 0` if nothing found | [x] `t_property_miss` |
| 44 | jsrun.c:630 | jsR_hasproperty | `jsV_getproperty` returns NULL — name absent from object and its whole prototype chain | skips getter/value push, falls to `return 0` | [x] `t_property_miss` |
| 45 | jsrun.c:642 | jsR_hasproperty | no matching special case and no property found | `return 0` — property does not exist | [x] `t_property_miss` |
| 46 | jsrun.c:648 | jsR_getproperty | `jsR_hasproperty` returned 0 | pushes JS_TUNDEFINED (reads of missing properties yield `undefined`, not an error) | [x] `t_property_miss` |
| 47 | jsrun.c:659 | jsR_hasindex | simple JS_CARRAY, `k` outside `[0, flat_length)` | `return 0` | [x] `t_property_miss` |
| 48 | jsrun.c:667 | jsR_getindex | `jsR_hasindex` returned 0 | pushes JS_TUNDEFINED | [x] `t_property_miss` |
| 49 | jsrun.c:673 | jsR_setarrayindex | called with `obj->u.a.simple == 0` (non-flat array) | `abort()` via `assert(obj->u.a.simple)` | [-] jsrun.c:673 assert(obj->u.a.simple) in the static jsR_setarrayindex. Both call sites (jsrun.c:722, jsrun.c:806) first test u.a.simple && k >= 0 && k <= flat_length, so no exported entry point can violate it; jsR_setarrayindex itself is not exported. |
| 50 | jsrun.c:674 | jsR_setarrayindex | called with negative index `k < 0` | `abort()` via `assert(k >= 0)` | [-] jsrun.c:674 assert(k >= 0) -- same guards as row 49. |
| 51 | jsrun.c:675 | jsR_setarrayindex | `k + 1 > JS_ARRAYLIMIT` (JS_ARRAYLIMIT == 1<<26, 64M entries) | `js_rangeerror(J, "array too large")` → RangeError "array too large" | [-] jsrun.c:675 needs flat_length >= 1<<26, i.e. 64M live js_Values = 1 GiB of flat array data, because jsR_setarrayindex is only called with k <= flat_length and flat_length grows one element at a time (asserted at jsrun.c:678). Not bounded work. The reachable sibling 'array too large' site (jsrun.c:708) is covered by t_array_length_errors. |
| 52 | jsrun.c:678 | jsR_setarrayindex | growing write where `newlen > flat_length` but `newlen != flat_length + 1` (non-append hole) | `abort()` via `assert(newlen == obj->u.a.flat_length + 1)` | [-] jsrun.c:678 assert(newlen == flat_length + 1) -- same guards as row 49. |
| 53 | jsrun.c:706 | jsR_setproperty | array `length` assigned a value whose integer form differs from the raw number or is negative (`a.length = 1.5`, `-1`, `NaN`, `Infinity`, `"foo"`) | `js_rangeerror(J, "invalid array length")` → RangeError "invalid array length" | [x] `t_array_length_errors` |
| 54 | jsrun.c:708 | jsR_setproperty | array `length` assigned a value `> JS_ARRAYLIMIT` | `js_rangeerror(J, "array too large")` → RangeError "array too large" | [x] `t_array_length_errors` |
| 55 | jsrun.c:737 | jsR_setproperty | assignment to `length` of a JS_CSTRING wrapper | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 56 | jsrun.c:739 | jsR_setproperty | assignment to an in-range character index `0 <= k < u.s.length` of a JS_CSTRING wrapper | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 57 | jsrun.c:745 | jsR_setproperty | assignment to regexp `source` | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 58 | jsrun.c:746 | jsR_setproperty | assignment to regexp `global` | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 59 | jsrun.c:747 | jsR_setproperty | assignment to regexp `ignoreCase` | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 60 | jsrun.c:748 | jsR_setproperty | assignment to regexp `multiline` | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 61 | jsrun.c:756 | jsR_setproperty | JS_CUSERDATA whose `put` callback returns non-zero | `return` — assignment consumed by host callback, no further property created | [x] `t_userdata_hooks_shortcircuit` |
| 62 | jsrun.c:773 | jsR_setproperty | `J->strict` and the found property has a getter but no setter | `js_typeerror(J, "setting property '%s' that only has a getter", name)` → TypeError "setting property '\<name\>' that only has a getter" | [x] `t_setproperty_readonly_forks` |
| 63 | jsrun.c:775 | jsR_setproperty | found property has `JS_READONLY` attribute | `goto readonly` (row 67) | [x] `t_setproperty_readonly_forks` |
| 64 | jsrun.c:783 | jsR_setproperty | `transient` receiver (assigning a property on a primitive) and `J->strict` | `js_typeerror(J, "cannot create property '%s' on transient object", name)` → TypeError "cannot create property '\<name\>' on transient object" | [x] `t_setproperty_readonly_forks` |
| 65 | jsrun.c:785 | jsR_setproperty | `transient` receiver and not strict | `return` — assignment silently discarded | [x] `t_setproperty_readonly_forks` |
| 66 | jsrun.c:792 | jsR_setproperty | `jsV_setproperty` returned a ref carrying `JS_READONLY` | `goto readonly` (row 67) | [-] jsrun.c:792 is unreachable. It is only reached when !ref || !own held at jsrun.c:780, i.e. `name` is NOT an own property of obj. jsV_setproperty (jsproperty.c:221) then either inserts a fresh newproperty (jsproperty.c:35 sets atts = 0) or, for a non-extensible object, returns lookup(obj->properties, name) which is NULL for exactly the reason own was 0 (jsV_getpropertyx uses the same lookup). So the ref reaching jsrun.c:790 can never carry JS_READONLY. The reachable READONLY forks (jsrun.c:775 row 63, jsrun.c:800 row 67) are covered by t_setproperty_readonly_forks. |
| 67 | jsrun.c:800 | jsR_setproperty | `readonly:` label reached with `J->strict` set | `js_typeerror(J, "'%s' is read-only", name)` → TypeError "'\<name\>' is read-only"; in non-strict mode the write is silently dropped | [x] `t_setproperty_readonly_forks` |
| 68 | jsrun.c:821 | jsR_defproperty | defining/redefining `length` on a JS_CARRAY | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 69 | jsrun.c:828 | jsR_defproperty | defining `length` on a JS_CSTRING wrapper | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 70 | jsrun.c:830 | jsR_defproperty | defining an in-range character index `0 <= k < u.s.length` on a JS_CSTRING wrapper | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 71 | jsrun.c:836 | jsR_defproperty | defining regexp `source` | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 72 | jsrun.c:837 | jsR_defproperty | defining regexp `global` | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 73 | jsrun.c:838 | jsR_defproperty | defining regexp `ignoreCase` | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 74 | jsrun.c:839 | jsR_defproperty | defining regexp `multiline` | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 75 | jsrun.c:840 | jsR_defproperty | defining regexp `lastIndex` (unlike jsR_setproperty, defineProperty on lastIndex is refused) | `goto readonly` (row 82) | [x] `t_defproperty_forks` |
| 76 | jsrun.c:844 | jsR_defproperty | JS_CUSERDATA whose `put` callback returns non-zero | `return` — definition consumed by host callback | [x] `t_userdata_hooks_shortcircuit` |
| 77 | jsrun.c:849 | jsR_defproperty | `jsV_setproperty` returned NULL (non-extensible object, non-strict) | whole `if (ref)` block skipped — definition silently dropped | [x] `t_defproperty_forks` |
| 78 | jsrun.c:854 | jsR_defproperty | value supplied for an existing `JS_READONLY` property while `J->strict` | `js_typeerror(J, "'%s' is read-only", name)` → TypeError "'\<name\>' is read-only" | [x] `t_defproperty_forks` |
| 79 | jsrun.c:860 | jsR_defproperty | getter supplied for a property with `JS_DONTCONF` while `J->strict` | `js_typeerror(J, "'%s' is non-configurable", name)` → TypeError "'\<name\>' is non-configurable" | [x] `t_defproperty_forks` |
| 80 | jsrun.c:866 | jsR_defproperty | setter supplied for a property with `JS_DONTCONF` while `J->strict` | `js_typeerror(J, "'%s' is non-configurable", name)` → TypeError "'\<name\>' is non-configurable" | [x] `t_defproperty_forks` |
| 81 | jsrun.c:874 | jsR_defproperty | `readonly:` label reached with neither `J->strict` nor `throw` set | falls off the end — definition silently ignored | [x] `t_defproperty_forks` |
| 82 | jsrun.c:875 | jsR_defproperty | `readonly:` label reached with `J->strict || throw` | `js_typeerror(J, "'%s' is read-only or non-configurable", name)` → TypeError "'\<name\>' is read-only or non-configurable" | [x] `t_defproperty_forks` |
| 83 | jsrun.c:884 | jsR_delproperty | `delete` of `length` on a JS_CARRAY | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 84 | jsrun.c:891 | jsR_delproperty | `delete` of `length` on a JS_CSTRING wrapper | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 85 | jsrun.c:893 | jsR_delproperty | `delete` of in-range character index `0 <= k < u.s.length` on a JS_CSTRING wrapper | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 86 | jsrun.c:899 | jsR_delproperty | `delete` of regexp `source` | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 87 | jsrun.c:900 | jsR_delproperty | `delete` of regexp `global` | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 88 | jsrun.c:901 | jsR_delproperty | `delete` of regexp `ignoreCase` | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 89 | jsrun.c:902 | jsR_delproperty | `delete` of regexp `multiline` | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 90 | jsrun.c:903 | jsR_delproperty | `delete` of regexp `lastIndex` | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 91 | jsrun.c:907 | jsR_delproperty | JS_CUSERDATA whose `delete` callback returns non-zero | `return 1` — delete consumed by host callback | [x] `t_userdata_hooks_shortcircuit` |
| 92 | jsrun.c:911 | jsR_delproperty | `jsV_getownproperty` returns NULL (property not an own property) | skips deletion, still `return 1` (delete of a missing property "succeeds") | [x] `t_delproperty_forks` |
| 93 | jsrun.c:913 | jsR_delproperty | own property has `JS_DONTCONF` attribute | `goto dontconf` (row 96) | [x] `t_delproperty_forks` |
| 94 | jsrun.c:920 | jsR_delproperty | `dontconf:` label reached in non-strict mode | `return 0` — delete reports failure without throwing | [x] `t_delproperty_forks` |
| 95 | jsrun.c:921 | jsR_delproperty | `dontconf:` label reached with `J->strict` | `js_typeerror(J, "'%s' is non-configurable", name)` → TypeError "'\<name\>' is non-configurable" | [x] `t_delproperty_forks` |
| 96 | jsrun.c:1094 | js_hasvar | `jsV_getproperty` returns NULL for the variable in this environment record | continues to `E->outer`; nothing pushed for this level | [x] `t_var_ops` |
| 97 | jsrun.c:1107 | js_hasvar | variable name not found in any environment record in the scope chain | `return 0` — nothing pushed; callers turn this into a ReferenceError | [x] `t_var_ops` |
| 98 | jsrun.c:1127 | js_setvar | variable found with `JS_READONLY` att (no setter) while `J->strict` | `js_typeerror(J, "'%s' is read-only", name)` → TypeError "'\<name\>' is read-only" | [x] `t_var_ops` |
| 99 | jsrun.c:1133 | js_setvar | assignment to a name not present in any scope while `J->strict` | `js_referenceerror(J, "assignment to undeclared variable '%s'", name)` → ReferenceError "assignment to undeclared variable '\<name\>'" | [x] `t_var_ops` |
| 100 | jsrun.c:1145 | js_delvar | `delete` of a variable whose own property has `JS_DONTCONF` while `J->strict` | `js_typeerror(J, "'%s' is non-configurable", name)` → TypeError "'\<name\>' is non-configurable"; non-strict path `return 0` | [x] `t_var_ops` |
| 101 | jsrun.c:1160 | jsR_savescope | `J->envtop + 1 >= JS_ENVLIMIT` (JS_ENVLIMIT == 1024) — deep recursion | js_stackoverflow → throws `"stack overflow"` | [-] jsrun.c:1160 jsR_savescope's env-stack overflow is unreachable because envtop <= tracetop is invariant: every jsR_savescope call site (jsrun.c:1176/1201/1243) sits inside a js_call branch that ran jsR_pushtrace first (jsrun.c:1315/1322/1326), and js_throw restores both counters together (jsrun.c:1471-1472). jsR_pushtrace trips at tracetop == 1023 while jsR_savescope needs envtop == 1023, so row 105's js_error("call stack overflow") always fires first (measured: the limit is hit at depth 1021, see t_call_stack_overflow). The other js_stackoverflow caller, CHECKSTACK, is covered exhaustively by t_value_stack_overflow_matrix. |
| 102 | jsrun.c:1178 | jsR_calllwfunction | lightweight call with `n > F->numparams` (more arguments than declared params) | excess arguments silently popped via `js_pop(J, n - F->numparams)`; no error | [x] `t_call_paths` |
| 103 | jsrun.c:1272 | jsR_callcfunction | native call with `n < min` (fewer arguments than the C function's declared length) | pads the stack with `js_pushundefined` for every missing argument | [x] `t_call_paths` |
| 104 | jsrun.c:1277 | jsR_callcfunction | native function returns without pushing a value (`TOP <= save_top`) | frame cleared and `js_pushundefined` — call result is `undefined` | [x] `t_call_paths` |
| 105 | jsrun.c:1289 | jsR_pushtrace | `J->tracetop + 1 == JS_ENVLIMIT` (1024 nested calls) | `js_error(J, "call stack overflow")` → Error "call stack overflow" | [x] `t_call_stack_overflow` |
| 106 | jsrun.c:1303 | js_call | `n < 0` (negative argument count from the C API) | `js_rangeerror(J, "number of arguments cannot be negative")` → RangeError "number of arguments cannot be negative" | [x] `t_call_paths` |
| 107 | jsrun.c:1306 | js_call | callee at `-n-2` is not JS_CFUNCTION/JS_CSCRIPT/JS_CCFUNCTION | `js_typeerror(J, "%s is not callable", js_typeof(J, -n-2))` → TypeError "\<typeof\> is not callable" | [x] `t_call_paths` |
| 108 | jsrun.c:1314 | js_call | callee object class is none of JS_CFUNCTION/JS_CSCRIPT/JS_CCFUNCTION (unreachable given row 107) | falls through all branches, restores BOT and returns leaving the stack unmodified | [-] jsrun.c:1314 is unreachable given row 107, as ERRORS.md itself notes: js_iscallable (jsrun.c:244) accepts exactly the three classes the if/else-if chain dispatches on. |
| 109 | jsrun.c:1340 | js_construct | `new X` where the value at `-n-1` is not callable | `js_typeerror(J, "%s is not callable", js_typeof(J, -n-1))` → TypeError "\<typeof\> is not callable" | [x] `t_call_paths` |
| 110 | jsrun.c:1363 | js_construct | constructor's `prototype` property is not an object | silently falls back to `J->Object_prototype` | [x] `t_call_paths` |
| 111 | jsrun.c:1383 | js_construct | constructor body returns a non-object | discards the return value and yields the freshly created object instead | [x] `t_call_paths` |
| 112 | jsrun.c:1392 | js_eval | argument at `-1` is not a string (`eval(42)`, `eval({})`) | `return` immediately — argument is left on the stack unevaluated, no error | [x] `t_call_paths` |
| 113 | jsrun.c:1403 | js_pconstruct | any exception thrown during `js_construct` (setjmp path taken) | stack trimmed to `STACK[savetop] = error`, `TOP = savetop + 1`, `return 1` | [x] `t_call_paths` |
| 114 | jsrun.c:1417 | js_pcall | any exception thrown during `js_call` (setjmp path taken) | stack trimmed to `STACK[savetop] = error`, `TOP = savetop + 1`, `return 1` | [x] `t_call_paths` |
| 115 | jsrun.c:1432 | js_savetrypc | `J->trytop == JS_TRYLIMIT` (64 nested try blocks) at bytecode `try` | js_trystackoverflow → throws `"exception stack overflow"` | [x] `t_try_limits` |
| 116 | jsrun.c:1446 | js_savetry | `J->trytop == JS_TRYLIMIT` (64) at a C-level `js_try` | js_trystackoverflow → throws `"exception stack overflow"` | [x] `t_try_limits` |
| 117 | jsrun.c:1460 | js_endtry | `J->trytop == 0` — endtry with no matching try | `js_error(J, "endtry: exception stack underflow")` → Error "endtry: exception stack underflow" | [x] `t_endtry_and_unhandled_throw` |
| 118 | jsrun.c:1479 | js_throw | throw with `J->trytop == 0` (no handler) and `J->panic` set | calls `J->panic(J)` (default reports "uncaught exception") | [x] `t_endtry_and_unhandled_throw` |
| 119 | jsrun.c:1481 | js_throw | throw with no handler and panic absent or returning | `abort()` — process terminates | [x] `t_endtry_and_unhandled_throw` |
| 120 | jsrun.c:1570 | jsR_isindex | JS_TNUMBER key that is not an exact non-negative integer (`a[1.5]`, `a[-1]`, `a[NaN]`) | `return 0` — falls back to string-keyed property access | [x] `t_run_loop_paths` |
| 121 | jsrun.c:1573 | jsR_isindex | key is not JS_TNUMBER at all | `return 0` — string-keyed property access | [x] `t_run_loop_paths` |
| 122 | jsrun.c:1602 | jsR_run | `J->runlimit > 0 && J->runlimit == 1` — instruction budget hit | js_runlimit → throws `"script ran too long"` | [x] `t_run_loop_paths` |
| 123 | jsrun.c:1608 | jsR_run | `J->gccounter > J->gcthresh` — allocation count exceeds GC threshold | `js_gc(J, 0)` forced collection mid-loop | [x] `t_forced_gc_in_run_loop` |
| 124 | jsrun.c:1668 | jsR_run (OP_GETLOCAL) | lightweight local read when `TOP + 1 >= JS_STACKSIZE` | js_stackoverflow → throws `"stack overflow"` | [x] `t_value_stack_overflow_matrix` |
| 125 | jsrun.c:1672 | jsR_run (OP_GETLOCAL) | non-lightweight local not present in the scope chain (`!js_hasvar`) | `js_referenceerror(J, "'%s' is not defined", str)` → ReferenceError "'\<name\>' is not defined" | [x] `t_var_ops` |
| 126 | jsrun.c:1697 | jsR_run (OP_GETVAR) | global/free variable read where `!js_hasvar(J, str)` | `js_referenceerror(J, "'%s' is not defined", str)` → ReferenceError "'\<name\>' is not defined" | [x] `t_var_ops` |
| 127 | jsrun.c:1703 | jsR_run (OP_HASVAR) | `typeof`-style read where `!js_hasvar(J, str)` | pushes JS_TUNDEFINED instead of raising ReferenceError | [x] `t_run_loop_paths` |
| 128 | jsrun.c:1720 | jsR_run (OP_IN) | right operand of `in` is not an object (`"x" in 1`, `"x" in null`) | `js_typeerror(J, "operand to 'in' is not an object")` → TypeError "operand to 'in' is not an object" | [x] `t_run_loop_paths` |
| 129 | jsrun.c:1813 | jsR_run (OP_ITERATOR) | `for..in` over a non-coercible value (undefined/null) | no iterator created; value left as-is so the following OP_NEXTITER takes the non-object path | [x] `t_run_loop_paths` |
| 130 | jsrun.c:1821 | jsR_run (OP_NEXTITER) | iterator slot is not an object | pops it and pushes `false` (loop terminates) | [x] `t_run_loop_paths` |
| 131 | jsrun.c:1824 | jsR_run (OP_NEXTITER) | `jsV_nextiterator` returns NULL (iteration exhausted) | pops iterator and pushes `false` | [x] `t_run_loop_paths` |
| 132 | jsrun.c:2025 | jsR_run (OP_THROW) | JavaScript `throw` statement | `js_throw(J)` — unwinds to nearest try, or panic/abort if none (rows 118-119) | [x] `t_run_loop_paths` |
| 133 | jsstate.c:6 | js_ptry | `J->trytop == JS_TRYLIMIT` (64) before installing a protected frame | pushes JS_TLITSTR `"exception stack overflow"` on the stack and `return 1` (no longjmp) | [x] `t_try_limits` |
| 134 | jsstate.c:38 | js_ploadstring | try stack already full (js_ptry returns 1) | `return 1` with the string `"exception stack overflow"` left on the stack | [x] `t_try_limits` |
| 135 | jsstate.c:40 | js_ploadstring | parse or compile error thrown by `js_loadstring` (setjmp path) | `return 1` with the error object on the stack | [x] `t_try_limits` |
| 136 | jsstate.c:50 | js_trystring | try stack full | `js_pop(J, 1)` then `return error` (the caller-supplied default string) | [x] `t_try_limits` |
| 137 | jsstate.c:54 | js_trystring | exception thrown while running `toString` conversion | `js_pop(J, 1)` then `return error` (caller default) | [x] `t_ffi_try_defaults` |
| 138 | jsstate.c:66 | js_trynumber | try stack full | `js_pop(J, 1)` then `return error` (caller-supplied default double) | [x] `t_try_limits` |
| 139 | jsstate.c:70 | js_trynumber | exception thrown while running `valueOf`/ToNumber conversion | `js_pop(J, 1)` then `return error` | [x] `t_ffi_try_defaults` |
| 140 | jsstate.c:82 | js_tryinteger | try stack full | `js_pop(J, 1)` then `return error` (caller default int) | [x] `t_try_limits` |
| 141 | jsstate.c:86 | js_tryinteger | exception thrown during ToInteger conversion | `js_pop(J, 1)` then `return error` | [x] `t_ffi_try_defaults` |
| 142 | jsstate.c:98 | js_tryboolean | try stack full | `js_pop(J, 1)` then `return error` (caller default int) | [x] `t_try_limits` |
| 143 | jsstate.c:102 | js_tryboolean | exception thrown during ToBoolean conversion | `js_pop(J, 1)` then `return error` | [-] jsstate.c:102 js_tryboolean's setjmp path cannot be taken: js_toboolean (jsrun.c:318) is jsV_toboolean (jsvalue.c:152), a total switch over v->t.type that only reads shrstr[0] / litstr[0] / memstr->p[0] / u.boolean / u.number -- it allocates nothing and calls nothing, so ToBoolean can never throw. The js_ptry half of js_tryboolean (row 142, try stack full) IS covered, by t_try_limits, and t_ffi_try_defaults shows js_tryboolean returning the real value for receivers whose toString/valueOf throw. |
| 144 | jsstate.c:116 | js_loadstringx | SyntaxError from `jsP_parse` or error from `jsC_compilescript` on malformed source | `jsP_freeparse(J)` to release the AST then `js_throw(J)` — rethrows the original SyntaxError | [x] `t_loadstring_paths` |
| 145 | jsstate.c:141 | js_dostring | try stack full before running the script | `js_report(J, "exception stack overflow")`, `js_pop(J, 1)`, `return 1` | [x] `t_try_limits` |
| 146 | jsstate.c:146 | js_dostring | any uncaught exception from parsing/compiling/running the source | `js_report(J, js_trystring(J, -1, "Error"))` — reports the error message or the literal `"Error"` if stringification itself fails — `js_pop(J, 1)`, `return 1` | [x] `t_loadstring_paths` |
| 147 | jsstate.c:191 | js_newstate | build where `sizeof(js_Value) != 16` | `abort()` via `assert(sizeof(js_Value) == 16)` | [-] jsstate.c:191 assert(sizeof(js_Value) == 16) is a property of the BUILD, not of any input; it holds in this build (both libraries agree on every js_pushstring / js_pushlstring short-string boundary, see t_shrstr_boundary). No input can make it fail. |
| 148 | jsstate.c:192 | js_newstate | build where the type tag is not at byte offset 15 of js_Value | `abort()` via `assert(soffsetof(js_Value, t.type) == 15)` | [-] jsstate.c:192 assert(soffsetof(js_Value, t.type) == 15) -- same reason as row 147; t_shrstr_boundary shows the tag really does sit at byte 15 in both libraries (15-byte strings are JS_TSHRSTR, 16-byte strings are JS_TMEMSTR). |
| 149 | jsstate.c:198 | js_newstate | allocator fails to allocate the js_State struct | `return NULL` (no state created, nothing leaked) | [x] `t_newstate_alloc_and_report` |
| 150 | jsstate.c:215 | js_newstate | allocator fails to allocate the `JS_STACKSIZE * sizeof(js_Value)` value stack | frees the partially built state via `alloc(actx, J, 0)` then `return NULL` | [x] `t_newstate_alloc_and_report` |
| 151 | jsstate.c:224 | js_newstate | exception thrown while creating the registry/global object/environment or during `jsB_init` (e.g. OOM) | `js_freestate(J)` then `return NULL` | [x] `t_newstate_alloc_and_report` |
| 152 | jsstate.c:168 | js_report | `J->report == NULL` (reporter cleared via js_setreport) | message silently dropped, no output | [x] `t_newstate_alloc_and_report` |
| 153 | jserror.c:10 | jsB_stacktrace | `J->tracetop - skip <= 0` — no frames left after skipping | `return 0` — no `stackTrace`/`stack` property is attached to the error | [x] `t_newerror_family` |
| 154 | jserror.c:18 | jsB_stacktrace | a single trace frame formats to more than 255 bytes | `snprintf(buf, sizeof buf, ...)` with `buf[256]` silently truncates that trace line | [x] `t_stacktrace_truncation` |
| 155 | jserror.c:35 | Ep_toString | `Error.prototype.toString` called with a non-object `this` | `js_typeerror(J, "not an object")` → TypeError "not an object" | [x] `t_error_prototype_tostring` |
| 156 | jserror.c:38 | Ep_toString | error object has no `name` property | keeps the default `name = "Error"` | [x] `t_error_prototype_tostring` |
| 157 | jserror.c:40 | Ep_toString | error object has no `message` property | keeps the default `message = ""`; with empty name too the result is the empty string | [x] `t_error_prototype_tostring` |
| 158 | jserror.c:66 | jsB_ErrorX | `new Error()` with argument 1 undefined (`js_isdefined(J, 1)` false) | no `message` property defined on the new error object | [x] `t_error_prototype_tostring` |
| 159 | jserror.c:95 | js_error / js_evalerror / js_rangeerror / js_referenceerror / js_syntaxerror / js_typeerror / js_urierror (DERROR macro) | formatted error message longer than 255 bytes | `vsnprintf(buf, sizeof buf, fmt, ap)` with `char buf[256]` silently truncates the message | [x] `t_error_varargs_ffi` |
| 160 | jserror.c:101 | js_error (DERROR(error, Error)) | any internal error condition (`js_error(J, fmt, ...)`) | builds a JS_CERROR object with `J->Error_prototype`, sets `message`/`stackTrace`, then `js_throw` → Error "\<formatted message\>" | [x] `t_error_varargs_ffi` |
| 161 | jserror.c:102 | js_evalerror (DERROR(evalerror, EvalError)) | eval-related failure | throws an EvalError object with the formatted message | [x] `t_error_varargs_ffi` |
| 162 | jserror.c:103 | js_rangeerror (DERROR(rangeerror, RangeError)) | out-of-range value (array length, string length, negative arg count, etc.) | throws a RangeError object with the formatted message | [x] `t_error_varargs_ffi` |
| 163 | jserror.c:104 | js_referenceerror (DERROR(referenceerror, ReferenceError)) | undefined variable read or strict-mode undeclared assignment | throws a ReferenceError object with the formatted message | [x] `t_error_varargs_ffi` |
| 164 | jserror.c:105 | js_syntaxerror (DERROR(syntaxerror, SyntaxError)) | malformed source detected by lexer/parser/compiler | throws a SyntaxError object with the formatted message | [x] `t_error_varargs_ffi` |
| 165 | jserror.c:106 | js_typeerror (DERROR(typeerror, TypeError)) | wrong type / non-callable / read-only / non-configurable operations | throws a TypeError object with the formatted message | [x] `t_error_varargs_ffi` |
| 166 | jserror.c:107 | js_urierror (DERROR(urierror, URIError)) | malformed URI escape sequence in encodeURI/decodeURI | throws a URIError object with the formatted message | [x] `t_error_varargs_ffi` |
| 167 | jsvalue.c:34 | js_strtol | first character whose digit value `table[c] >= base` (invalid digit for the radix, including any non-alphanumeric byte, which maps to 80) | parsing stops there; returns the digits accumulated so far and sets `*p` to that character (no error) | [x] `t_strtol_invalid_digits` |
| 168 | jsvalue.c:43 | jsV_numbertointeger | `n == 0` (including `-0`) | `return 0` | [x] `t_numbertointeger_and_int32` |
| 169 | jsvalue.c:44 | jsV_numbertointeger | `isnan(n)` | `return 0` — NaN silently coerced to 0 | [x] `t_numbertointeger_and_int32` |
| 170 | jsvalue.c:46 | jsV_numbertointeger | `n < INT_MIN` (including `-Infinity`) | `return INT_MIN` — clamped, not an error | [x] `t_numbertointeger_and_int32` |
| 171 | jsvalue.c:47 | jsV_numbertointeger | `n > INT_MAX` (including `+Infinity`) | `return INT_MAX` — clamped, not an error | [x] `t_numbertointeger_and_int32` |
| 172 | jsvalue.c:56 | jsV_numbertoint32 | `!isfinite(n) || n == 0` (NaN, ±Infinity, ±0) | `return 0` | [x] `t_numbertointeger_and_int32` |
| 173 | jsvalue.c:61 | jsV_numbertoint32 | value modulo 2^32 lands at or above 2^31 | `return n - two32` — wraps to a negative int32 | [x] `t_numbertointeger_and_int32` |
| 174 | jsvalue.c:87 | jsV_toString | object's `toString` property is not callable | `js_pop(J, 2)`, `return 0` — caller falls back to `valueOf` | [x] `t_toprimitive` |
| 175 | jsvalue.c:90 | jsV_toString | `toString()` returned a non-primitive (an object) | `js_pop(J, 1)`, `return 0` | [x] `t_toprimitive` |
| 176 | jsvalue.c:104 | jsV_valueOf | object's `valueOf` property is not callable | `js_pop(J, 2)`, `return 0` — caller falls back to `toString` | [x] `t_toprimitive` |
| 177 | jsvalue.c:107 | jsV_valueOf | `valueOf()` returned a non-primitive (an object) | `js_pop(J, 1)`, `return 0` | [x] `t_toprimitive` |
| 178 | jsvalue.c:121 | jsV_toprimitive | value is already a primitive (`v->t.type != JS_TOBJECT`) | `return` unchanged — no conversion attempted | [x] `t_toprimitive` |
| 179 | jsvalue.c:143 | jsV_toprimitive | both `toString` and `valueOf` fail to yield a primitive while `J->strict` (e.g. `Object.create(null)`) | `js_typeerror(J, "cannot convert object to primitive")` → TypeError "cannot convert object to primitive" | [x] `t_toprimitive` |
| 180 | jsvalue.c:146 | jsV_toprimitive | both conversions fail in non-strict mode | value is overwritten with JS_TLITSTR `"[object]"` and returned | [x] `t_toprimitive` |
| 181 | jsvalue.c:217 | js_stringtofloat | `strtod`/`strtol` end pointer disagrees with the hand-scanned end `e` (no valid numeric prefix, e.g. `"."`, `"e5"`, `"+"`) | `*ep = (char*)s` (nothing consumed) and `return 0` | [x] `t_stringtofloat_and_stringtonumber` |
| 182 | jsvalue.c:231 | jsV_stringtonumber | `"0x"`/`"0X"` with no digits (`s[2] == 0`) | hex branch not taken; falls through to js_stringtofloat, which parses just the leading `0` | [x] `t_stringtofloat_and_stringtonumber` |
| 183 | jsvalue.c:242 | jsV_stringtonumber | any non-whitespace characters remain after the numeric prefix (`"12abc"`, `"1 2"`, `""` after junk) | `return NAN` — string rejected as a number | [x] `t_stringtofloat_and_stringtonumber` |
| 184 | jsvalue.c:275 | jsV_numbertostring | `f == 0` | returns the static literal `"0"` (also collapses `-0` to `"0"`) | [x] `t_numbertostring` |
| 185 | jsvalue.c:276 | jsV_numbertostring | `isnan(f)` | returns the static literal `"NaN"` | [x] `t_numbertostring` |
| 186 | jsvalue.c:277 | jsV_numbertostring | `isinf(f)` | returns the static literal `"-Infinity"` or `"Infinity"` | [x] `t_numbertostring` |
| 187 | jsvalue.c:401 | jsV_toobject | ToObject on JS_TUNDEFINED (`undefined.x`, `Object(undefined)` internals) | `js_typeerror(J, "cannot convert undefined to object")` → TypeError "cannot convert undefined to object" | [x] `t_toobject_errors` |
| 188 | jsvalue.c:402 | jsV_toobject | ToObject on JS_TNULL (`null.x`) | `js_typeerror(J, "cannot convert null to object")` → TypeError "cannot convert null to object" | [x] `t_toobject_errors` |
| 189 | jsvalue.c:418 | js_newobjectx | prototype argument on the stack is not an object | `prototype = NULL` — object created with no prototype instead of erroring | [x] `t_newobjectx_userdatax_proto` |
| 190 | jsvalue.c:486 | js_newcfunctionx | exception thrown while allocating the JS_CCFUNCTION object (OOM) | calls `finalize(J, data)` on the userdata if present, then `js_throw(J)` to rethrow | [x] `t_oom_finalize` |
| 191 | jsvalue.c:544 | js_newuserdatax | prototype argument on the stack is not an object | `prototype = NULL` | [x] `t_newobjectx_userdatax_proto` |
| 192 | jsvalue.c:548 | js_newuserdatax | exception thrown while allocating the JS_CUSERDATA object (OOM) | calls `finalize(J, data)` then `js_throw(J)` to rethrow | [x] `t_oom_finalize` |
| 193 | jsvalue.c:578 | js_instanceof | right operand of `instanceof` is not callable (`1 instanceof {}`) | `js_typeerror(J, "instanceof: invalid operand")` → TypeError "instanceof: invalid operand" | [x] `t_instanceof` |
| 194 | jsvalue.c:581 | js_instanceof | left operand is not an object (`1 instanceof Object`) | `return 0` — evaluates to `false`, no error | [x] `t_instanceof` |
| 195 | jsvalue.c:585 | js_instanceof | constructor's `prototype` property is not an object | `js_typeerror(J, "instanceof: 'prototype' property is not an object")` → TypeError "instanceof: 'prototype' property is not an object" | [x] `t_instanceof` |
| 196 | jsvalue.c:597 | js_instanceof | prototype chain of the left operand exhausted without matching | `return 0` | [x] `t_instanceof` |
| 197 | jsvalue.c:610 | js_concat | exception raised while building the concatenated string (OOM in js_malloc, or RangeError in js_pushstring) | `js_free(J, sab)` to release the temporary buffer, then `js_throw(J)` to rethrow | [x] `t_concat_limits` |
| 198 | jsvalue.c:614 | js_concat | `strlen(sa) + strlen(sb) + 1` exceeds the memlimit | js_malloc → js_outofmemory → throws `"out of memory"` | [x] `t_concat_limits` |
| 199 | jsvalue.c:618 | js_concat | concatenated result longer than JS_STRLIMIT (1<<28) | `js_pushstring` → `js_rangeerror(J, "invalid string length")` → RangeError "invalid string length" | [x] `t_concat_limits` |
| 200 | jsvalue.c:640 | js_compare | either operand is NaN after ToPrimitive/ToNumber (`NaN < 1`) | `*okay = 0` — caller (OP_LT/GT/LE/GE) forces the comparison result to `false` | [x] `t_compare_equal_strictequal` |
| 201 | jsvalue.c:660 | js_equal | operands share a type not covered by the checks above | `return 0` | [x] `t_compare_equal_strictequal` |
| 202 | jsvalue.c:690 | js_equal | no loose-equality coercion rule matches (e.g. `null == 0`, `undefined == 0`) | `return 0` | [x] `t_compare_equal_strictequal` |
| 203 | jsvalue.c:701 | js_strictequal | `x->t.type != y->t.type` and not both strings | `return 0` | [x] `t_compare_equal_strictequal` |
| 204 | jsvalue.c:707 | js_strictequal | type matched none of undefined/null/number/boolean/object | `return 0` | [x] `t_compare_equal_strictequal` |
| 205 | jsproperty.c:57 | lookup | name walked to the `&sentinel` leaf without a strcmp match | `return NULL` — property not present in this object's tree | [x] `t_property_tree_and_extensibility` |
| 206 | jsproperty.c:92 | insert | inserting a name that already exists (`strcmp == 0`) | `return *result = node` — no new node allocated, existing property reused | [x] `t_property_tree_and_extensibility` |
| 207 | jsproperty.c:196 | jsV_getpropertyx | name absent from the object and its entire prototype chain | `return NULL` with `*own` left 0 | [x] `t_property_tree_and_extensibility` |
| 208 | jsproperty.c:207 | jsV_getproperty | name absent from the object and its entire prototype chain | `return NULL` | [x] `t_property_tree_and_extensibility` |
| 209 | jsproperty.c:214 | jsV_getenumproperty | property found but carries `JS_DONTENUM` | not returned; walk continues up the prototype chain | [x] `t_property_tree_and_extensibility` |
| 210 | jsproperty.c:218 | jsV_getenumproperty | no enumerable property with that name anywhere in the chain | `return NULL` | [x] `t_property_tree_and_extensibility` |
| 211 | jsproperty.c:227 | jsV_setproperty | creating a new property on a non-extensible object (`obj->extensible == 0`) while `J->strict` | `js_typeerror(J, "object is non-extensible")` → TypeError "object is non-extensible" | [x] `t_property_tree_and_extensibility` |
| 212 | jsproperty.c:229 | jsV_setproperty | creating a new property on a non-extensible object in non-strict mode | `return result` which is NULL — the write is silently dropped by callers that test `ref` | [x] `t_property_tree_and_extensibility` |
| 213 | jsproperty.c:256 | itwalk | property carries `JS_DONTENUM` | omitted from the iterator's name list (invisible to `for..in`) | [x] `t_property_tree_and_extensibility` |
| 214 | jsproperty.c:257 | itwalk | prototype-chain property shadowed by an enumerable property already `seen` on a derived object | omitted from the iterator list (no duplicate enumeration) | [x] `t_property_tree_and_extensibility` |
| 215 | jsproperty.c:302 | jsV_nextiterator | `io->type != JS_CITERATOR` — object passed to js_nextiterator is not an iterator | `js_typeerror(J, "not an iterator")` → TypeError "not an iterator" | [x] `t_iterators` |
| 216 | jsproperty.c:312 | jsV_nextiterator | queued name no longer resolves on the target (property deleted mid-iteration) | name skipped, loop continues to the next queued name | [x] `t_iterators` |
| 217 | jsproperty.c:315 | jsV_nextiterator | index range and name queue both exhausted | `return NULL` — signals end of iteration | [x] `t_iterators` |
| 218 | jsproperty.c:325 | jsV_resizearray | called on a simple/flat array (`obj->u.a.simple` true) | `abort()` via `assert(!obj->u.a.simple)` | [-] jsproperty.c:325 assert(!obj->u.a.simple) in jsV_resizearray. The only in-library caller (jsrun.c:715) is the else branch of `if (obj->u.a.simple)`, so the assert is unreachable through the public API. jsV_resizearray IS exported, so it can be called directly on a simple array, but that is a deliberate invariant violation whose C outcome is abort(); the Rust translation carries no assert!s at all (it matches a -DNDEBUG build), so such a call would be a known non-input-driven difference rather than a bug. t_resizearray drives jsV_resizearray directly on NON-simple arrays. |
| 219 | jsproperty.c:326 | jsV_resizearray | `newlen >= obj->u.a.length` (growing or unchanged) | no properties deleted; only `obj->u.a.length` updated | [x] `t_resizearray` |
| 220 | jsproperty.c:331 | jsV_resizearray | enumerated key is below `newlen`, or is not the canonical decimal form of its number (`"01"`, `"x"`, `"1e2"`) | not deleted — key preserved across the truncation | [x] `t_resizearray` |
| 221 | jsintern.c:8 | js_putc | first byte written into a NULL buffer and js_malloc fails / exceeds memlimit | js_malloc → js_outofmemory → throws `"out of memory"` | [x] `t_intern` |
| 222 | jsintern.c:13 | js_putc | buffer full (`sb->n == sb->m`) and the doubling `js_realloc(J, sb, (sb->m *= 2) + soffsetof(js_Buffer, s))` exceeds the memlimit | js_realloc → js_outofmemory → throws `"out of memory"` (note `sb->m` has already been doubled) | [x] `t_intern` |
| 223 | jsintern.c:46 | jsS_newstringnode | interning a string with `strlen(string) > JS_STRLIMIT` (1<<28) | `js_rangeerror(J, "invalid string length")` → RangeError "invalid string length" | [x] `t_intern_string_limit` |
| 224 | jsintern.c:48 | jsS_newstringnode | allocation of `soffsetof(js_StringNode, string) + n + 1` bytes fails | js_malloc → js_outofmemory → throws `"out of memory"` | [x] `t_intern` |
| 225 | jsintern.c:87 | jsS_insert | string already interned (`strcmp == 0`) | `return *result = node->string` — no new node allocated, existing canonical pointer returned | [x] `t_intern` |
| 226 | jsintern.c:98 | dumpstringnode | child pointer equals `&jsS_sentinel` | recursion stops (sentinel is never printed) | [x] `t_dumpstrings_sentinel` |
| 227 | jsintern.c:112 | jsS_dumpstrings | `J->strings` is NULL or `&jsS_sentinel` (nothing interned) | prints only the empty `interned strings { }` block | [x] `t_dumpstrings_sentinel` |
| 228 | jsintern.c:119 | jsS_freestringnode | child pointer equals `&jsS_sentinel` | recursion stops — the static sentinel is never passed to js_free | [x] `t_dumpstrings_sentinel` |
| 229 | jsintern.c:126 | jsS_freestrings | `J->strings` NULL or `&jsS_sentinel` | no-op — nothing freed | [x] `t_dumpstrings_sentinel` |
| 230 | jsintern.c:133 | js_intern | `J->strings == NULL` on the very first intern | initialises the tree to `&jsS_sentinel` before inserting | [x] `t_intern` |
| 231 | jsgc.c:19 | jsG_freeproperty | `node->left->level == 0` / `node->right->level == 0` (child is the shared sentinel) | recursion stops — the static `sentinel` property node is never freed | [x] `t_gc_free_and_sweep` |
| 232 | jsgc.c:26 | jsG_freeiterator | `node == NULL` (empty iterator name list) | `while (node)` loop body never runs — nothing freed | [x] `t_gc_free_and_sweep` |
| 233 | jsgc.c:35 | jsG_freeobject | `obj->properties->level == 0` (property tree is just the sentinel) | property tree walk skipped | [x] `t_gc_free_and_sweep` |
| 234 | jsgc.c:42 | jsG_freeobject | JS_CSTRING whose `u.s.string == u.s.shrstr` (short string stored inline) | `js_free` skipped — the inline buffer is not passed to the allocator | [x] `t_gc_free_and_sweep` |
| 235 | jsgc.c:45 | jsG_freeobject | JS_CARRAY that is not `simple` (already unflattened) | `js_free(J, obj->u.a.array)` skipped — flat array pointer is only valid in simple mode | [x] `t_gc_free_and_sweep` |
| 236 | jsgc.c:49 | jsG_freeobject | JS_CUSERDATA with `obj->u.user.finalize == NULL` | host finalizer not invoked; only the object itself is freed | [x] `t_gc_runs_host_finalizers` |
| 237 | jsgc.c:51 | jsG_freeobject | JS_CCFUNCTION with `obj->u.c.finalize == NULL` | host finalizer not invoked | [x] `t_gc_runs_host_finalizers` |
| 238 | jsgc.c:69 | jsG_markfunction | nested function already carries the current `mark` | recursion skipped — prevents infinite recursion on cyclic function tables | [x] `t_gc_free_and_sweep` |
| 239 | jsgc.c:77 | jsG_markenvironment | `env->variables->gcmark == mark` (variables object already marked) | jsG_markobject skipped for that object | [x] `t_gc_free_and_sweep` |
| 240 | jsgc.c:80 | jsG_markenvironment | `env->outer == NULL` or the outer env already carries `mark` | loop terminates — prevents cycles in the scope chain from looping forever | [x] `t_gc_free_and_sweep` |
| 241 | jsgc.c:88 | jsG_markproperty | property value is JS_TMEMSTR already marked | re-mark skipped | [x] `t_gc_free_and_sweep` |
| 242 | jsgc.c:90 | jsG_markproperty | property value is JS_TOBJECT already marked | not re-queued on the gcroot scan list | [x] `t_gc_free_and_sweep` |
| 243 | jsgc.c:92 | jsG_markproperty | `node->getter == NULL` or already marked | getter not queued | [x] `t_gc_free_and_sweep` |
| 244 | jsgc.c:94 | jsG_markproperty | `node->setter == NULL` or already marked | setter not queued | [x] `t_gc_free_and_sweep` |
| 245 | jsgc.c:101 | jsG_scanobject | `obj->properties->level == 0` (sentinel-only tree) | property marking skipped | [x] `t_gc_free_and_sweep` |
| 246 | jsgc.c:103 | jsG_scanobject | `obj->prototype == NULL` or already marked | prototype not queued | [x] `t_gc_free_and_sweep` |
| 247 | jsgc.c:115 | jsG_scanobject | JS_CITERATOR whose `u.iter.target` already carries `mark` | target not re-queued | [x] `t_gc_free_and_sweep` |
| 248 | jsgc.c:119 | jsG_scanobject | JS_CFUNCTION/JS_CSCRIPT with `obj->u.f.scope == NULL` (or already marked) | scope chain not marked | [x] `t_gc_free_and_sweep` |
| 249 | jsgc.c:121 | jsG_scanobject | JS_CFUNCTION/JS_CSCRIPT with `obj->u.f.function == NULL` (or already marked) | function/bytecode not marked | [x] `t_gc_free_and_sweep` |
| 250 | jsgc.c:183 | js_gc | `J->gcroot == NULL` — scan queue drained | mark phase loop exits | [x] `t_gc_sweep_report` |
| 251 | jsgc.c:194 | js_gc | environment record with `env->gcmark != mark` (unreachable) | unlinked from `J->gcenv` and freed via jsG_freeenvironment | [x] `t_gc_sweep_report` |
| 252 | jsgc.c:207 | js_gc | function with `fun->gcmark != mark` (unreachable) | unlinked from `J->gcfun` and freed via jsG_freefunction | [x] `t_gc_sweep_report` |
| 253 | jsgc.c:221 | js_gc | object with `obj->gcmark != mark` (unreachable) | unlinked from `J->gcobj` and freed via jsG_freeobject (runs host finalizers) | [x] `t_gc_runs_host_finalizers` |
| 254 | jsgc.c:235 | js_gc | js_String with `str->gcmark != mark` (unreachable) | unlinked from `J->gcstr` and freed | [x] `t_gc_sweep_report` |
| 255 | jsgc.c:250 | js_gc | end of collection — sets the next allocation budget `J->gcthresh = remaining * JS_GCFACTOR` (JS_GCFACTOR == 5.0) | subsequent allocations past this threshold force another `js_gc` from jsR_run (row 123) | [x] `t_forced_gc_in_run_loop` |
| 256 | jsgc.c:255 | js_gc | `report` non-zero while `ntot == 0` (nothing tracked at all) | `100*gtot/ntot` divides by zero — SIGFPE / undefined behaviour instead of a report | [-] jsgc.c:255 100*gtot/ntot with ntot == 0 is unreachable from js_newstate. ntot = nenv+nfun+nobj+nstr+nprop and js_newstate unconditionally creates J->R, J->G, the global environment and the whole builtin tree before returning (jsstate.c:229-234), so J->gcobj and J->gcenv are never both empty for any state a caller can hold. t_gc_sweep_report asserts ntot > 0 for every state it collects. |
| 257 | jsgc.c:255 | js_gc | report line longer than 255 bytes | `snprintf(buf, sizeof buf, ...)` with `char buf[256]` silently truncates the report | [-] jsgc.c:255 report line longer than 255 bytes is unreachable: the format string is 62 fixed bytes plus 11 %d conversions of unsigned int, i.e. at most 62 + 11*11 = 183 bytes. t_gc_sweep_report compares the full report text for many states instead. |
| 258 | jsgc.c:267 | js_freestate | `J == NULL` | `return` immediately — safe no-op on a null state | [x] `t_newstate_alloc_and_report` |
| 259 | jslex.c:6 | jsY_error | any lexical rejection (shared helper); formats message with vsnprintf into msgbuf[256], prefixes `"%s:%d: "` with J->filename and J->lexline into buf[512] via snprintf+strcat | js_newsyntaxerror(J, buf) then js_throw(J): throws a SyntaxError object whose message is `"<filename>:<lexline>: <msg>"`; JS_NORETURN | [x] `t_error_prefix_filename_line_and_truncation` |
| 260 | jslex.c:68 | jsY_tokenstring | token < 0 or token >= nelem(tokenstring) (i.e. >= 157) | returns the literal string `"<unknown>"` (no throw) | [x] `t_tokenstring_unknown` |
| 261 | jslex.c:69 | jsY_tokenstring | token in range but tokenstring[token] == NULL (the 0x80..0xFF filler slots initialised to 0) | returns the literal string `"<unknown>"` (no throw) | [x] `t_tokenstring_unknown` |
| 262 | jslex.c:95 | jsY_findword | binary search over sorted list finds no exact strcmp match for s | returns -1 (failure signal; used by jsY_findkeyword and jscompile.c checkfutureword) | [x] `t_findword_no_match` |
| 263 | jslex.c:154 | jsY_tohex | c is not one of 0-9 a-f A-F | returns 0 silently (no error; masks a bad digit if callers do not pre-check with jsY_ishex) | [x] `t_tohex_non_hex` |
| 264 | jslex.c:160-162 | jsY_next | current source byte is NUL (end of the NUL-terminated source buffer) | sets J->lexchar = EOF and returns without advancing J->source | [x] `t_eof_at_end_of_source` |
| 265 | jslex.c:177 | jsY_expect (macro) | J->lexchar != x, i.e. the required literal character is absent | jsY_error → SyntaxError with format `"expected '%c'"` (x) | [x] `t_json_keyword_expects` |
| 266 | jslex.c:181,191-192 | jsY_unescape | a `\` appears in identifier position but the next char is not `u` (e.g. `a\b`) | jsY_error → SyntaxError `"unexpected escape sequence"` | [x] `t_unescape_identifier_errors` |
| 267 | jslex.c:184 | jsY_unescape | 1st digit of `\uXXXX` identifier escape is not a hex digit | goto error → SyntaxError `"unexpected escape sequence"` | [x] `t_unescape_identifier_errors` |
| 268 | jslex.c:185 | jsY_unescape | 2nd digit of `\uXXXX` identifier escape is not a hex digit | goto error → SyntaxError `"unexpected escape sequence"` | [x] `t_unescape_identifier_errors` |
| 269 | jslex.c:186 | jsY_unescape | 3rd digit of `\uXXXX` identifier escape is not a hex digit | goto error → SyntaxError `"unexpected escape sequence"` | [x] `t_unescape_identifier_errors` |
| 270 | jslex.c:187 | jsY_unescape | 4th digit of `\uXXXX` identifier escape is not a hex digit | goto error → SyntaxError `"unexpected escape sequence"` | [x] `t_unescape_identifier_errors` |
| 271 | jslex.c:198-201 | textinit | J->lexbuf.text is NULL on first use | allocates cap = 4096 via js_malloc; if memlimit exceeded or malloc fails, js_outofmemory throws the raw literal string `"out of memory"` | [x] `t_lexbuf_allocation_out_of_memory` |
| 272 | jslex.c:212-216 | textpush | J->lexbuf.len + runelen(c) > J->lexbuf.cap (token text longer than current buffer) | doubles cap and js_realloc; on memlimit/allocation failure js_outofmemory throws raw literal `"out of memory"`; no upper limit (JS_STRLIMIT is not applied here) | [x] `t_lexbuf_allocation_out_of_memory` |
| 273 | jslex.c:238-248 | lexcomment | EOF reached before a closing `*/` is seen | returns -1 (failure signal to jsY_lexx) | [x] `t_block_comment_not_terminated` |
| 274 | jslex.c:254-255 | lexhex | `0x` / `0X` prefix not followed by at least one hex digit (e.g. `0x`, `0xg`) | jsY_error → SyntaxError `"malformed hexadecimal number"` | [x] `t_number_literal_errors` |
| 275 | jslex.c:350-351 | lexnumber | leading `0` immediately followed by a decimal digit (e.g. `01`, `08`) | jsY_error → SyntaxError `"number with leading zero"` | [x] `t_number_literal_errors` |
| 276 | jslex.c:357-358 | lexnumber | `.` not followed by a decimal digit | returns the punctuation token `'.'` instead of TK_NUMBER (non-error failure of the number rule) | [x] `t_number_literal_errors` |
| 277 | jslex.c:376-377 | lexnumber | `e`/`E` (with optional `+`/`-`) not followed by a decimal digit (e.g. `1e`, `1e+`) | jsY_error → SyntaxError `"missing exponent"` | [x] `t_number_literal_errors` |
| 278 | jslex.c:380-381 | lexnumber | identifier-start character immediately follows the numeric literal (e.g. `1x`, `3abc`, `1$`) | jsY_error → SyntaxError `"number with letter suffix"` | [x] `t_number_literal_errors` |
| 279 | jslex.c:399 | lexescape | EOF immediately after `\` inside a string | jsY_error → SyntaxError `"unterminated escape sequence"` | [x] `t_string_literal_errors` |
| 280 | jslex.c:402 | lexescape | 1st digit of string `\uXXXX` is not hex | returns 1 (caller lexstring turns it into an error) | [x] `t_string_literal_errors` |
| 281 | jslex.c:403 | lexescape | 2nd digit of string `\uXXXX` is not hex | returns 1 (caller lexstring turns it into an error) | [x] `t_string_literal_errors` |
| 282 | jslex.c:404 | lexescape | 3rd digit of string `\uXXXX` is not hex | returns 1 (caller lexstring turns it into an error) | [x] `t_string_literal_errors` |
| 283 | jslex.c:405 | lexescape | 4th digit of string `\uXXXX` is not hex | returns 1 (caller lexstring turns it into an error) | [x] `t_string_literal_errors` |
| 284 | jslex.c:410 | lexescape | 1st digit of string `\xXX` is not hex | returns 1 (caller lexstring turns it into an error) | [x] `t_string_literal_errors` |
| 285 | jslex.c:411 | lexescape | 2nd digit of string `\xXX` is not hex | returns 1 (caller lexstring turns it into an error) | [x] `t_string_literal_errors` |
| 286 | jslex.c:439-440 | lexstring | EOF or `\n` encountered before the closing quote (unterminated string literal) | jsY_error → SyntaxError `"string not terminated"` | [x] `t_string_literal_errors` |
| 287 | jslex.c:442-443 | lexstring | lexescape() returned 1 (malformed `\u` or `\x` escape) | jsY_error → SyntaxError `"malformed escape sequence"` | [x] `t_string_literal_errors` |
| 288 | jslex.c:449 | lexstring | jsY_expect(J, q): closing quote char q missing | jsY_error → SyntaxError `"expected '%c'"` (the quote char); unreachable in practice because the loop only exits when lexchar == q | [-] unreachable. jslex.c:449 jsY_expect(J, q) in lexstring runs only after the `while (J->lexchar != q)` loop exits, which happens exactly when lexchar == q, so the accept always succeeds. Documented + pinned by t_unreachable_closing_delimiter_expects (asserts no input ever produces "expected '\''" / "expected '\"'"). |
| 289 | jslex.c:489-490 | lexregexp | EOF or `\n` inside the regexp body before the closing `/` | jsY_error → SyntaxError `"regular expression not terminated"` | [x] `t_regexp_literal_errors` |
| 290 | jslex.c:496-497 | lexregexp | EOF or `\n` immediately after a `\` inside the regexp body | jsY_error → SyntaxError `"regular expression not terminated"` | [x] `t_regexp_literal_errors` |
| 291 | jslex.c:510 | lexregexp | jsY_expect(J, '/'): closing `/` missing | jsY_error → SyntaxError `"expected '%c'"` with '/'; unreachable because the loop only exits on lexchar == '/' | [-] unreachable. jslex.c:510 jsY_expect(J, '/') in lexregexp runs only after `while (J->lexchar != '/' || inclass)` exits, i.e. lexchar == '/'. Documented + pinned by t_unreachable_closing_delimiter_expects. |
| 292 | jslex.c:517-521 | lexregexp | identifier-part character in the flags position that is not `g`, `i` or `m` (e.g. `/a/x`) | jsY_error → SyntaxError `"illegal flag in regular expression: %c"` (J->lexchar) | [x] `t_regexp_literal_errors, t_regexp_flag_char_is_truncated_to_one_byte` |
| 293 | jslex.c:524-525 | lexregexp | duplicated flag: g > 1 or i > 1 or m > 1 (e.g. `/a/gg`) | jsY_error → SyntaxError `"duplicated flag in regular expression"` | [x] `t_regexp_literal_errors` |
| 294 | jslex.c:573-574 | jsY_lexx | lexcomment() returned -1: `/*` block comment reaches EOF without `*/` | jsY_error → SyntaxError `"multi-line comment not terminated"` | [x] `t_block_comment_not_terminated` |
| 295 | jslex.c:704-705 | jsY_lexx | J->lexchar == EOF at token start | returns token 0 (end-of-file token, not an error) | [x] `t_eof_at_end_of_source` |
| 296 | jslex.c:727-728 | jsY_lexx | character not matched by any token rule and in printable ASCII 0x20..0x7E (e.g. `@`, `#`, backtick) | jsY_error → SyntaxError `"unexpected character: '%c'"` (J->lexchar) | [x] `t_unexpected_character` |
| 297 | jslex.c:729 | jsY_lexx | character not matched by any rule and outside 0x20..0x7E (non-identifier Unicode rune, control char, or EOF reached via jsY_unescape) | jsY_error → SyntaxError `"unexpected character: \\u%04X"` (J->lexchar) | [x] `t_unexpected_character, t_raw_byte_sources` |
| 298 | jslex.c:756-760 | lexjsonnumber | first char after the optional `-` is neither `0` nor `1`-`9` (e.g. JSON input `-`, `-x`) | jsY_error → SyntaxError `"unexpected non-digit"` | [x] `t_json_number_errors` |
| 299 | jslex.c:762-767 | lexjsonnumber | `.` accepted but not followed by a digit (e.g. `1.`) | jsY_error → SyntaxError `"missing digits after decimal point"` | [x] `t_json_number_errors` |
| 300 | jslex.c:770-777 | lexjsonnumber | `e`/`E` (with optional sign) not followed by a digit (e.g. `1e`, `1e-`) | jsY_error → SyntaxError `"missing digits after exponent indicator"` | [x] `t_json_number_errors` |
| 301 | jslex.c:790-791 | lexjsonescape | escape char after `\` is not one of u " \ / b f n r t (e.g. `"\q"`) | jsY_error → SyntaxError `"invalid escape sequence"` | [x] `t_json_invalid_escape_sequence` |
| 302 | jslex.c:794 | lexjsonescape | 1st digit of JSON `\uXXXX` is not hex | returns 1; the return value is DISCARDED by lexjsonstring, so no error is raised and the offending chars stay in the stream | [x] `t_json_malformed_unicode_escape_is_silently_accepted` |
| 303 | jslex.c:795 | lexjsonescape | 2nd digit of JSON `\uXXXX` is not hex | returns 1; return value discarded by caller (no error) | [x] `t_json_malformed_unicode_escape_is_silently_accepted` |
| 304 | jslex.c:796 | lexjsonescape | 3rd digit of JSON `\uXXXX` is not hex | returns 1; return value discarded by caller (no error) | [x] `t_json_malformed_unicode_escape_is_silently_accepted` |
| 305 | jslex.c:797 | lexjsonescape | 4th digit of JSON `\uXXXX` is not hex | returns 1; return value discarded by caller (no error) | [x] `t_json_malformed_unicode_escape_is_silently_accepted` |
| 306 | jslex.c:819-820 | lexjsonstring | EOF before the closing `"` of a JSON string | jsY_error → SyntaxError `"unterminated string"` | [x] `t_json_string_errors` |
| 307 | jslex.c:821-822 | lexjsonstring | raw character with code < 32 inside a JSON string (unescaped control character) | jsY_error → SyntaxError `"invalid control character in string"` | [x] `t_json_string_errors` |
| 308 | jslex.c:823-824 | lexjsonstring | lexjsonescape(J) returns 1 for a malformed `\u` escape | return value ignored: lexing continues with the unconsumed characters (silent acceptance) | [x] `t_json_malformed_unicode_escape_is_silently_accepted` |
| 309 | jslex.c:830 | lexjsonstring | jsY_expect(J, '"'): closing quote missing | jsY_error → SyntaxError `"expected '%c'"` with '"'; unreachable because the loop only exits on lexchar == '"' | [-] unreachable. jslex.c:830 jsY_expect(J, '"') in lexjsonstring runs only after `while (J->lexchar != '"')` exits, i.e. lexchar == '"'. Documented + pinned by t_unreachable_closing_delimiter_expects. |
| 310 | jslex.c:862 | jsY_lexjson | JSON token starts with `f` but next char is not `a` | jsY_error → SyntaxError `"expected '%c'"` with 'a' | [x] `t_json_keyword_expects` |
| 311 | jslex.c:862 | jsY_lexjson | `fa` not followed by `l` | jsY_error → SyntaxError `"expected '%c'"` with 'l' | [x] `t_json_keyword_expects` |
| 312 | jslex.c:862 | jsY_lexjson | `fal` not followed by `s` | jsY_error → SyntaxError `"expected '%c'"` with 's' | [x] `t_json_keyword_expects` |
| 313 | jslex.c:862 | jsY_lexjson | `fals` not followed by `e` | jsY_error → SyntaxError `"expected '%c'"` with 'e' | [x] `t_json_keyword_expects` |
| 314 | jslex.c:866 | jsY_lexjson | JSON token starts with `n` but next char is not `u` | jsY_error → SyntaxError `"expected '%c'"` with 'u' | [x] `t_json_keyword_expects` |
| 315 | jslex.c:866 | jsY_lexjson | `nu` not followed by `l` | jsY_error → SyntaxError `"expected '%c'"` with 'l' | [x] `t_json_keyword_expects` |
| 316 | jslex.c:866 | jsY_lexjson | `nul` not followed by the second `l` | jsY_error → SyntaxError `"expected '%c'"` with 'l' | [x] `t_json_keyword_expects` |
| 317 | jslex.c:870 | jsY_lexjson | JSON token starts with `t` but next char is not `r` | jsY_error → SyntaxError `"expected '%c'"` with 'r' | [x] `t_json_keyword_expects` |
| 318 | jslex.c:870 | jsY_lexjson | `tr` not followed by `u` | jsY_error → SyntaxError `"expected '%c'"` with 'u' | [x] `t_json_keyword_expects` |
| 319 | jslex.c:870 | jsY_lexjson | `tru` not followed by `e` | jsY_error → SyntaxError `"expected '%c'"` with 'e' | [x] `t_json_keyword_expects` |
| 320 | jslex.c:873-874 | jsY_lexjson | J->lexchar == EOF at token start | returns token 0 (end-of-file, not an error) | [x] `t_json_unexpected_character_and_eof` |
| 321 | jslex.c:877-878 | jsY_lexjson | character not matched by any JSON token rule and in 0x20..0x7E (e.g. `'`, `x`, `+`) | jsY_error → SyntaxError `"unexpected character: '%c'"` (J->lexchar) | [x] `t_json_unexpected_character_and_eof` |
| 322 | jslex.c:879 | jsY_lexjson | character not matched by any JSON rule and outside 0x20..0x7E | jsY_error → SyntaxError `"unexpected character: \\u%04X"` (J->lexchar) | [x] `t_json_unexpected_character_and_eof, t_raw_byte_sources` |
| 323 | jsparse.c:29 | jsP_error | any syntactic rejection (shared helper); vsnprintf into msgbuf[256], prefix `"%s:%d: "` with J->filename and J->lexline into buf[512] | js_newsyntaxerror(J, buf) then js_throw(J): throws SyntaxError `"<filename>:<lexline>: <msg>"`; JS_NORETURN | [x] `t_error_prefix_filename_line_and_truncation, t_error_message_truncation_edges` |
| 324 | jsparse.c:46-58 | jsP_warning | non-fatal diagnostic helper; formats `"%s:%d: warning: %s"` (filename, lexline, msg) | js_report(J, buf); parsing continues (no throw) | [x] `t_function_statement_warning` |
| 325 | jsparse.c:62 | jsP_newnode | AST node allocation of sizeof(js_Ast) via js_malloc fails or exceeds J->memlimit | js_outofmemory throws the raw literal string `"out of memory"` (no node is returned) | [x] `t_astnode_allocation_out_of_memory` |
| 326 | jsparse.c:24 | INCREC (macro) | ++J->astdepth > JS_ASTLIMIT (400, jsi.h:107) — nesting/recursion depth of expressions and statements exceeded | jsP_error → SyntaxError `"too much recursion"` | [x] `t_recursion_limit_threshold` |
| 327 | jsparse.c:405 | memberexp | INCREC: member/index chain nesting pushes astdepth past 400 (e.g. `a.b.c...` 400+ deep) | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_expressions` |
| 328 | jsparse.c:419 | callexp | INCREC: call/member/index chain nesting pushes astdepth past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_expressions` |
| 329 | jsparse.c:441 | unary | INCREC: unary operator nesting past 400 (e.g. 401 `!` prefixes) | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_expressions` |
| 330 | jsparse.c:462 | multiplicative | INCREC: `*` `/` `%` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_expressions` |
| 331 | jsparse.c:477 | additive | INCREC: `+` `-` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 332 | jsparse.c:491 | shift | INCREC: `<<` `>>` `>>>` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 333 | jsparse.c:506 | relational | INCREC: `<` `>` `<=` `>=` `instanceof` `in` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 334 | jsparse.c:524 | equality | INCREC: `==` `!=` `===` `!==` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 335 | jsparse.c:540 | bitand | INCREC: `&` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 336 | jsparse.c:554 | bitxor | INCREC: `^` chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 337 | jsparse.c:568 | bitor | INCREC: bitwise-or chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 338 | jsparse.c:581 | logand | INCREC: `&&` right-recursion past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 339 | jsparse.c:593 | logor | INCREC: logical-or right-recursion past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 340 | jsparse.c:606 | conditional | INCREC: `?:` nesting past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 341 | jsparse.c:620 | assignment | INCREC: assignment-operator right-recursion past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 342 | jsparse.c:643 | expression | INCREC: comma-expression chain past 400 | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_operators` |
| 343 | jsparse.c:779 | statement | INCREC: statement nesting past 400 (e.g. 401 nested `{ }` or `if`) | SyntaxError `"too much recursion"` | [x] `t_recursion_limit_statements` |
| 344 | jsparse.c:143 | jsP_expect (macro) | J->lookahead != x, i.e. the required token is absent | jsP_error → SyntaxError `"unexpected token: %s (expected %s)"` with jsY_tokenstring(J->lookahead) and jsY_tokenstring(x) | [x] `t_parse_expect_sites` |
| 345 | jsparse.c:232 | propassign | getter shorthand `get name` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 346 | jsparse.c:233 | propassign | getter parameter list is not empty, i.e. `get name(` not immediately followed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 347 | jsparse.c:239 | propassign | setter shorthand `set name` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 348 | jsparse.c:241 | propassign | setter parameter list not closed after the single identifier | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 349 | jsparse.c:247 | propassign | object-literal property name not followed by `:` (and not a get/set shorthand) | SyntaxError `"unexpected token: %s (expected %s)"`, expected `':'` | [x] `t_parse_expect_sites` |
| 350 | jsparse.c:284 | fundec | function declaration name not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 351 | jsparse.c:286 | fundec | function declaration parameter list not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 352 | jsparse.c:295 | funstm | function statement name not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 353 | jsparse.c:297 | funstm | function statement parameter list not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 354 | jsparse.c:307 | funexp | function expression (optional name) not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 355 | jsparse.c:309 | funexp | function expression parameter list not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 356 | jsparse.c:349 | primary | object literal not closed by `}` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'}'` | [x] `t_parse_expect_sites` |
| 357 | jsparse.c:354 | primary | array literal not closed by `]` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `']'` | [x] `t_parse_expect_sites` |
| 358 | jsparse.c:359 | primary | parenthesised expression not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 359 | jsparse.c:387 | newexp | `new X(` argument list not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 360 | jsparse.c:408 | memberexp | index expression `a[expr` not closed by `]` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `']'` | [x] `t_parse_expect_sites` |
| 361 | jsparse.c:422 | callexp | index expression `a[expr` not closed by `]` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `']'` | [x] `t_parse_expect_sites` |
| 362 | jsparse.c:423 | callexp | call argument list `f(args` not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 363 | jsparse.c:608 | conditional | `cond ? a` not followed by `:` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `':'` | [x] `t_parse_expect_sites` |
| 364 | jsparse.c:689 | caseclause | `case expr` not followed by `:` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `':'` | [x] `t_parse_expect_sites` |
| 365 | jsparse.c:695 | caseclause | `default` not followed by `:` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `':'` | [x] `t_parse_expect_sites` |
| 366 | jsparse.c:718 | block | block statement does not start with `{` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'{'` | [x] `t_parse_expect_sites` |
| 367 | jsparse.c:720 | block | block statement not closed by `}` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'}'` | [x] `t_parse_expect_sites` |
| 368 | jsparse.c:729 | forexpression | for-header clause not terminated by the expected end token (`;` for init/cond, `)` for update) | SyntaxError `"unexpected token: %s (expected %s)"`, expected `';'` or `')'` | [x] `t_parse_expect_sites` |
| 369 | jsparse.c:736 | forstatement | `for` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 370 | jsparse.c:747 | forstatement | `for (var x in expr` not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 371 | jsparse.c:766 | forstatement | `for (lhs in expr` not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 372 | jsparse.c:797 | statement | `if` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 373 | jsparse.c:799 | statement | `if (expr` not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 374 | jsparse.c:810 | statement | `do stm` not followed by the `while` keyword | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'while'` | [x] `t_parse_expect_sites` |
| 375 | jsparse.c:811 | statement | `do stm while` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 376 | jsparse.c:813 | statement | do-while condition not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 377 | jsparse.c:819 | statement | `while` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 378 | jsparse.c:821 | statement | while condition not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 379 | jsparse.c:852 | statement | `with` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 380 | jsparse.c:854 | statement | with object expression not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 381 | jsparse.c:860 | statement | `switch` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 382 | jsparse.c:862 | statement | switch discriminant not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 383 | jsparse.c:863 | statement | switch head not followed by `{` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'{'` | [x] `t_parse_expect_sites` |
| 384 | jsparse.c:865 | statement | switch body not closed by `}` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'}'` | [-] unreachable. jsparse.c:865 jsP_expect(J, '}') runs after caselist(), whose only two exits are `J->lookahead == '}'` (the early return and the while condition), so the accept always succeeds. Documented + pinned by t_unreachable_switch_and_funbody_expects. |
| 385 | jsparse.c:879 | statement | `catch` not followed by `(` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'('` | [x] `t_parse_expect_sites` |
| 386 | jsparse.c:881 | statement | catch parameter not closed by `)` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `')'` | [x] `t_parse_expect_sites` |
| 387 | jsparse.c:949 | funbody | function body does not start with `{` | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'{'` | [x] `t_parse_expect_sites` |
| 388 | jsparse.c:951 | funbody | function body not closed by `}` (e.g. EOF inside a function) | SyntaxError `"unexpected token: %s (expected %s)"`, expected `'}'` | [-] unreachable. jsparse.c:951 jsP_expect(J, '}') in funbody runs after script(J, '}'), whose only exits are `J->lookahead == terminator`, so the accept always succeeds; EOF inside a function body is reported from inside the body instead. Documented + pinned by t_unreachable_switch_and_funbody_expects. |
| 389 | jsparse.c:147-153 | semicolon | statement not terminated: lookahead is not `;`, no preceding newline (J->newline == 0), and lookahead is neither `}` nor 0 (EOF) | jsP_error → SyntaxError `"unexpected token: %s (expected ';')"` with jsY_tokenstring(J->lookahead) | [x] `t_parse_identifier_and_semicolon` |
| 390 | jsparse.c:166 | identifier | lookahead is not TK_IDENTIFIER where a binding identifier is required (var name, parameter, catch var, function name, setter arg) | jsP_error → SyntaxError `"unexpected token: %s (expected identifier)"`; JS_NORETURN so no value is returned | [x] `t_parse_identifier_and_semicolon` |
| 391 | jsparse.c:171-173 | identifieropt | lookahead is not TK_IDENTIFIER (optional identifier absent, e.g. anonymous function, bare `break`) | returns NULL (no error) | [x] `t_parse_identifier_and_semicolon` |
| 392 | jsparse.c:178-183 | identifiername | lookahead is neither TK_IDENTIFIER nor a keyword token (>= TK_BREAK) where a property name is required (after `.`, in object literal) | jsP_error → SyntaxError `"unexpected token: %s (expected identifier or keyword)"` | [x] `t_parse_identifier_and_semicolon` |
| 393 | jsparse.c:197-198 | arrayliteral | lookahead is `]` immediately (empty array literal) | returns NULL element list (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 394 | jsparse.c:255-256 | objectliteral | lookahead is `}` immediately (empty object literal) | returns NULL property list (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 395 | jsparse.c:271-272 | parameters | lookahead is `)` immediately (empty parameter list) | returns NULL parameter list (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 396 | jsparse.c:363 | primary | lookahead cannot start a primary expression (e.g. `;`, `)`, `,`, an operator, EOF) | jsP_error → SyntaxError `"unexpected token in expression: %s"` with jsY_tokenstring(J->lookahead) | [x] `t_parse_empty_paths_and_diagnostics` |
| 397 | jsparse.c:369-370 | arguments | lookahead is `)` immediately (empty argument list) | returns NULL argument list (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 398 | jsparse.c:674-675 | statementlist | lookahead is `}`, `case` or `default` (empty statement list) | returns NULL (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 399 | jsparse.c:687-700 | caseclause | inside a switch body the next token is neither `case` nor `default` | jsP_error → SyntaxError `"unexpected token in switch: %s (expected 'case' or 'default')"` | [x] `t_parse_empty_paths_and_diagnostics` |
| 400 | jsparse.c:706-707 | caselist | lookahead is `}` immediately (switch with no clauses) | returns NULL (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 401 | jsparse.c:727-728 | forexpression | lookahead equals the end token (omitted for-header clause) | leaves a = NULL and only consumes the end token (no error) | [x] `t_parse_empty_paths_and_diagnostics` |
| 402 | jsparse.c:745-751 | forstatement | after `for (var declist` the next token is neither `;` nor `in` (e.g. `for (var i)`) | jsP_error → SyntaxError `"unexpected token in for-var-statement: %s"` with jsY_tokenstring(J->lookahead) | [x] `t_parse_empty_paths_and_diagnostics` |
| 403 | jsparse.c:764-770 | forstatement | after the non-var for init expression the next token is neither `;` nor `in` (e.g. `for (i)`) | jsP_error → SyntaxError `"unexpected token in for-statement: %s"` with jsY_tokenstring(J->lookahead) | [x] `t_parse_empty_paths_and_diagnostics` |
| 404 | jsparse.c:887-888 | statement | `try` block followed by neither a `catch` clause nor a `finally` clause | jsP_error → SyntaxError `"unexpected token in try: %s (expected 'catch' or 'finally')"` | [x] `t_parse_empty_paths_and_diagnostics` |
| 405 | jsparse.c:897-898 | statement | `function` keyword in statement position (non-standard function statement) | jsP_warning → js_report `"%s:%d: warning: function statements are not standard"`; parsing continues, rewritten as `var X = function X() {}` | [x] `t_function_statement_warning` |
| 406 | jsparse.c:938-939 | script | lookahead already equals the terminator (empty program or empty function body) | returns NULL AST (no error); jsP_parse then skips constant folding | [x] `t_parse_empty_paths_and_diagnostics` |
| 407 | jsparse.c:962-963 | toint32 | operand is NaN/Inf (!isfinite) or exactly 0 during constant folding | returns 0 (no error) | [x] `t_constfold_toint32` |
| 408 | jscompile.c:14 | jsC_error | any compile-time (semantic) rejection; vsnprintf into msgbuf[256], prefix `"%s:%d: "` with J->filename and node->line into buf[512] | js_newsyntaxerror(J, buf) then js_throw(J): throws SyntaxError `"<filename>:<node line>: <msg>"`; JS_NORETURN | [x] `t_error_prefix_filename_line_and_truncation, t_error_message_truncation_edges` |
| 409 | jscompile.c:42-43 | checkfutureword | identifier is one of the future reserved words: class, const, enum, export, extends, import, super | jsC_error → SyntaxError `"'%s' is a future reserved word"` (exp->string) | [x] `t_future_reserved_words` |
| 410 | jscompile.c:44-46 | checkfutureword | in strict mode, identifier is one of: implements, interface, let, package, private, protected, public, static, yield | jsC_error → SyntaxError `"'%s' is a strict mode future reserved word"` (exp->string) | [x] `t_future_reserved_words` |
| 411 | jscompile.c:74-75 | emitraw | emitted value does not round-trip through js_Instruction (unsigned short by default): value != (js_Instruction)value, e.g. an argument or line number > 65535 | js_syntaxerror(J, ...) → throws SyntaxError `"integer overflow in instruction coding"` (no filename prefix) | [x] `t_instruction_coding_overflow_via_line_number, t_instruction_coding_overflow_via_argument_count` |
| 412 | jscompile.c:76-79 | emitraw | F->codelen >= F->codecap (code buffer full) | grows codecap (0 -> 64, then doubling) with js_realloc; allocation failure or memlimit throws raw literal `"out of memory"`; no MAXCODE-style limit exists | [x] `t_compile_table_growth_and_out_of_memory` |
| 413 | jscompile.c:100-104 | addfunction | F->funlen >= F->funcap (nested-function table full) | grows funcap (0 -> 16, then doubling) via js_realloc; failure throws raw literal `"out of memory"`; no MAXFUN limit; index returned is used as an instruction arg so > 65535 functions later trips row 153 | [x] `t_compile_table_growth_and_out_of_memory` |
| 414 | jscompile.c:112-114 | addlocal | in strict mode a parameter or var is named `arguments` | jsC_error → SyntaxError `"redefining 'arguments' is not allowed in strict mode"` | [x] `t_addlocal_and_emitlocal` |
| 415 | jscompile.c:115-116 | addlocal | in strict mode a parameter or var is named `eval` | jsC_error → SyntaxError `"redefining 'eval' is not allowed in strict mode"` | [x] `t_addlocal_and_emitlocal` |
| 416 | jscompile.c:117-120 | addlocal | in non-strict mode a parameter or var is named `eval` | js_evalerror(J, "%s:%d: invalid use of 'eval'", J->filename, ident->line) → throws EvalError with that message | [x] `t_addlocal_and_emitlocal` |
| 417 | jscompile.c:121-127 | addlocal | reuse == 1 and the name already exists in F->vartab | returns the existing slot index i+1 (no new local, no error) | [x] `t_addlocal_and_emitlocal, t_local_slot_allocation` |
| 418 | jscompile.c:127-128 | addlocal | F->strict and the name already exists in vartab while reuse == 0 (duplicate formal parameter) | jsC_error → SyntaxError `"duplicate formal parameter '%s'"` (name) | [x] `t_addlocal_and_emitlocal` |
| 419 | jscompile.c:132-135 | addlocal | F->varlen >= F->varcap (local variable table full) | grows varcap (0 -> 16, then doubling) via js_realloc; failure throws raw literal `"out of memory"`; no MAXLOCAL/MAXVARS limit; slot index is emitted as an instruction arg so > 65535 locals later trips row 153 | [x] `t_compile_table_growth_and_out_of_memory` |
| 420 | jscompile.c:140-147 | findlocal | name is not present in F->vartab | returns -1, meaning "not a local"; callers then emit the by-name variable opcode | [x] `t_local_slot_allocation` |
| 421 | jscompile.c:157-162 | emitnumber | num == 0 (including -0.0 detected by signbit) | emits OP_INTEGER with arg 32768 (bias) and, for -0.0, an extra OP_NEG | [x] `t_emitnumber_shapes` |
| 422 | jscompile.c:163-165 | emitnumber | num within SHRT_MIN..SHRT_MAX and num == (int)num | emits OP_INTEGER with arg num + 32768; values outside this range fall through to the OP_NUMBER path emitting the raw double words | [x] `t_emitnumber_shapes` |
| 423 | jscompile.c:202-204 | emitlocal | F->strict and oploc == OP_SETLOCAL and the target identifier is `arguments` | jsC_error → SyntaxError `"'arguments' is read-only in strict mode"` | [x] `t_addlocal_and_emitlocal` |
| 424 | jscompile.c:205-206 | emitlocal | F->strict and oploc == OP_SETLOCAL and the target identifier is `eval` | jsC_error → SyntaxError `"'eval' is read-only in strict mode"` | [x] `t_addlocal_and_emitlocal` |
| 425 | jscompile.c:208-209 | emitlocal | identifier being referenced or assigned is named `eval` (any mode) | js_evalerror(J, "%s:%d: invalid use of 'eval'", J->filename, ident->line) → throws EvalError | [x] `t_addlocal_and_emitlocal` |
| 426 | jscompile.c:211-217 | emitlocal | findlocal returned < 0 (identifier is not a local slot) | emits the by-name variant (opvar with the interned string) instead of the indexed local op; no error | [x] `t_local_slot_allocation` |
| 427 | jscompile.c:236-238 | emitjumpto | jump destination does not fit in js_Instruction: dest != (js_Instruction)dest (code longer than 65535 instructions) | js_syntaxerror → SyntaxError `"jump address integer overflow"` | [x] `t_jump_address_overflow` |
| 428 | jscompile.c:243-245 | labelto | patched jump address does not fit in js_Instruction: addr != (js_Instruction)addr | js_syntaxerror → SyntaxError `"jump address integer overflow"` | [x] `t_jump_address_overflow` |
| 429 | jscompile.c:307-315 | checkdup | strict mode object literal contains two properties with the same key (numeric keys compared after jsV_numbertostring into a 32-byte buffer) | jsC_error → SyntaxError `"duplicate property '%s' in object literal"` (needle) | [x] `t_object_literal_duplicate_property, t_checkdup_at_scale_and_completion_values` |
| 430 | jscompile.c:329-336 | cobject | object literal key node is not AST_IDENTIFIER, EXP_STRING or EXP_NUMBER | jsC_error → SyntaxError `"invalid property name in object initializer"` | [-] unreachable. jscompile.c:336 "invalid property name in object initializer" needs cobject's key node to be something other than AST_IDENTIFIER / EXP_STRING / EXP_NUMBER, but the key always comes from propname() (jsparse.c:207) which produces exactly those three, and jsP_foldconst never rewrites a property-name node. Documented + pinned by t_unreachable_object_key_paths (sweeps every key shape the grammar admits). |
| 431 | jscompile.c:342-343 | cobject | kv->type is not EXP_PROP_VAL/GET/SET (marked "impossible") | silent `break`: nothing emitted for the property value, leaving the key on the stack | [-] unreachable. jscompile.c:343 the `default: /* impossible */ break;` of the kv->type switch needs a property node that is not EXP_PROP_VAL / EXP_PROP_GET / EXP_PROP_SET, and propassign() (jsparse.c:222) only ever builds those three. Documented + pinned by t_unreachable_object_key_paths. |
| 432 | jscompile.c:399-400 | cassign | assignment target is not EXP_IDENTIFIER, EXP_INDEX or EXP_MEMBER (e.g. `1 = 2`, `f() = 1`) | jsC_error → SyntaxError `"invalid l-value in assignment"` | [x] `t_invalid_lvalues` |
| 433 | jscompile.c:408-410 | cassignforin | STM_FOR_IN_VAR whose var declaration list has a second element (`for (var a, b in x)`) | jsC_error → SyntaxError `"more than one loop variable in for-in statement"` | [x] `t_for_in_var_multiple_variables` |
| 434 | jscompile.c:438-439 | cassignforin | for-in loop target is not EXP_IDENTIFIER, EXP_INDEX or EXP_MEMBER (e.g. `for (1 in x)`) | jsC_error → SyntaxError `"invalid l-value in for-in loop assignment"` | [x] `t_invalid_lvalues` |
| 435 | jscompile.c:463-464 | cassignop1 | compound-assignment / inc / dec target is not EXP_IDENTIFIER, EXP_INDEX or EXP_MEMBER (e.g. `1 += 2`, `++1`) | jsC_error → SyntaxError `"invalid l-value in assignment"` | [x] `t_invalid_lvalues` |
| 436 | jscompile.c:486-487 | cassignop2 | same invalid target reached on the store side of a compound assignment / postfix operator | jsC_error → SyntaxError `"invalid l-value in assignment"` | [-] unreachable. jscompile.c:487 cassignop2's `default:` has exactly the same EXP_IDENTIFIER / EXP_INDEX / EXP_MEMBER switch as cassignop1 (jscompile.c:445) and is only reached after cassignop1 already accepted the SAME node, so it can never be taken. Documented + pinned by t_unreachable_compile_defaults (asserts the diagnostic always comes from cassignop1). |
| 437 | jscompile.c:506-508 | cdelete | `delete x` where x is a bare identifier and F->strict | jsC_error → SyntaxError `"delete on an unqualified name is not allowed in strict mode"` | [x] `t_invalid_lvalues` |
| 438 | jscompile.c:523-524 | cdelete | delete operand is not EXP_IDENTIFIER, EXP_INDEX or EXP_MEMBER (e.g. `delete 1`) | jsC_error → SyntaxError `"invalid l-value in delete expression"` | [x] `t_invalid_lvalues` |
| 439 | jscompile.c:779-780 | cexp | expression node type is not handled by any case of the switch (unknown/statement node in expression position) | jsC_error → SyntaxError `"unknown expression type"` | [-] unreachable. jscompile.c:780 cexp's `default: "unknown expression type"` needs an AST node in expression position with no `case`; every node the parser can put there (all EXP_*, incl. EXP_ELISION) has one, and every node cstm's own `default:` forwards to cexp is an expression. Documented + pinned by t_unreachable_compile_defaults (sweeps 60+ expression shapes in 5 positions). |
| 440 | jscompile.c:832-846 | breaktarget | walk up the parent chain hits a function boundary (isfun) or the root without finding a loop/switch (unlabelled) or a matching label | returns NULL (failure signal consumed at jscompile.c:1217 / 1221) | [x] `t_break_continue_return_targets` |
| 441 | jscompile.c:849-862 | continuetarget | walk up the parent chain hits a function boundary or the root without finding a loop (unlabelled) or a loop with the matching label | returns NULL (failure signal consumed at jscompile.c:1233 / 1237) | [x] `t_break_continue_return_targets` |
| 442 | jscompile.c:865-872 | returntarget | walk up the parent chain reaches the root without finding an enclosing function node | returns NULL (failure signal consumed at jscompile.c:1251) | [x] `t_break_continue_return_targets` |
| 443 | jscompile.c:882-885 | cexit | node type on the unwind path is not STM_WITH / STM_FOR_IN / STM_FOR_IN_VAR / STM_TRY (marked "impossible") | silent `break`: no stack/scope rebalancing code emitted for that frame | [x] `t_break_continue_return_targets` |
| 444 | jscompile.c:1023-1025 | cswitch | switch body contains a second `default` clause | jsC_error → SyntaxError `"more than one default label in switch"` | [x] `t_switch_multiple_defaults` |
| 445 | jscompile.c:1213-1217 | cstm (STM_BREAK) | `break label` where breaktarget() returned NULL (no enclosing statement carries that label) | jsC_error → SyntaxError `"break label '%s' not found"` (stm->a->string) | [x] `t_break_continue_return_targets` |
| 446 | jscompile.c:1218-1221 | cstm (STM_BREAK) | bare `break` where breaktarget() returned NULL (not inside a loop or switch) | jsC_error → SyntaxError `"unlabelled break must be inside loop or switch"` | [x] `t_break_continue_return_targets` |
| 447 | jscompile.c:1229-1233 | cstm (STM_CONTINUE) | `continue label` where continuetarget() returned NULL (no enclosing loop carries that label) | jsC_error → SyntaxError `"continue label '%s' not found"` (stm->a->string) | [x] `t_break_continue_return_targets` |
| 448 | jscompile.c:1234-1237 | cstm (STM_CONTINUE) | bare `continue` where continuetarget() returned NULL (not inside a loop) | jsC_error → SyntaxError `"continue must be inside loop"` | [x] `t_break_continue_return_targets` |
| 449 | jscompile.c:1249-1251 | cstm (STM_RETURN) | `return` outside any function (returntarget() returned NULL, i.e. at script top level) | jsC_error → SyntaxError `"return not in function"` | [x] `t_break_continue_return_targets` |
| 450 | jscompile.c:1263-1266 | cstm (STM_WITH) | `with` statement while F->strict | jsC_error → SyntaxError `"'with' statements are not allowed in strict mode"` (node used for the line number is stm->a) | [x] `t_with_in_strict_mode` |
| 451 | jscompile.c:1277-1285 | cstm (STM_TRY) | try node lacks a catch clause or catch var (not (stm->b && stm->c)) | falls into ctryfinally(J, F, stm->a, stm->d); relies on the parser (jsparse.c:887) having guaranteed stm->d is non-NULL, otherwise cstm dereferences NULL | [x] `t_break_continue_return_targets` |
| 452 | jscompile.c:1293-1302 | cstm | statement node type matches no case (an expression statement) | compiled as an expression, with OP_POP ordering depending on F->script (no error) | [x] `t_checkdup_at_scale_and_completion_values` |
| 453 | jscompile.c:1214, 1230 | cstm | `break`/`continue` label identifier is a future reserved word | checkfutureword → SyntaxError `"'%s' is a future reserved word"` / `"'%s' is a strict mode future reserved word"` | [x] `t_future_reserved_words` |
| 454 | jscompile.c:958-963 | ctrycatch | catch variable is a future reserved word, or in strict mode is named `arguments` or `eval` | checkfutureword error, else jsC_error `"redefining 'arguments' is not allowed in strict mode"` / `"redefining 'eval' is not allowed in strict mode"` | [x] `t_trycatchfinally_futureword_asymmetry` |
| 455 | jscompile.c:990-995 | ctrycatchfinally | in strict mode the catch variable is a future reserved word, or is named `arguments` or `eval` (note: checkfutureword is only called when F->strict here, unlike ctrycatch) | jsC_error → `"'%s' is a future reserved word"` / `"redefining 'arguments' is not allowed in strict mode"` / `"redefining 'eval' is not allowed in strict mode"` | [x] `t_trycatchfinally_futureword_asymmetry` |
| 456 | jscompile.c:1327-1330 | cparams | a parameter name is a future reserved word, `eval`, `arguments`, or a strict-mode duplicate | checkfutureword + addlocal errors (rows 151, 152, 156, 157, 158, 160) | [x] `t_params_vardecs_fundecs_and_selfbinding` |
| 457 | jscompile.c:1347-1349 | cvardecs | a `var` declaration name is a future reserved word, `eval`, or `arguments` in strict mode | checkfutureword + addlocal(reuse=1) errors (rows 151, 152, 156, 157, 158) | [x] `t_params_vardecs_fundecs_and_selfbinding` |
| 458 | jscompile.c:1362-1367 | cfundecs | a hoisted function declaration name is a future reserved word / `eval` / `arguments` in strict mode | addlocal(reuse=1) errors (rows 156, 157, 158) | [x] `t_params_vardecs_fundecs_and_selfbinding` |
| 459 | jscompile.c:1383-1385 | cfunbody | first statement of the body is the string literal `"use strict"` | sets F->strict = 1, enabling all strict-mode rejections above (no error) | [x] `t_use_strict_directive` |
| 460 | jscompile.c:1396-1405 | cfunbody | named function expression whose own name is a future reserved word | checkfutureword → SyntaxError `"'%s' is a future reserved word"` / `"'%s' is a strict mode future reserved word"`; if findlocal(name) < 0 a self-binding local is added instead | [x] `t_params_vardecs_fundecs_and_selfbinding` |
| 461 | jsdtoa.c:386 | minus | x.e != y.e (diy_fp operands not at the same binary exponent) | assert fires: with NDEBUG unset, abort() via the C library (process termination, not a JS exception); with NDEBUG the subtraction proceeds with mismatched exponents | [-] assert() -> abort(). jsdtoa.c:386 `assert(x.e == y.e)` in the static minus(); c_src is built with no -DNDEBUG, so a violation terminates the process instead of raising a JS exception, and an aborting process cannot be differentially compared. The only inputs that break the precondition are the ones js_grisu2's contract excludes (0.0 / negative / non-finite; jsV_numbertostring special-cases all three and passes fabs(v)). Documented + the in-contract domain is verified by t_grisu_invariants_hold_inside_the_contract. |
| 462 | jsdtoa.c:387 | minus | x.f < y.f (subtraction would underflow the unsigned significand) | assert fires: abort() when asserts are enabled; otherwise r.f wraps around modulo 2^64 | [-] assert() -> abort(). jsdtoa.c:387 `assert(x.f >= y.f)` in the static minus(); same reasoning as row 461 (verified experimentally: js_grisu2(-1.0, ...) aborts the C process with "jsdtoa.c:387: minus: Assertion `x.f >= y.f' failed"). Documented + t_grisu_invariants_hold_inside_the_contract. |
| 463 | jsdtoa.c:370-377 | cached_power | index = 343 + k outside 0..(nelem(powers_ten)-1) | NO bounds check: reads powers_ten[index] / powers_ten_e[index] out of bounds (undefined behaviour); callers are expected to pass only k values derived from k_comp on finite doubles | [-] C UNDEFINED BEHAVIOUR, deliberately not tested. jsdtoa.c:370-377 cached_power() indexes powers_ten[343 + k] / powers_ten_e[343 + k] with no bounds check; an out-of-table k is an out-of-bounds read. Documented in the doc comment of t_grisu_invariants_hold_inside_the_contract. |
| 464 | jsdtoa.c:480 | digit_gen | Mp.e >= 0 would make the shift count `-Mp.e` non-positive | NO check: `((uint64_t)1) << -Mp.e` is undefined behaviour; the grisu invariants (alpha=-59, gamma=-56) are assumed to guarantee Mp.e < 0 | [-] C UNDEFINED BEHAVIOUR, deliberately not tested. jsdtoa.c:480 digit_gen() computes `((uint64_t)1) << -Mp.e`, which is UB whenever Mp.e >= 0; the grisu invariants (alpha=-59, gamma=-56) are assumed to guarantee Mp.e < 0. Documented in t_grisu_invariants_hold_inside_the_contract. |
| 465 | jsdtoa.c:486, 495 | digit_gen | digit output beyond the caller's buffer capacity | NO bounds check on `buffer[(*len)++]`: the caller must supply a buffer large enough (js_grisu2 callers use at least 32 bytes) | [-] C UNDEFINED BEHAVIOUR, deliberately not tested. jsdtoa.c:486,495 digit_gen() writes `buffer[(*len)++]` with no bounds check; overflowing the caller's buffer is UB. Documented in t_grisu_invariants_hold_inside_the_contract. |
| 466 | jsdtoa.c:36-43 | js_fmtexp | exponent needs more than 9 decimal digits | NO bounds check on se[9]: digits are written into a fixed 9-byte array (overflow would be undefined behaviour; unreachable for double exponents) | [-] C UNDEFINED BEHAVIOUR, deliberately not tested. jsdtoa.c:36-43 js_fmtexp() writes the exponent digits into `char se[9]`, so a 10-digit exponent (|e| >= 1e9) writes se[9], one byte past the array. Already documented in tests/ll_num.rs t_fmtexp; re-documented in t_grisu_invariants_hold_inside_the_contract. |
| 467 | jsdtoa.c:40-41 | js_fmtexp | e == 0, so the digit loop produced no digits | pads with a single `'0'`, producing `"e+0"` | [x] `t_fmtexp_zero_exponent` |
| 468 | jsdtoa.c:596-598 | js_strtod | leading space, tab, newline or carriage return characters | skipped silently before sign/digit parsing (no error) | [x] `t_strtod_scanning` |
| 469 | jsdtoa.c:617-623 | js_strtod | non-digit character, or a second `.`, encountered while measuring the mantissa | scan loop breaks there; the rest of the string is not part of the number | [x] `t_strtod_scanning` |
| 470 | jsdtoa.c:641-643 | js_strtod | mantissa has more than 18 significant digits | mantSize clamped to 18 and fracExp = decPt - 18: the extra digits are dropped (precision loss, no error) | [x] `t_strtod_18_digit_mantissa_clamp` |
| 471 | jsdtoa.c:647-650 | js_strtod | mantSize == 0, i.e. no digits at all (empty string, sign only, `.` only, or non-numeric text) | fraction = 0.0, p reset to the original `string`, goto done: returns 0.0 (or -0.0 when a `-` was seen) and sets *endPtr = string, signalling "no conversion performed" | [x] `t_strtod_scanning` |
| 472 | jsdtoa.c:683-700 | js_strtod | `E`/`e` present but not followed by any digit | the digit loops never run so exp stays 0, yet p has already advanced past the `e` and the sign: *endPtr points after the bogus exponent characters (they are silently consumed) | [x] `t_strtod_scanning` |
| 473 | jsdtoa.c:694 | js_strtod | exponent digit accumulation would push exp to or past INT_MAX/100 | accumulation stops (overflow guard) and the remaining digits are skipped by the loop at lines 698-699 | [x] `t_strtod_exponent_clamping` |
| 474 | jsdtoa.c:714-717 | js_strtod | combined exponent exp < -maxExponent (maxExponent = 511, jsdtoa.c:536) | exp forced to 511 with expSign = TRUE and errno = ERANGE: result underflows toward 0 (fraction / 1e511) | [x] `t_strtod_exponent_clamping` |
| 475 | jsdtoa.c:718-721 | js_strtod | combined exponent exp > maxExponent (511) | exp forced to 511 with expSign = FALSE and errno = ERANGE: result overflows to HUGE_VAL/inf (fraction * 1e511) | [x] `t_strtod_exponent_clamping` |
| 476 | jsdtoa.c:741-743 | js_strtod | endPtr == NULL (the calling convention used by jslex.c lexnumber and lexjsonnumber) | the end pointer is simply not stored; callers cannot detect a partial or failed conversion | [x] `t_strtod_null_endptr_callers, t_strtod_scanning` |
| 477 | jsarray.c:11 | js_getlength | `this.length` is NaN, negative, or > INT_MAX; `js_tointeger` funnels through `jsV_numbertointeger` (jsvalue.c:41) | no throw; NaN -> 0, `n < INT_MIN` -> `INT_MIN`, `n > INT_MAX` -> `INT_MAX`; silently truncated `int len` | [x] `t_array_getlength_coercion` |
| 478 | jsarray.c:19 | js_setlength | `js_setproperty(J, idx, "length")` on a JS_CARRAY with a non-integral/negative computed length | propagates RangeError `"invalid array length"` from jsrun.c:707 | [x] `t_array_setlength_rangeerrors` |
| 479 | jsarray.c:19 | js_setlength | computed length > `JS_ARRAYLIMIT` (1<<26) | propagates RangeError `"array too large"` from jsrun.c:709 | [x] `t_array_setlength_rangeerrors` |
| 480 | jsarray.c:31 | jsB_new_Array | `new Array(x)` with exactly one numeric arg (`top == 2 && js_isnumber(J,1)`) where x is fractional or negative, e.g. `new Array(-1)` / `new Array(1.5)` | RangeError `"invalid array length"` (raised in jsrun.c:707 via `js_setproperty(...,"length")`) | [x] `t_array_constructor_single_arg` |
| 481 | jsarray.c:31 | jsB_new_Array | `new Array(x)` with single numeric arg x > 1<<26 | RangeError `"array too large"` (jsrun.c:709) | [x] `t_array_constructor_single_arg` |
| 482 | jsarray.c:34 | jsB_new_Array | `new Array(x)` with single NON-number arg: takes `js_setindex(J,-2,0)` branch instead of setting length | no error; produces 1-element array (behaviour fork on `js_isnumber`) | [x] `t_array_constructor_single_arg` |
| 483 | jsarray.c:71 | Ap_join_cycle | `Array.prototype.join.call(undefined)` / `.call(null)` — `js_toobject(J, 0)` on a non-coercible `this` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` (jsvalue.c:401/402) | [x] `t_array_join_paths` |
| 484 | jsarray.c:76 | Ap_join_cycle | trace frame's callee slot `&J->stack[stk-1]` is not `JS_TOBJECT` | `return 0` — cycle detection abandoned, join proceeds (may recurse) | [-] jsarray.c:76 is unreachable -- every trace frame with index > 0 was pushed by jsR_pushtrace from js_call/js_construct, which set BOT = TOP-n-1 only after js_iscallable accepted the value in stack[BOT-1], and jsR_callcfunction overwrites that slot only after F returned; trace[0] is never examined |
| 485 | jsarray.c:77 | Ap_join_cycle | callee object's `type != JS_CCFUNCTION` (a script function on the trace) | `return 0` — cycle detection abandoned | [x] `t_array_join_cycle_bailouts` |
| 486 | jsarray.c:81 | Ap_join_cycle | matched `Ap_join` frame but its `this` slot `&J->stack[stk]` is not `JS_TOBJECT` | `return 0` — cycle detection abandoned | [-] jsarray.c:81 is unreachable -- fun == Ap_join means that frame's own Ap_join_cycle already ran js_toobject(J,0) on exactly that slot, and jsV_toobject rewrites it in place to JS_TOBJECT (jsvalue.c:409-411) or throws |
| 487 | jsarray.c:91 | Ap_join_cycle | trace frame callee is a C function other than `Ap_join` or `Ap_toString` | `return 0` — cycle detection abandoned | [x] `t_array_join_cycle_bailouts` |
| 488 | jsarray.c:95 | Ap_join_cycle | walked `J->tracetop - 1` down to `top == 0` without finding a self-referencing `Ap_join` frame | `return 0` (no cycle) | [x] `t_array_join_cycle_bailouts` |
| 489 | jsarray.c:106 | Ap_join | self-referential array, e.g. `a=[]; a[0]=a; a.join()` — `Ap_join_cycle` returned 1 | no throw; pushes `""` and returns (infinite-recursion guard) | [x] `t_array_join_paths` |
| 490 | jsarray.c:121 | Ap_join | `len <= 0` (empty array, or `length` coerced to 0/negative) | pushes `""` and returns before any allocation | [x] `t_array_join_paths` |
| 491 | jsarray.c:126 | Ap_join | any throw inside the join loop (OOM in `js_malloc`/`js_realloc` at 142/150, or a throwing element `toString`) | `js_try` handler frees `out` then `js_throw(J)` re-raises the pending exception | [x] `t_array_join_paths` |
| 492 | jsarray.c:134 | Ap_join | element k is `undefined` or `null` (`!js_iscoercible(J,-1)`) | no error; `rlen = 0`, element contributes empty text (r keeps its prior value but is unused) | [x] `t_array_join_paths` |
| 493 | jsarray.c:148 | Ap_join | accumulated `n + seplen + rlen > JS_STRLIMIT` (1<<28) — e.g. joining with a huge separator | RangeError `"invalid string length"` (leaks nothing: js_try at 126 frees `out`) | [x] `t_builtin_string_limit_rows` |
| 494 | jsarray.c:175 | Ap_pop | `Array.prototype.pop` on array with `js_getlength() <= 0` | no throw; `js_setlength(J,0,0)` then pushes `undefined` (jsarray.c:180-181) | [x] `t_array_pop_shift_empty` |
| 495 | jsarray.c:236 | Ap_shift | `js_getlength(J,0) == 0` | no throw; `js_setlength(J,0,0)` then pushes `undefined`, early return | [x] `t_array_pop_shift_empty` |
| 496 | jsarray.c:266 | Ap_slice | negative `start` argument: `sv < 0` -> `sv += len` | index rebased; no error | [x] `t_array_slice_clamping` |
| 497 | jsarray.c:267 | Ap_slice | negative `end` argument: `ev < 0` -> `ev += len` | index rebased; no error | [x] `t_array_slice_clamping` |
| 498 | jsarray.c:269 | Ap_slice | `start` still negative after rebase, or `start > len` | clamped to `0` / `len` (`s = sv < 0 ? 0 : sv > len ? len : sv`) | [x] `t_array_slice_clamping` |
| 499 | jsarray.c:270 | Ap_slice | `end` still negative after rebase, or `end > len` | clamped to `0` / `len` | [x] `t_array_slice_clamping` |
| 500 | jsarray.c:279 | Ap_sort_cmp | `js_tovalue(J,0)->u.object` read unconditionally — `this` is not an object (e.g. `Array.prototype.sort.call("abc")` reaching here) | undefined behaviour: `u.object` read off a non-object `js_Value` with no type check | [-] UNDEFINED BEHAVIOUR -- jsarray.c:279 reads js_tovalue(J,0)->u.object with no type check and then u.a.simple / u.a.flat_length with no class check, so a non-object or non-array `this` reads a never-stored union member (for JS_CSTRING a heap ADDRESS, so the outcome varies run to run). Only plain JS_COBJECT receivers (memset to 0 by jsV_newobject) are driven, in t_array_sort_generic_path |
| 501 | jsarray.c:280 | Ap_sort_cmp | `!obj->u.a.simple || idx_b >= obj->u.a.flat_length` — index past the flat array or a non-simple (sparse) array | falls back to the generic `js_hasindex` path (bounds guard preventing OOB read of `u.a.array`) | [x] `t_array_sort_generic_path` |
| 502 | jsarray.c:285 | Ap_sort_cmp | flat path: `val_a` is `JS_TUNDEFINED` | `return und_b` — undefined sorts to the end without calling the comparator | [x] `t_array_sort_generic_path` |
| 503 | jsarray.c:286 | Ap_sort_cmp | flat path: `val_b` is `JS_TUNDEFINED` (and a is not) | `return -1` | [x] `t_array_sort_generic_path` |
| 504 | jsarray.c:296 | Ap_sort_cmp | comparator returned a value whose `js_tonumber` is NaN | `return 0` (treated as "equal"; NaN result rejected) | [x] `t_array_sort_generic_path` |
| 505 | jsarray.c:316 | Ap_sort_cmp | generic path: neither index present (`!has_a && !has_b`) — two holes | `return 0` | [x] `t_array_sort_generic_path` |
| 506 | jsarray.c:319 | Ap_sort_cmp | `has_a && !has_b` — b is a hole | `js_pop(J,1)`; `return -1` (holes sort last) | [x] `t_array_sort_generic_path` |
| 507 | jsarray.c:323 | Ap_sort_cmp | `!has_a && has_b` — a is a hole | `js_pop(J,1)`; `return 1` | [x] `t_array_sort_generic_path` |
| 508 | jsarray.c:330 | Ap_sort_cmp | generic path: element a is `undefined` | `js_pop(J,2)`; `return und_b` | [x] `t_array_sort_generic_path` |
| 509 | jsarray.c:334 | Ap_sort_cmp | generic path: element b is `undefined` | `js_pop(J,2)`; `return -1` | [x] `t_array_sort_generic_path` |
| 510 | jsarray.c:348 | Ap_sort_cmp | generic path: comparator result is NaN | `js_pop(J,3)` already done; `return 0` | [x] `t_array_sort_generic_path` |
| 511 | jsarray.c:366 | Ap_sort_swap | `!obj->u.a.simple || idx_b >= obj->u.a.flat_length` | takes generic has/set/del path — bounds guard against OOB write into `u.a.array` | [x] `t_array_sort_generic_path` |
| 512 | jsarray.c:434 | Ap_sort | `js_getlength(J,0) <= 1` | no sort performed; `js_copy(J,0)` returns `this` | [x] `t_array_sort_guards` |
| 513 | jsarray.c:439 | Ap_sort | comparator argument is neither callable nor `undefined`, e.g. `[3,1].sort(1)` | TypeError `"comparison function must be a function or undefined"` | [x] `t_array_sort_guards` |
| 514 | jsarray.c:442 | Ap_sort | `len >= INT_MAX` (length clamped to `INT_MAX` by `jsV_numbertointeger`, e.g. `{length: 1e30}`) | RangeError `"array is too large to sort"` | [x] `t_array_sort_guards` |
| 515 | jsarray.c:457 | Ap_splice | negative `start`: `start = (len + start) > 0 ? len + start : 0` | rebased and floored at 0 | [x] `t_array_splice_clamping` |
| 516 | jsarray.c:459 | Ap_splice | `start > len` | clamped to `len` | [x] `t_array_splice_clamping` |
| 517 | jsarray.c:466 | Ap_splice | `deleteCount > len - start` | clamped to `len - start` | [x] `t_array_splice_clamping` |
| 518 | jsarray.c:468 | Ap_splice | `deleteCount < 0` | clamped to `0` | [x] `t_array_splice_clamping` |
| 519 | jsarray.c:536 | Ap_toString | `Array.prototype.toString.call(undefined)` / `.call(null)` (`!js_iscoercible(J,0)`) | TypeError `"'this' is not an object"` | [x] `t_array_tostring` |
| 520 | jsarray.c:539 | Ap_toString | `this.join` is not callable (e.g. `({join:1})` with Array.prototype.toString) | no throw; pops and substitutes `Object.prototype.toString` (jsarray.c:542-546) | [x] `t_array_tostring` |
| 521 | jsarray.c:558 | Ap_indexOf | `fromIndex < 0` -> `from = len + from` | rebased | [x] `t_array_indexof_bounds` |
| 522 | jsarray.c:559 | Ap_indexOf | `from` still `< 0` after rebase | clamped to `0` | [x] `t_array_indexof_bounds` |
| 523 | jsarray.c:572 | Ap_indexOf | search value not strictly equal to any present element | pushes `-1` (not-found sentinel) | [x] `t_array_indexof_bounds` |
| 524 | jsarray.c:581 | Ap_lastIndexOf | `from > len - 1` | clamped to `len - 1` | [x] `t_array_indexof_bounds` |
| 525 | jsarray.c:582 | Ap_lastIndexOf | `from < 0` -> `from = len + from` (note: applied AFTER the `len-1` clamp, so a negative arg is rebased against `len`, and can stay negative) | rebased; loop `for (k=from; k>=0; --k)` then does not execute | [x] `t_array_indexof_bounds` |
| 526 | jsarray.c:595 | Ap_lastIndexOf | no strictly-equal element found | pushes `-1` | [x] `t_array_indexof_bounds` |
| 527 | jsarray.c:603 | Ap_every | `Array.prototype.every` callback argument not callable, e.g. `[].every()` | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 528 | jsarray.c:632 | Ap_some | `Array.prototype.some` callback not callable | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 529 | jsarray.c:661 | Ap_forEach | `Array.prototype.forEach` callback not callable | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 530 | jsarray.c:688 | Ap_map | `Array.prototype.map` callback not callable | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 531 | jsarray.c:717 | Ap_filter | `Array.prototype.filter` callback not callable | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 532 | jsarray.c:750 | Ap_reduce | `Array.prototype.reduce` callback not callable | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 533 | jsarray.c:756 | Ap_reduce | `len == 0 && js_gettop(J) < 3` — `[].reduce(fn)` with no initialValue | TypeError `"no initial value"` | [x] `t_array_callback_typeerrors` |
| 534 | jsarray.c:766 | Ap_reduce | no initialValue and array is all holes (`k == len` after the scan), e.g. `new Array(5).reduce(fn)` | TypeError `"no initial value"` | [x] `t_array_callback_typeerrors` |
| 535 | jsarray.c:791 | Ap_reduceRight | `Array.prototype.reduceRight` callback not callable | TypeError `"callback is not a function"` | [x] `t_array_callback_typeerrors` |
| 536 | jsarray.c:797 | Ap_reduceRight | `len == 0 && !hasinitial` — `[].reduceRight(fn)` | TypeError `"no initial value"` | [x] `t_array_callback_typeerrors` |
| 537 | jsarray.c:807 | Ap_reduceRight | no initialValue and array is all holes (`k < 0` after the scan) | TypeError `"no initial value"` | [x] `t_array_callback_typeerrors` |
| 538 | jsarray.c:829 | A_isArray | argument is not an object (`!js_isobject(J,1)`), e.g. `Array.isArray("x")` | no throw; pushes `false` (jsarray.c:833) | [x] `t_array_isarray` |
| 539 | jsarray.c:831 | A_isArray | argument is an object whose `type != JS_CARRAY` | pushes `false` | [x] `t_array_isarray` |
| 540 | jsobject.c:5 | jsB_new_Object | `new Object(undefined)` / `new Object(null)` | no throw; `js_newobject(J)` — plain empty object instead of `js_toobject` TypeError | [x] `t_object_constructor` |
| 541 | jsobject.c:8 | jsB_new_Object | `new Object(v)` where v is a primitive string/number/boolean | wraps via `js_toobject` (jsvalue.c:404-408); no error | [x] `t_object_constructor` |
| 542 | jsobject.c:13 | jsB_Object | `Object(undefined)` / `Object(null)` called as a function | no throw; `js_newobject(J)` | [x] `t_object_constructor` |
| 543 | jsobject.c:21 | Op_toString | `Object.prototype.toString.call(undefined)` | no throw; pushes `"[object Undefined]"` (guard before `js_toobject`) | [x] `t_object_tostring_all_classes` |
| 544 | jsobject.c:23 | Op_toString | `Object.prototype.toString.call(null)` | pushes `"[object Null]"` | [x] `t_object_tostring_all_classes` |
| 545 | jsobject.c:27 | Op_toString | `self->type` matches none of the 16 enumerated `case` labels — `switch` has NO `default` | nothing is pushed; the C function's return value is read from the stack top (`this`) instead of a string | [x] `t_object_tostring_all_classes` |
| 546 | jsobject.c:61 | Op_hasOwnProperty | `Object.prototype.hasOwnProperty.call(undefined, "x")` — `js_toobject(J,0)` on non-coercible `this` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_object_own_property_predicates` |
| 547 | jsobject.c:67 | Op_hasOwnProperty | `JS_CSTRING` receiver: name parses as an array index but `k < 0 || k >= self->u.s.length` | bounds check fails; falls through to `jsV_getownproperty` -> `false` | [x] `t_object_own_property_predicates` |
| 548 | jsobject.c:74 | Op_hasOwnProperty | `JS_CARRAY` + `u.a.simple` receiver: index `k < 0 || k >= self->u.a.flat_length` | bounds check fails; falls through to `jsV_getownproperty` | [x] `t_object_own_property_predicates` |
| 549 | jsobject.c:80 | Op_hasOwnProperty | `jsV_getownproperty` returns `NULL` (property absent) | pushes `false` (`ref != NULL`) | [x] `t_object_own_property_predicates` |
| 550 | jsobject.c:86 | Op_isPrototypeOf | `js_toobject(J,0)` with `this` undefined/null | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_object_own_property_predicates` |
| 551 | jsobject.c:87 | Op_isPrototypeOf | argument is not an object | pushes `false` (jsobject.c:97) without walking | [x] `t_object_own_property_predicates` |
| 552 | jsobject.c:95 | Op_isPrototypeOf | prototype chain walked to `V == NULL` without matching `self` | pushes `false` | [x] `t_object_own_property_predicates` |
| 553 | jsobject.c:102 | Op_propertyIsEnumerable | `js_toobject(J,0)` with `this` undefined/null | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_object_own_property_predicates` |
| 554 | jsobject.c:105 | Op_propertyIsEnumerable | `jsV_getownproperty` returns `NULL`, or the property has `JS_DONTENUM` | pushes `false` | [x] `t_object_own_property_predicates` |
| 555 | jsobject.c:111 | O_getPrototypeOf | `Object.getPrototypeOf(v)` where `!js_isobject(J,1)` (primitive, undefined, null) | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 556 | jsobject.c:114 | O_getPrototypeOf | `obj->prototype == NULL` (e.g. `Object.create(null)` result) | no throw; pushes `null` (jsobject.c:117) | [x] `t_object_static_typeerrors` |
| 557 | jsobject.c:124 | O_getOwnPropertyDescriptor | first argument not an object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 558 | jsobject.c:128 | O_getOwnPropertyDescriptor | `jsV_getproperty` returns `NULL` — no such property (also true for built-in string/array index/length, per the TODO at 129) | no throw; pushes `undefined` | [x] `t_object_static_typeerrors` |
| 559 | jsobject.c:175 | O_getOwnPropertyNames | first argument not an object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 560 | jsobject.c:181 | O_getOwnPropertyNames | `obj->properties->level == 0` (empty/sentinel property tree) | skips the walk; `i = 0` | [x] `t_object_static_typeerrors` |
| 561 | jsobject.c:234 | ToPropertyDescriptor | descriptor lacks `writable` | `haswritable` stays 0 and `writable` stays 0, so `atts |= JS_READONLY` (jsobject.c:252) — absent means read-only | [x] `t_object_defineproperty_descriptor` |
| 562 | jsobject.c:239 | ToPropertyDescriptor | descriptor lacks `enumerable` | `atts |= JS_DONTENUM` (jsobject.c:253) | [x] `t_object_defineproperty_descriptor` |
| 563 | jsobject.c:243 | ToPropertyDescriptor | descriptor lacks `configurable` | `atts |= JS_DONTCONF` (jsobject.c:254) | [x] `t_object_defineproperty_descriptor` |
| 564 | jsobject.c:256 | ToPropertyDescriptor | descriptor has BOTH `get` and (`writable` or `value`), e.g. `Object.defineProperty(o,"x",{get:f,value:1})` | TypeError `"value/writable and get/set attributes are exclusive"` | [x] `t_object_defineproperty_descriptor` |
| 565 | jsobject.c:259 | ToPropertyDescriptor | descriptor has no `get` | pushes `undefined` in the getter slot | [x] `t_object_defineproperty_descriptor` |
| 566 | jsobject.c:263 | ToPropertyDescriptor | descriptor has BOTH `set` and (`writable` or `value`) | TypeError `"value/writable and get/set attributes are exclusive"` | [x] `t_object_defineproperty_descriptor` |
| 567 | jsobject.c:266 | ToPropertyDescriptor | descriptor has no `set` | pushes `undefined` in the setter slot | [x] `t_object_defineproperty_descriptor` |
| 568 | jsobject.c:277 | O_defineProperty | `Object.defineProperty(v, name, desc)` where `!js_isobject(J,1)` — target not an object | TypeError `"not an object"` | [x] `t_object_defineproperty_descriptor` |
| 569 | jsobject.c:278 | O_defineProperty | descriptor argument (index 3) is not an object, e.g. `Object.defineProperty({}, "x", 1)` | TypeError `"not an object"` | [x] `t_object_defineproperty_descriptor` |
| 570 | jsobject.c:288 | O_defineProperties_walk | an enumerable own property of `props` has `value.t.type != JS_TOBJECT` — a non-object descriptor, e.g. `Object.defineProperties({}, {x: 1})` | TypeError `"not an object"` | [x] `t_object_defineproperties` |
| 571 | jsobject.c:304 | O_defineProperties_imp | `!js_isobject(J,2)` — the properties bag is not an object (reached from `Object.defineProperties` and from `Object.create(p, x)`) | TypeError `"not an object"` | [x] `t_object_defineproperties` |
| 572 | jsobject.c:307 | O_defineProperties_imp | `props->properties->level == 0` (no own properties) | silently does nothing | [x] `t_object_defineproperties` |
| 573 | jsobject.c:313 | O_defineProperties_imp | `js_hasproperty(J, 2, name)` false for a name collected by the walk | that descriptor is skipped | [x] `t_object_defineproperties` |
| 574 | jsobject.c:326 | O_defineProperties | first argument not an object | TypeError `"not an object"` | [x] `t_object_defineproperties` |
| 575 | jsobject.c:337 | O_create | `Object.create(v)` where v is neither an object nor `null`, e.g. `Object.create(1)` / `Object.create()` | TypeError `"not an object or null"` (jsobject.c:342) | [x] `t_object_static_typeerrors` |
| 576 | jsobject.c:339 | O_create | `Object.create(null)` | accepted: `proto = NULL`, object created with no prototype | [x] `t_object_defineproperties` |
| 577 | jsobject.c:347 | O_create | second argument `undefined` | `O_defineProperties_imp` skipped entirely | [x] `t_object_defineproperties` |
| 578 | jsobject.c:371 | O_keys | `Object.keys(v)` where `!js_isobject(J,1)` | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 579 | jsobject.c:402 | O_preventExtensions | `Object.preventExtensions(v)` with non-object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 580 | jsobject.c:412 | O_isExtensible | `Object.isExtensible(v)` with non-object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 581 | jsobject.c:430 | O_seal | `Object.seal(v)` with non-object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 582 | jsobject.c:448 | O_isSealed_walk | any property lacking `JS_DONTCONF` | `return 0` -> `Object.isSealed` pushes `false` | [x] `t_object_seal_freeze_walks` |
| 583 | jsobject.c:460 | O_isSealed | `Object.isSealed(v)` with non-object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 584 | jsobject.c:464 | O_isSealed | `obj->extensible` still set | pushes `false`, early return (no walk) | [x] `t_object_seal_freeze_walks` |
| 585 | jsobject.c:488 | O_freeze | `Object.freeze(v)` with non-object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 586 | jsobject.c:506 | O_isFrozen_walk | any property lacking `JS_READONLY` | `return 0` | [x] `t_object_seal_freeze_walks` |
| 587 | jsobject.c:508 | O_isFrozen_walk | any property lacking `JS_DONTCONF` | `return 0` | [x] `t_object_seal_freeze_walks` |
| 588 | jsobject.c:520 | O_isFrozen | `Object.isFrozen(v)` with non-object | TypeError `"not an object"` | [x] `t_object_static_typeerrors` |
| 589 | jsobject.c:526 | O_isFrozen | `O_isFrozen_walk` returned 0 | pushes `false`, early return | [x] `t_object_seal_freeze_walks` |
| 590 | jsstring.c:8 | js_doregexec | `js_regexec` returned a negative value (regexp engine failure, e.g. `REG_ERROR`/backtrack-stack overflow); reached from `Sp_match` (500), `Sp_search` (537), `Sp_replace_regexp` (554, 633), `Sp_split_regexp` (739, 748) | plain Error `"regexec failed"` | [x] `t_string_regexec_failed` |
| 591 | jsstring.c:15 | checkstring | `!js_iscoercible(J, idx)` — `this` is `undefined` or `null` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 592 | jsstring.c:27 | js_runeat | walked past the end of the string (`rune == 0` while `i >= 0`) — index >= string length, or a negative index (loop body never runs and `rune` stays `EOF`) | `return EOF` (-1) | [x] `t_string_charat_bounds` |
| 593 | jsstring.c:107 | Sp_toString | `String.prototype.toString.call(undefined/null)` — `js_toobject(J,0)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_string_tostring_valueof` |
| 594 | jsstring.c:108 | Sp_toString | `self->type != JS_CSTRING`, e.g. `String.prototype.toString.call(1)` (Number wrapper) | TypeError `"not a string"` | [x] `t_string_tostring_valueof` |
| 595 | jsstring.c:114 | Sp_valueOf | `String.prototype.valueOf.call(undefined/null)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_string_tostring_valueof` |
| 596 | jsstring.c:115 | Sp_valueOf | `self->type != JS_CSTRING` | TypeError `"not a string"` | [x] `t_string_tostring_valueof` |
| 597 | jsstring.c:122 | Sp_charAt | `String.prototype.charAt.call(undefined/null, i)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 598 | jsstring.c:125 | Sp_charAt | `js_runeat` returned `EOF` (< 0) — index negative or >= length, e.g. `"abc".charAt(9)` | no throw; pushes `""` (jsstring.c:129) | [x] `t_string_charat_bounds` |
| 599 | jsstring.c:135 | Sp_charCodeAt | `String.prototype.charCodeAt.call(undefined/null, i)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 600 | jsstring.c:138 | Sp_charCodeAt | `js_runeat` returned `EOF` (< 0) — out-of-range index, e.g. `"abc".charCodeAt(9)` | no throw; pushes `NAN` (jsstring.c:141) | [x] `t_string_charat_bounds` |
| 601 | jsstring.c:151 | Sp_concat | `js_gettop(J) == 1` — no arguments | returns without pushing anything; the C-function result is read off the stack top, i.e. `this` | [x] `t_string_concat_paths` |
| 602 | jsstring.c:154 | Sp_concat | `String.prototype.concat.call(undefined/null, ...)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 603 | jsstring.c:157 | Sp_concat | any throw after the `js_try` (RangeError at 163/171, OOM in `js_malloc`/`js_realloc`, throwing `toString`) | handler frees `out` then `js_throw(J)` re-raises | [x] `t_string_concat_paths` |
| 604 | jsstring.c:162 | Sp_concat | `n = 1 + strlen(this) > JS_STRLIMIT` (1<<28) — receiver already at the string limit | RangeError `"invalid string length"` | [x] `t_builtin_string_limit_rows` |
| 605 | jsstring.c:170 | Sp_concat | accumulated `n > JS_STRLIMIT` (1<<28) after appending argument i | RangeError `"invalid string length"` | [x] `t_builtin_string_limit_rows` |
| 606 | jsstring.c:183 | Sp_indexOf | `String.prototype.indexOf.call(undefined/null, x)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 607 | jsstring.c:190 | Sp_indexOf | rune index `k < pos` — a match occurring before `fromIndex` | match rejected, scan continues | [x] `t_string_indexof_paths` |
| 608 | jsstring.c:197 | Sp_indexOf | needle never matched at any `k >= pos` (loop exits on `*haystack == 0`) | pushes `-1` | [x] `t_string_indexof_paths` |
| 609 | jsstring.c:202 | Sp_lastIndexOf | `String.prototype.lastIndexOf.call(undefined/null, x)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 610 | jsstring.c:208 | Sp_lastIndexOf | rune index `k > pos` | loop terminates; positions after `fromIndex` are rejected | [x] `t_string_indexof_paths` |
| 611 | jsstring.c:214 | Sp_lastIndexOf | needle never matched (`last` stays at its `-1` initialiser, line 206) | pushes `-1` | [x] `t_string_indexof_paths` |
| 612 | jsstring.c:219 | Sp_localeCompare | `String.prototype.localeCompare.call(undefined/null, x)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 613 | jsstring.c:248 | Sp_substring_imp | `i == a && k == n` — the requested range does not split a surrogate pair | fast path: `js_pushlstring(J, head, tail-head)`, returns before allocating | [x] `t_string_substring_surrogates` |
| 614 | jsstring.c:253 | Sp_substring_imp | any throw after the `js_try` — **including OOM inside `js_malloc` at line 258**, at which point `p` (declared uninitialised at line 228) has never been assigned | handler runs `js_free(J, p)` on an **indeterminate pointer**, then `js_throw(J)` | [-] UNDEFINED BEHAVIOUR -- jsstring.c:253-258's js_try handler runs js_free(J, p) on `p`, declared uninitialised at jsstring.c:228 and only assigned by the js_malloc at :258, so reaching the handler frees an indeterminate stack value. Every non-OOM path through Sp_substring_imp (rows 613/615/616) is driven in t_string_substring_surrogates |
| 615 | jsstring.c:261 | Sp_substring_imp | `i > a` — substring starts in the middle of a surrogate pair (start index lands on a low surrogate) | emits a synthesized low surrogate `0xdc00 + ((head_rune-0x10000) & 0x3ff)` prefix | [x] `t_string_substring_surrogates` |
| 616 | jsstring.c:269 | Sp_substring_imp | `k > n` — substring ends in the middle of a surrogate pair | emits a synthesized high surrogate `0xd800 + ((tail_rune-0x10000) >> 10)` suffix | [x] `t_string_substring_surrogates` |
| 617 | jsstring.c:283 | Sp_slice | `String.prototype.slice.call(undefined/null, ...)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 618 | jsstring.c:288 | Sp_slice | `start < 0` -> `s = s + len` | rebased from the end | [x] `t_string_slice_substring_clamping` |
| 619 | jsstring.c:289 | Sp_slice | `end < 0` -> `e = e + len` | rebased from the end | [x] `t_string_slice_substring_clamping` |
| 620 | jsstring.c:291 | Sp_slice | `s` still `< 0` after rebase, or `s > len` | clamped to `0` / `len` | [x] `t_string_slice_substring_clamping` |
| 621 | jsstring.c:292 | Sp_slice | `e` still `< 0` after rebase, or `e > len` | clamped to `0` / `len` | [x] `t_string_slice_substring_clamping` |
| 622 | jsstring.c:298 | Sp_slice | `s == e` after clamping (empty range) | pushes `""` | [x] `t_string_slice_substring_clamping` |
| 623 | jsstring.c:304 | Sp_substring | `String.prototype.substring.call(undefined/null, ...)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 624 | jsstring.c:309 | Sp_substring | `start < 0` (no rebasing, unlike slice) or `start > len` | clamped to `0` / `len` | [x] `t_string_slice_substring_clamping` |
| 625 | jsstring.c:310 | Sp_substring | `end < 0` or `end > len` | clamped to `0` / `len` | [x] `t_string_slice_substring_clamping` |
| 626 | jsstring.c:316 | Sp_substring | `s == e` after clamping | pushes `""` | [x] `t_string_slice_substring_clamping` |
| 627 | jsstring.c:322 | Sp_toLowerCase | `String.prototype.toLowerCase.call(undefined/null)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 628 | jsstring.c:344 | Sp_toLowerCase | any throw after `js_try` (OOM in `js_malloc(J, n)` at 349, or in `js_pushstring` at 365) | handler frees `dst` then `js_throw(J)` | [x] `t_builtin_oom_try_handlers` |
| 629 | jsstring.c:333 | Sp_toLowerCase | `tolowerrune_full(rune)` returns `NULL` (no multi-char mapping) | falls back to single-rune `tolowerrune` | [x] `t_string_case_mapping` |
| 630 | jsstring.c:372 | Sp_toUpperCase | `String.prototype.toUpperCase.call(undefined/null)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 631 | jsstring.c:383 | Sp_toUpperCase | `toupperrune_full(rune)` returns `NULL` | falls back to single-rune `toupperrune` | [x] `t_string_case_mapping` |
| 632 | jsstring.c:394 | Sp_toUpperCase | any throw after `js_try` (OOM at 399, or in `js_pushstring` at 415) | handler frees `dst` then `js_throw(J)` | [x] `t_builtin_oom_try_handlers` |
| 633 | jsstring.c:434 | Sp_trim | `String.prototype.trim.call(undefined/null)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 634 | jsstring.c:435 | Sp_trim | leading byte matches `istrim` (0x9, 0xB, 0xC, 0x20, 0xA0, 0xFEFF, 0xA, 0xD, 0x2028, 0x2029) — note `istrim` is applied to a raw `char`, so U+00A0/U+FEFF/U+2028/U+2029 never match a single UTF-8 byte | leading bytes skipped; multi-byte whitespace not trimmed | [x] `t_string_trim` |
| 635 | jsstring.c:438 | Sp_trim | `e > s && istrim(e[-1])` | trailing bytes trimmed; loop stops at `e == s` (all-whitespace string -> `""`) | [x] `t_string_trim` |
| 636 | jsstring.c:450 | S_fromCharCode | any throw after `js_try` — OOM in `js_malloc(J, (top-1)*UTFmax + 1)` at 455, or in `js_pushstring` at 463 | handler frees `s` then `js_throw(J)` | [x] `t_builtin_oom_try_handlers` |
| 637 | jsstring.c:455 | S_fromCharCode | `(top-1) * UTFmax + 1` — NO limit check against `JS_STRLIMIT` and the product can overflow `int` for a huge argument count | signed-overflow / undersized allocation; no RangeError is raised | [-] UNDEFINED BEHAVIOUR -- jsstring.c:455's (top-1)*UTFmax+1 only overflows int for more than (INT_MAX-1)/4 ~ 536M arguments, which JS_STACKSIZE == 4096 forbids anyway; signed overflow is UB. The reachable half of the row (no RangeError however many arguments are passed) is driven in t_string_fromcharcode_range |
| 638 | jsstring.c:458 | S_fromCharCode | char code out of `Rune` range or negative, e.g. `String.fromCharCode(-1)` / `(0x110000)` — `js_touint32` wraps modulo 2^32 | no error; the wrapped value is passed to `runetochar` | [x] `t_string_fromcharcode_range` |
| 639 | jsstring.c:477 | Sp_match | `String.prototype.match.call(undefined/null, re)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 640 | jsstring.c:481 | Sp_match | argument is `undefined` | substituted with the empty regexp `js_newregexp(J, "", 0)` | [x] `t_string_match_paths` |
| 641 | jsstring.c:484 | Sp_match | argument is a non-regexp whose string form is not a valid pattern, e.g. `"x".match("[")` | SyntaxError from `js_newregexp`/`js_regcomp` (propagates out of `Sp_match`) | [x] `t_string_match_paths` |
| 642 | jsstring.c:487 | Sp_match | regexp lacks the `g` flag (`!(re->flags & JS_REGEXP_G)`) | delegates to `js_RegExp_prototype_exec` and returns | [x] `t_string_match_paths` |
| 643 | jsstring.c:499 | Sp_match | `a > e` — scan pointer walked past end of text | loop exits | [x] `t_string_match_paths` |
| 644 | jsstring.c:514 | Sp_match | global match produced zero matches (`len == 0`) | pops the array and pushes `null` | [x] `t_string_match_paths` |
| 645 | jsstring.c:526 | Sp_search | `String.prototype.search.call(undefined/null, re)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 646 | jsstring.c:531 | Sp_search | argument is `undefined` | substituted with the empty regexp | [x] `t_string_search_paths` |
| 647 | jsstring.c:533 | Sp_search | argument is a non-regexp with an invalid pattern string | SyntaxError from `js_newregexp` | [x] `t_string_search_paths` |
| 648 | jsstring.c:539 | Sp_search | `js_doregexec` returned nonzero (no match) | pushes `-1` | [x] `t_string_search_paths` |
| 649 | jsstring.c:551 | Sp_replace_regexp | `String.prototype.replace.call(undefined/null, re, r)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 650 | jsstring.c:552 | Sp_replace_regexp | `js_toregexp(J, 1)` — argument 1 not a `JS_CREGEXP` object (guarded by `js_isregexp` in `Sp_replace`, so only reachable via a direct internal call) | TypeError `"not a regexp"` (jsrun.c:373) | [-] dead call site -- Sp_replace_regexp is static and only entered from Sp_replace (jsstring.c:710) behind js_isregexp(J,1), which accepts exactly what js_toregexp accepts. The TypeError "not a regexp" it would raise is pinned on the exported js_toregexp in t_ffi_toregexp_not_a_regexp, which also shows every non-regexp argument routes to Sp_replace_string instead |
| 651 | jsstring.c:554 | Sp_replace_regexp | first `js_doregexec` found no match | no throw; `js_copy(J,0)` returns the original string unchanged | [x] `t_string_replace_regexp` |
| 652 | jsstring.c:561 | Sp_replace_regexp | any throw after `js_try` (throwing replacer function, OOM in `js_putc`/`js_puts`/`js_putm`) | handler frees `sb` then `js_throw(J)` | [x] `t_string_replace_regexp` |
| 653 | jsstring.c:588 | Sp_replace_regexp | replacement string ends with a lone `$` (`*(++r) == 0`) | `--r` backs up and falls through to the `'$'` case, emitting a literal `$` | [x] `t_string_replace_regexp` |
| 654 | jsstring.c:601 | Sp_replace_regexp | `$N` group reference with `x == 0` or `x >= m.nsub` — out-of-range capture group | not substituted; emits `$` followed by the digits literally | [x] `t_string_replace_regexp` |
| 655 | jsstring.c:605 | Sp_replace_regexp | two-digit out-of-range reference with `x == 10` exactly: the test is `x > 10`, not `x >= 10` | takes the single-digit branch `js_putc(J, &sb, '0' + 10)`, emitting `$:` instead of `$10` | [x] `t_string_replace_regexp` |
| 656 | jsstring.c:613 | Sp_replace_regexp | `$` followed by any other character | emits `$` and that character literally | [x] `t_string_replace_regexp` |
| 657 | jsstring.c:627 | Sp_replace_regexp | global replace where the match was empty (`n == 0`) and `*source == 0` — end of input | `goto end` to avoid an infinite loop | [x] `t_string_replace_regexp` |
| 658 | jsstring.c:652 | Sp_replace_string | `String.prototype.replace.call(undefined/null, s, r)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 659 | jsstring.c:655 | Sp_replace_string | `strstr(source, needle)` returned `NULL` — substring not found | no throw; `js_copy(J,0)` returns the original string | [x] `t_string_replace_string` |
| 660 | jsstring.c:662 | Sp_replace_string | any throw after `js_try` (throwing replacer function, buffer OOM) | handler frees `sb` then `js_throw(J)` | [x] `t_string_replace_string` |
| 661 | jsstring.c:686 | Sp_replace_string | replacement string ends with a lone `$` | `--r` backs up; emits a literal `$` | [x] `t_string_replace_string` |
| 662 | jsstring.c:692 | Sp_replace_string | `$` followed by a character other than `$ & \` '` (note: `$N` group refs are NOT supported in the string path) | emits `$` and the character literally | [x] `t_string_replace_string` |
| 663 | jsstring.c:725 | Sp_split_regexp | `String.prototype.split.call(undefined/null, re, n)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 664 | jsstring.c:726 | Sp_split_regexp | `js_toregexp(J, 1)` — argument 1 not a regexp (guarded by `Sp_split`) | TypeError `"not a regexp"` (jsrun.c:373) | [-] dead call site -- Sp_split_regexp is static and only entered from Sp_split (jsstring.c:824) behind js_isregexp(J,1). Same TypeError "not a regexp" pinned in t_ffi_toregexp_not_a_regexp |
| 665 | jsstring.c:727 | Sp_split_regexp | `limit` argument undefined | defaults to the magic cap `1 << 30` (not 2^32-1 as ES5 requires) | [x] `t_string_split_regexp` |
| 666 | jsstring.c:732 | Sp_split_regexp | `limit == 0` (including a negative or NaN limit coerced to 0) | returns the empty array immediately | [x] `t_string_split_regexp` |
| 667 | jsstring.c:738 | Sp_split_regexp | input string is empty (`e == text`) and the regexp does NOT match it | pushes `""` as element 0; if it does match, returns the empty array | [x] `t_string_split_regexp` |
| 668 | jsstring.c:755 | Sp_split_regexp | empty match at the end of the previous match (`b == c && b == p`) | that match is rejected; advances one rune and continues (infinite-loop guard) | [x] `t_string_split_regexp` |
| 669 | jsstring.c:760 | Sp_split_regexp | `len == limit` before pushing the pre-match piece | returns early, truncating the result | [x] `t_string_split_regexp` |
| 670 | jsstring.c:765 | Sp_split_regexp | `len == limit` before pushing capture group k | returns early | [x] `t_string_split_regexp` |
| 671 | jsstring.c:773 | Sp_split_regexp | `len == limit` before pushing the trailing remainder | returns early | [x] `t_string_split_regexp` |
| 672 | jsstring.c:780 | Sp_split_string | `String.prototype.split.call(undefined/null, s, n)` | TypeError `"string function called on null or undefined"` | [x] `t_string_checkstring_all_sites` |
| 673 | jsstring.c:782 | Sp_split_string | `limit` argument undefined | defaults to `1 << 30` | [x] `t_string_split_string` |
| 674 | jsstring.c:787 | Sp_split_string | `limit == 0` | returns the empty array immediately | [x] `t_string_split_string` |
| 675 | jsstring.c:793 | Sp_split_string | separator is the empty string (`n == 0`) | splits into individual runes, capped at `i < limit` | [x] `t_string_split_string` |
| 676 | jsstring.c:804 | Sp_split_string | `i >= limit`, or `str == NULL` after the final unmatched piece (line 813) | loop terminates; result truncated at `limit` elements | [x] `t_string_split_string` |
| 677 | jsstring.c:820 | Sp_split | separator argument is `undefined` | returns a 1-element array containing the whole string; never consults `limit` | [x] `t_string_split_string` |
| 678 | jsstring.c:97 | jsB_new_String | `new String()` with `js_gettop(J) <= 1` | uses `""` instead of reading a missing argument | [x] `t_string_constructor` |
| 679 | jsstring.c:102 | jsB_String | `String()` with no arguments | pushes `""` | [x] `t_string_constructor` |
| 680 | jsnumber.c:11 | jsB_new_Number | `new Number()` with `js_gettop(J) <= 1` | uses `0` | [x] `t_number_constructor` |
| 681 | jsnumber.c:16 | jsB_Number | `Number()` with no arguments | pushes `0` | [x] `t_number_constructor` |
| 682 | jsnumber.c:21 | Np_valueOf | `Number.prototype.valueOf.call(undefined/null)` — `js_toobject(J,0)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_number_valueof_tostring` |
| 683 | jsnumber.c:22 | Np_valueOf | `self->type != JS_CNUMBER`, e.g. `Number.prototype.valueOf.call("x")` | TypeError `"not a number"` | [x] `t_number_valueof_tostring` |
| 684 | jsnumber.c:29 | Np_toString | `Number.prototype.toString.call(undefined/null)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_number_valueof_tostring` |
| 685 | jsnumber.c:30 | Np_toString | `radix` argument is `undefined` | defaults to 10 (checked BEFORE the type check, so `js_tointeger(J,1)` on a throwing valueOf runs first) | [x] `t_number_valueof_tostring` |
| 686 | jsnumber.c:32 | Np_toString | `self->type != JS_CNUMBER` | TypeError `"not a number"` | [x] `t_number_valueof_tostring` |
| 687 | jsnumber.c:35 | Np_toString | `radix == 10` | fast path via `jsV_numbertostring`, returns before the radix range check | [x] `t_number_valueof_tostring` |
| 688 | jsnumber.c:39 | Np_toString | `radix < 2 || radix > 36`, e.g. `(5).toString(1)` / `(5).toString(37)` / `(5).toString(0)` (0 is only reachable when arg is defined) | RangeError `"invalid radix"` | [x] `t_number_valueof_tostring` |
| 689 | jsnumber.c:48 | Np_toString | `limit = ((uint64_t)1<<52)` — the mantissa cap used at 61/63 to decide how many radix digits fit in a `uint64_t` | digits beyond 2^52 of precision are dropped (`u = number*pow(radix,exp) + 0.5`) | [x] `t_number_radix_digits` |
| 690 | jsnumber.c:52 | Np_toString | `number == 0` with a non-10 radix | pushes `"0"` and returns (the `while (number*pow(...) < limit)` loops at 61/63 would not terminate otherwise) | [x] `t_number_valueof_tostring` |
| 691 | jsnumber.c:53 | Np_toString | `isnan(number)` with a non-10 radix | pushes `"NaN"` and returns | [x] `t_number_valueof_tostring` |
| 692 | jsnumber.c:54 | Np_toString | `isinf(number)` with a non-10 radix | pushes `sign ? "-Infinity" : "Infinity"` and returns | [x] `t_number_valueof_tostring` |
| 693 | jsnumber.c:68 | Np_toString | `u % radix == 0` — trailing zero digits | trimmed, decrementing `exp` | [x] `t_number_radix_digits` |
| 694 | jsnumber.c:76 | Np_toString | digits written into `char buf[100]` (line 28) with no bound check on `ndigits` | bounded only implicitly by the 2^52 `limit` and radix >= 2 (max ~53 digits); no explicit guard | [x] `t_number_radix_digits` |
| 695 | jsnumber.c:81 | Np_toString | any throw after `js_try` (OOM inside `js_putc` while building `sb`) | handler frees `sb` then `js_throw(J)` | [x] `t_builtin_oom_try_handlers` |
| 696 | jsnumber.c:120 | numtostr | `strchr(buf, 'e') == NULL` — the formatted number has no exponent | the `e%+d` exponent-normalisation rewrite is skipped | [x] `t_number_precision_ranges` |
| 697 | jsnumber.c:130 | Np_toFixed | `Number.prototype.toFixed.call(undefined/null)` — `js_toobject(J,0)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_number_precision_ranges` |
| 698 | jsnumber.c:134 | Np_toFixed | `self->type != JS_CNUMBER` | TypeError `"not a number"` | [x] `t_number_precision_ranges` |
| 699 | jsnumber.c:135 | Np_toFixed | `width < 0`, e.g. `(1).toFixed(-1)` | RangeError formatted with `"precision %d out of range"`, width | [x] `t_number_precision_ranges` |
| 700 | jsnumber.c:136 | Np_toFixed | `width > 20`, e.g. `(1).toFixed(21)` | RangeError formatted with `"precision %d out of range"`, width | [x] `t_number_precision_ranges` |
| 701 | jsnumber.c:138 | Np_toFixed | `isnan(x) || isinf(x) || x <= -1e21 || x >= 1e21` | no throw; falls back to `jsV_numbertostring(J, buf, x)` instead of `%.*f` | [x] `t_number_precision_ranges` |
| 702 | jsnumber.c:146 | Np_toExponential | `Number.prototype.toExponential.call(undefined/null)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_number_precision_ranges` |
| 703 | jsnumber.c:150 | Np_toExponential | `self->type != JS_CNUMBER` | TypeError `"not a number"` | [x] `t_number_precision_ranges` |
| 704 | jsnumber.c:151 | Np_toExponential | `width < 0`, e.g. `(1).toExponential(-1)` | RangeError `"precision %d out of range"`, width | [x] `t_number_precision_ranges` |
| 705 | jsnumber.c:152 | Np_toExponential | `width > 20`, e.g. `(1).toExponential(21)` | RangeError `"precision %d out of range"`, width | [x] `t_number_precision_ranges` |
| 706 | jsnumber.c:154 | Np_toExponential | `isnan(x) || isinf(x)` | no throw; falls back to `jsV_numbertostring` | [x] `t_number_precision_ranges` |
| 707 | jsnumber.c:162 | Np_toPrecision | `Number.prototype.toPrecision.call(undefined/null)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_number_precision_ranges` |
| 708 | jsnumber.c:166 | Np_toPrecision | `self->type != JS_CNUMBER` | TypeError `"not a number"` | [x] `t_number_precision_ranges` |
| 709 | jsnumber.c:167 | Np_toPrecision | `width < 1`, e.g. `(1).toPrecision(0)` or `(1).toPrecision()` (undefined -> `js_tointeger` -> 0) | RangeError `"precision %d out of range"`, width | [x] `t_number_precision_ranges` |
| 710 | jsnumber.c:168 | Np_toPrecision | `width > 21`, e.g. `(1).toPrecision(22)` | RangeError `"precision %d out of range"`, width | [x] `t_number_precision_ranges` |
| 711 | jsnumber.c:170 | Np_toPrecision | `isnan(x) || isinf(x)` | no throw; falls back to `jsV_numbertostring` | [x] `t_number_precision_ranges` |
| 712 | jsboolean.c:15 | Bp_toString | `Boolean.prototype.toString.call(undefined/null)` — `js_toobject(J,0)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_boolean_prototype` |
| 713 | jsboolean.c:16 | Bp_toString | `self->type != JS_CBOOLEAN`, e.g. `Boolean.prototype.toString.call(1)` | TypeError `"not a boolean"` | [x] `t_boolean_prototype` |
| 714 | jsboolean.c:22 | Bp_valueOf | `Boolean.prototype.valueOf.call(undefined/null)` | TypeError `"cannot convert undefined to object"` / `"cannot convert null to object"` | [x] `t_boolean_prototype` |
| 715 | jsboolean.c:23 | Bp_valueOf | `self->type != JS_CBOOLEAN` | TypeError `"not a boolean"` | [x] `t_boolean_prototype` |
| 716 | jsbuiltin.c:12 | jsB_propf | `strrchr(name, '.') == NULL` — a name with no dot | `pname = name`, the full name is used as the property key | [x] `t_builtin_propf_names` |
| 717 | jsbuiltin.c:33 | jsB_parseInt | `radix` argument undefined | defaults to sentinel `0`, triggering the auto-detect branch at 46 | [x] `t_builtin_parseint_parsefloat` |
| 718 | jsbuiltin.c:46 | jsB_parseInt | `radix == 0` and the string starts `0x`/`0X` | radix forced to 16 and the prefix skipped | [x] `t_builtin_parseint_parsefloat` |
| 719 | jsbuiltin.c:52 | jsB_parseInt | `radix < 2 || radix > 36` with radix explicitly given, e.g. `parseInt("10", 1)` / `parseInt("10", 37)` | no throw; pushes `NAN` and returns | [x] `t_builtin_parseint_parsefloat` |
| 720 | jsbuiltin.c:57 | jsB_parseInt | `js_strtol` consumed nothing (`s == e`), e.g. `parseInt("zzz")` / `parseInt("")` | pushes `NAN` | [x] `t_builtin_parseint_parsefloat` |
| 721 | jsbuiltin.c:78 | jsB_parseFloat | `js_stringtofloat` consumed nothing (`e == s`), e.g. `parseFloat("abc")` | pushes `NAN` | [x] `t_builtin_parseint_parsefloat` |
| 722 | jsbuiltin.c:105 | Encode | any throw after `js_try` (OOM inside `js_putc` while growing `sb`) | handler frees `sb` then `js_throw(J)` | [x] `t_builtin_oom_try_handlers` |
| 723 | jsbuiltin.c:112 | Encode | byte `c` not present in the `unescaped` set — for `encodeURI` that is `URIALPHA URIDIGIT URIMARK URIRESERVED "#"`, for `encodeURIComponent` just `URIALPHA URIDIGIT URIMARK` | percent-escaped as `%` + `HEX[(c>>4)&0xf]` + `HEX[c&0xf]`; note `Encode` walks raw BYTES and NEVER validates UTF-8, so no URIError is ever raised for lone surrogates / malformed sequences | [x] `t_builtin_uri_errors` |
| 724 | jsbuiltin.c:122 | Encode | `sb == NULL` (input string was empty, so `js_putc` was only called once for the terminator — or not at all) | pushes `""` via `sb ? sb->s : ""` | [x] `t_builtin_uri_errors` |
| 725 | jsbuiltin.c:134 | Decode | any throw after `js_try` (the URIErrors at 145/149, or OOM inside `js_putc`) | handler frees `sb` then `js_throw(J)` | [x] `t_builtin_uri_errors` |
| 726 | jsbuiltin.c:144 | Decode | `%` with fewer than two bytes remaining (`!str[0] \|\| !str[1]`), e.g. `decodeURI("%")` / `decodeURI("%A")` | URIError `"truncated escape sequence"` | [x] `t_builtin_uri_errors` |
| 727 | jsbuiltin.c:148 | Decode | `%XY` where X or Y is not a hex digit (`!jsY_ishex(a) \|\| !jsY_ishex(b)`), e.g. `decodeURI("%zz")` | URIError `"invalid escape sequence"` | [x] `t_builtin_uri_errors` |
| 728 | jsbuiltin.c:151 | Decode | decoded byte `c` IS in the `reserved` set — `URIRESERVED "#"` (`;/?:@&=+$,#`) for `decodeURI`, `""` for `decodeURIComponent` | escape left intact: re-emits `%` + original `a` + original `b`; note the decoded byte stream is NEVER validated as UTF-8, so no URIError for malformed output | [x] `t_builtin_uri_errors` |
| 729 | jsbuiltin.c:162 | Decode | `sb == NULL` (empty input) | pushes `""` via `sb ? sb->s : ""` | [x] `t_builtin_uri_errors` |
| 730 | jsbuiltin.c:205 | jsB_init | `js_regcompx(J->alloc, J->actx, "(?:)", 0, NULL)` — the error-out parameter is `NULL`, so a compile failure of the built-in `RegExp.prototype` pattern is not reported | `u.r.prog` would be left `NULL` with no diagnostic (unchecked return) | [x] `t_builtin_regexp_prototype_prog` |
| 731 | regexp.c:67 | die | any compile-time rejection (called from 32 sites below) | sets `g->error = message` then `longjmp(g->kaboom, 1)` back into `regcompx` | [x] `t_regexp_die_sites` |
| 732 | regexp.c:903 | regcompx | `setjmp(g.kaboom)` returns non-zero, i.e. any `die()` fired | frees `g.pstart`, `g.prog->cclass`, `g.prog->start`, `g.prog`; `if (errorp) *errorp = g.error`; returns NULL | [x] `t_regexp_die_sites` |
| 733 | regexp.c:101 | hex | `\xHH` / `\uHHHH` digit that is not `[0-9a-fA-F]` (e.g. `/\xZZ/`) | `regcomp` sets `*errorp = "invalid escape sequence"` and returns NULL | [x] `t_regexp_die_sites` |
| 734 | regexp.c:108 | dec | non-digit inside `{M,N}` count (e.g. `/a{x}/`) | `regcomp` sets `*errorp = "invalid quantifier"` and returns NULL | [x] `t_regexp_die_sites` |
| 735 | regexp.c:128 | nextrune | pattern ends immediately after `\` (e.g. `/a\\/` source `a\`) | `regcomp` sets `*errorp = "unterminated escape sequence"` and returns NULL | [x] `t_regexp_die_sites` |
| 736 | regexp.c:138 | nextrune | `\c` at end of pattern (no control letter follows) | `regcomp` sets `*errorp = "unterminated escape sequence"` and returns NULL | [x] `t_regexp_die_sites` |
| 737 | regexp.c:143 | nextrune | `\x` with fewer than 2 remaining bytes (e.g. `/\x4/`) | `regcomp` sets `*errorp = "unterminated escape sequence"` and returns NULL | [x] `t_regexp_die_sites` |
| 738 | regexp.c:153 | nextrune | `\u` with fewer than 4 remaining bytes (e.g. `/\u12/`) | `regcomp` sets `*errorp = "unterminated escape sequence"` and returns NULL | [x] `t_regexp_die_sites` |
| 739 | regexp.c:170 | nextrune | identity escape of a unicode letter or `_` not in `ESCAPES` (e.g. `/\y/`, `/\_/`) | `regcomp` sets `*errorp = "invalid escape character"` and returns NULL | [x] `t_regexp_die_sites` |
| 740 | regexp.c:186 | lexcount | `{M...}` where the accumulated min reaches `REPINF` (255), e.g. `/a{1000}/` | `regcomp` sets `*errorp = "numeric overflow"` and returns NULL | [x] `t_regexp_numeric_overflow_boundary` |
| 741 | regexp.c:200 | lexcount | `{M,N}` where the accumulated max reaches `REPINF` (255), e.g. `/a{1,1000}/` | `regcomp` sets `*errorp = "numeric overflow"` and returns NULL | [x] `t_regexp_numeric_overflow_boundary` |
| 742 | regexp.c:213 | newcclass | more than `REG_MAXCLASS` (128) character classes in one pattern (each `[...]`, `\d`, `\s`, `\w`, `\D`, `\S`, `\W` allocates one) | `regcomp` sets `*errorp = "too many character classes"` and returns NULL | [x] `t_regexp_maxclass_boundary` |
| 743 | regexp.c:224 | addrange | class range with `a > b` (e.g. `/[z-a]/`) | `regcomp` sets `*errorp = "invalid character class range"` and returns NULL | [x] `t_regexp_die_sites` |
| 744 | regexp.c:253 | addrange | more than `REG_MAXSPAN`/2 (32) non-overlapping spans in one class: `cc->end + 2 >= cc->spans + 64` | `regcomp` sets `*errorp = "too many character class ranges"` and returns NULL | [x] `t_regexp_maxspan_boundary` |
| 745 | regexp.c:322 | lexclass | EOF reached inside `[...]` (unterminated class, e.g. `/[abc/`) | `regcomp` sets `*errorp = "unterminated character class"` and returns NULL | [x] `t_regexp_die_sites` |
| 746 | regexp.c:493 | newrep | unbounded repeat (`*`, `+`, `{n,}`) of an atom that can match empty (e.g. `/(?:)*/`, `/(a*)*/`) — `max == REPINF && empty(atom)` | `regcomp` sets `*errorp = "infinite loop matching the empty string"` and returns NULL | [x] `t_regexp_die_sites` |
| 747 | regexp.c:541 | parseatom | back-reference to group 0, to `>= g->nsub`, or to a group not yet defined: `g->yychar == 0 \|\| g->yychar >= g->nsub \|\| !g->sub[g->yychar]` (e.g. `/\1/`, `/\2(a)/`) | `regcomp` sets `*errorp = "invalid back-reference"` and returns NULL | [x] `t_regexp_die_sites` |
| 748 | regexp.c:552 | parseatom | 16th capturing group: `g->nsub == REG_MAXSUB` (REG_MAXSUB = 16, index 0 reserved) | `regcomp` sets `*errorp = "too many captures"` and returns NULL | [x] `t_regexp_maxsub_boundary` |
| 749 | regexp.c:557 | parseatom | capturing `(` never closed by `)` (e.g. `/(a/`) | `regcomp` sets `*errorp = "unmatched '('"` and returns NULL | [x] `t_regexp_die_sites` |
| 750 | regexp.c:563 | parseatom | `(?:` group never closed by `)` (e.g. `/(?:a/`) | `regcomp` sets `*errorp = "unmatched '('"` and returns NULL | [x] `t_regexp_die_sites` |
| 751 | regexp.c:570 | parseatom | `(?=` lookahead never closed by `)` (e.g. `/(?=a/`) | `regcomp` sets `*errorp = "unmatched '('"` and returns NULL | [x] `t_regexp_die_sites` |
| 752 | regexp.c:577 | parseatom | `(?!` negative lookahead never closed by `)` (e.g. `/(?!a/`) | `regcomp` sets `*errorp = "unmatched '('"` and returns NULL | [x] `t_regexp_die_sites` |
| 753 | regexp.c:580 | parseatom | lookahead token cannot start an atom (`*`, `+`, `?`, `{`-count or EOF where an atom is required, e.g. `/*a/`, `/a\|*/`) | `regcomp` sets `*errorp = "syntax error"` and returns NULL | [x] `t_regexp_die_sites` |
| 754 | regexp.c:598 | parserep | `{M,N}` with `max < min` (e.g. `/a{3,1}/`) | `regcomp` sets `*errorp = "invalid quantifier"` and returns NULL | [x] `t_regexp_die_sites` |
| 755 | regexp.c:661 | count | parse-tree recursion depth `> REG_MAXREC` (4096), e.g. thousands of nested groups | `regcomp` sets `*errorp = "stack overflow"` and returns NULL | [x] `t_regexp_count_recursion_limit` |
| 756 | regexp.c:672 | count | instruction count for a `P_REP` node overflows: `n < 0 \|\| n > REG_MAXPROG` (32768), e.g. `/(?:a{100}){100}{100}/`-style nesting | `regcomp` sets `*errorp = "program too large"` and returns NULL | [x] `t_regexp_rep_program_too_large` |
| 757 | regexp.c:916 | regcompx | `alloc(ctx, NULL, sizeof (Reprog))` returns NULL (out of memory) | `regcomp` sets `*errorp = "cannot allocate regular expression"` and returns NULL | [x] `t_regexp_alloc_failures` |
| 758 | regexp.c:922 | regcompx | `strlen(pattern) * 2 > REG_MAXPROG` (pattern longer than 16384 bytes) | `regcomp` sets `*errorp = "program too large"` and returns NULL | [x] `t_regexp_pattern_length_limit` |
| 759 | regexp.c:926 | regcompx | `alloc` of `sizeof (Renode) * n` parse-node array returns NULL | `regcomp` sets `*errorp = "cannot allocate regular expression parse list"` and returns NULL | [x] `t_regexp_alloc_failures` |
| 760 | regexp.c:940 | regcompx | after `parsealt`, lookahead is `)` — unbalanced close paren (e.g. `/a)/`) | `regcomp` sets `*errorp = "unmatched ')'"` and returns NULL | [x] `t_regexp_die_sites` |
| 761 | regexp.c:942 | regcompx | after `parsealt`, lookahead is neither EOF nor `)` (unconsumed input) | `regcomp` sets `*errorp = "syntax error"` and returns NULL | [-] regexp.c:942 is unreachable. parsecat (regexp.c:610/614) only loops while the lookahead is not EOF/'|'/')', and parsealt (regexp.c:630) consumes every '|', so after parsealt the lookahead is always EOF or ')'; regexp.c:939 catches ')' (row 760) and EOF is the success path. The other "syntax error" site (regexp.c:580, row 753) is covered by t_regexp_die_sites. |
| 762 | regexp.c:951 | regcompx | total program size `6 + count(...)` is `< 0` or `> REG_MAXPROG` (32768) | `regcomp` sets `*errorp = "program too large"` and returns NULL | [x] `t_regexp_total_program_size_limit` |
| 763 | regexp.c:956 | regcompx | `alloc` of `n * sizeof (Reinst)` returns NULL | `regcomp` sets `*errorp = "cannot allocate regular expression instruction list"` and returns NULL | [x] `t_regexp_alloc_failures` |
| 764 | regexp.c:961 | regcompx | `alloc` of `g.ncclass * sizeof (Reclass)` returns NULL | `regcomp` sets `*errorp = "cannot allocate regular expression character class list"` and returns NULL | [x] `t_regexp_alloc_failures` |
| 765 | regexp.c:1076 | match | backtracking recursion depth `> REG_MAXREC` (4096), e.g. catastrophic backtracking on `/(a*)*b/` against a long "aaaa…" | returns -1 (distinct from no-match); callers raise Error "regexec failed" | [x] `t_regexp_match_recursion_limit` |
| 766 | regexp.c:1089 | match | `I_SPLIT`: recursive `match` on `pc->x` returned -1 | propagates `return -1` (stack overflow) | [x] `t_regexp_match_recursion_limit` |
| 767 | regexp.c:1100 | match | `I_PLA`: recursive `match` returned -1 | propagates `return -1` | [x] `t_regexp_match_recursion_limit` |
| 768 | regexp.c:1102 | match | `I_PLA`: positive lookahead body failed (`result == 1`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 769 | regexp.c:1109 | match | `I_NLA`: recursive `match` returned -1 | propagates `return -1` | [x] `t_regexp_match_recursion_limit` |
| 770 | regexp.c:1111 | match | `I_NLA`: negative lookahead body *did* match (`result == 0`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 771 | regexp.c:1116 | match | `I_ANYNL` at end of subject (`!*sp`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 772 | regexp.c:1121 | match | `I_ANY` at end of subject (`!*sp`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 773 | regexp.c:1124 | match | `I_ANY` where the next rune is `\n`, `\r`, U+2028 or U+2029 (`isnewline`) | returns 1 (no match) — `.` never matches line terminators | [x] `t_regexp_match_nomatch_sites` |
| 774 | regexp.c:1128 | match | `I_CHAR` at end of subject (`!*sp`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 775 | regexp.c:1133 | match | `I_CHAR` where decoded rune (canon'd if `REG_ICASE`) `!= pc->c` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 776 | regexp.c:1137 | match | `I_CCLASS` at end of subject (`!*sp`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 777 | regexp.c:1141 | match | `I_CCLASS` with `REG_ICASE` and `!incclasscanon(pc->cc, canon(c))` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 778 | regexp.c:1144 | match | `I_CCLASS` and `!incclass(pc->cc, c)` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 779 | regexp.c:1149 | match | `I_NCCLASS` at end of subject (`!*sp`) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 780 | regexp.c:1153 | match | `I_NCCLASS` with `REG_ICASE` and `incclasscanon(pc->cc, canon(c))` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 781 | regexp.c:1156 | match | `I_NCCLASS` and `incclass(pc->cc, c)` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 782 | regexp.c:1164 | match | `I_REF` with `REG_ICASE` and `strncmpcanon(sp, out->sub[n].sp, i) != 0` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 783 | regexp.c:1167 | match | `I_REF` and `strncmp(sp, out->sub[n].sp, i) != 0` | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 784 | regexp.c:1185 | match | `I_BOL` where `sp != bol` or `REG_NOTBOL` set, and (no `REG_NEWLINE` or `sp[-1]` is not a newline) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 785 | regexp.c:1197 | match | `I_EOL` where `*sp != 0` and (no `REG_NEWLINE` or `*sp` is not a newline) | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 786 | regexp.c:1202 | match | `I_WORD` (`\b`) where `iswordchar(sp[-1]) ^ iswordchar(sp[0])` is 0 | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 787 | regexp.c:1209 | match | `I_NWORD` (`\B`) where `iswordchar(sp[-1]) ^ iswordchar(sp[0])` is 1 | returns 1 (no match) | [x] `t_regexp_match_nomatch_sites` |
| 788 | regexp.c:1222 | match | `default:` — unknown/corrupt opcode in the compiled program | returns 1 (treated as no match, not an error) | [-] regexp.c:1221 default: needs an opcode outside the 17 I_* enumerators. Every instruction is written by emit() (regexp.c:680), every pc transition targets a slot a later emit() fills, and count() (regexp.c:657) never under-counts what compile() emits, so no uninitialised Reinst is reachable without memory corruption. The 21 reachable no-match sites are covered by t_regexp_match_nomatch_sites. |
| 789 | regexp.c:1038 | incclass | rune `c` not inside any `[p[0],p[1]]` span of the class | returns 0 (caller turns this into no-match) | [x] `t_regexp_match_nomatch_sites` |
| 790 | regexp.c:1048 | incclasscanon | no rune `r` in any span satisfies `c == canon(r)` (case-insensitive class miss) | returns 0 (caller turns this into no-match) | [x] `t_regexp_match_nomatch_sites` |
| 791 | regexp.c:1056 | strncmpcanon | subject `a` ends before `n` runes compared (back-reference longer than remaining input) | returns -1 (non-zero => `I_REF` no match) | [x] `t_regexp_match_nomatch_sites` |
| 792 | regexp.c:1057 | strncmpcanon | reference text `b` ends before `n` runes compared | returns 1 (non-zero => `I_REF` no match) | [x] `t_regexp_match_nomatch_sites` |
| 793 | regexp.c:1239 | regexec | any of the above `match` failures on the given `sp` | returns `match(...)`: 0 = match, 1 = `REG_NOMATCH`, -1 = recursion/stack overflow; `sub->sub[0..REG_MAXSUB-1]` pre-cleared to NULL | [x] `t_regexp_match_nomatch_sites` |
| 794 | jsregexp.c:38 | js_newregexpx | `js_regcompx` returned NULL for any reason in rows 3-34 | `js_syntaxerror(J, "regular expression: %s", error)` — e.g. `SyntaxError: regular expression: unmatched '('` | [x] `t_newregexp_syntaxerror` |
| 795 | jsregexp.c:63 | js_RegExp_prototype_exec | `/g` regexp whose `lastIndex` (`re->last`) is past end of subject: `re->last > strlen(haystack)` | resets `re->last = 0` and pushes `null` (no error) | [x] `t_regexp_lastindex_past_end` |
| 796 | jsregexp.c:77 | js_RegExp_prototype_exec | `js_regexec` returned < 0 (REG_MAXREC recursion blow-up) | `js_error(J, "regexec failed")` (plain `Error: regexec failed`) | [x] `t_regexp_regexec_failed` |
| 797 | jsregexp.c:96 | js_RegExp_prototype_exec | `js_regexec` returned 1 (no match) | if `/g`, sets `re->last = 0`; pushes `null` | [x] `t_regexp_nomatch_resets_lastindex` |
| 798 | jsregexp.c:107 | Rp_test | `RegExp.prototype.test` called with a `this` that is not a RegExp object | `js_toregexp` -> `js_typeerror(J, "not a regexp")` (jsrun.c:373) | [x] `t_regexp_not_a_regexp` |
| 799 | jsregexp.c:113 | Rp_test | `/g` regexp with `re->last > strlen(text)` | resets `re->last = 0`, pushes `false` (no error) | [x] `t_regexp_lastindex_past_end` |
| 800 | jsregexp.c:126 | Rp_test | `js_regexec` returned < 0 | `js_error(J, "regexec failed")` | [x] `t_regexp_regexec_failed` |
| 801 | jsregexp.c:137 | Rp_test | `js_regexec` returned 1 (no match) | if `/g`, `re->last = 0`; pushes `false` | [x] `t_regexp_nomatch_resets_lastindex` |
| 802 | jsregexp.c:149 | jsB_new_RegExp | `new RegExp(/re/, flags)` — arg 1 is a RegExp and arg 2 is defined | `js_typeerror(J, "cannot supply flags when creating one RegExp from another")` | [x] `t_regexp_ctor_flags` |
| 803 | jsregexp.c:172 | jsB_new_RegExp | flags string contains a character other than `g`, `i`, `m` (e.g. `new RegExp("a","x")`) | `js_syntaxerror(J, "invalid regular expression flag: '%c'", *s)` | [x] `t_regexp_ctor_flags` |
| 804 | jsregexp.c:175 | jsB_new_RegExp | flag `g` given more than once (`g > 1`) | `js_syntaxerror(J, "invalid regular expression flag: 'g'")` | [x] `t_regexp_ctor_flags` |
| 805 | jsregexp.c:176 | jsB_new_RegExp | flag `i` given more than once (`i > 1`) | `js_syntaxerror(J, "invalid regular expression flag: 'i'")` | [x] `t_regexp_ctor_flags` |
| 806 | jsregexp.c:177 | jsB_new_RegExp | flag `m` given more than once (`m > 1`) | `js_syntaxerror(J, "invalid regular expression flag: 'm'")` | [x] `t_regexp_ctor_flags` |
| 807 | jsregexp.c:198 | Rp_toString | `RegExp.prototype.toString` called on a non-RegExp `this` | `js_toregexp` -> `js_typeerror(J, "not a regexp")` | [x] `t_regexp_not_a_regexp` |
| 808 | jsregexp.c:221 | Rp_exec | `RegExp.prototype.exec` called on a non-RegExp `this` | `js_toregexp` -> `js_typeerror(J, "not a regexp")` | [x] `t_regexp_not_a_regexp` |
| 809 | json.c:41 | jsonexpect | lookahead token != expected token `t` | `js_syntaxerror(J, "JSON: unexpected token: %s (expected %s)", jsY_tokenstring(J->lookahead), jsY_tokenstring(t))` | [x] `t_json_parse_errors` |
| 810 | json.c:70 | jsonvalue | object member missing `:` after the key (e.g. `JSON.parse('{"a" 1}')`) | via `jsonexpect(J, ':')` -> `SyntaxError "JSON: unexpected token: <tok> (expected ':')"` | [x] `t_json_parse_errors` |
| 811 | json.c:75 | jsonvalue | object not terminated by `}` after last member (e.g. `JSON.parse('{"a":1')`) | via `jsonexpect(J, '}')` -> `SyntaxError "JSON: unexpected token: <tok> (expected '}')"` | [x] `t_json_parse_errors` |
| 812 | json.c:88 | jsonvalue | array not terminated by `]` after last element (e.g. `JSON.parse('[1,2')`) | via `jsonexpect(J, ']')` -> `SyntaxError "JSON: unexpected token: <tok> (expected ']')"` | [x] `t_json_parse_errors` |
| 813 | json.c:67 | jsonvalue | object member key is not a string token (e.g. `JSON.parse('{a:1}')`, `JSON.parse('{1:2}')`) | `js_syntaxerror(J, "JSON: unexpected token: %s (expected string)", jsY_tokenstring(J->lookahead))` | [x] `t_json_parse_errors` |
| 814 | json.c:107 | jsonvalue | token cannot start a JSON value — not string/number/`{`/`[`/true/false/null (e.g. `JSON.parse('')`, `JSON.parse('undefined')`, `JSON.parse(',')`) | `js_syntaxerror(J, "JSON: unexpected token: %s", jsY_tokenstring(J->lookahead))` | [x] `t_json_parse_errors` |
| 815 | json.c:246 | filterprop | `JSON.stringify(v, ["a"])` — key not present in the replacer array (or array entry is not string/number/String/Number object) | returns `found = 0`; `fmtobject` skips the property entirely | [x] `t_json_replacer_array` |
| 816 | json.c:261 | fmtobject | object already present in the holder chain on the stack (`js_toobject(J,i) == js_toobject(J,-1)` for `4 <= i < top-1`) — cyclic structure, e.g. `a={};a.a=a;JSON.stringify(a)` | `js_typeerror(J, "cyclic object value")` | [x] `t_json_cyclic` |
| 817 | json.c:297 | fmtarray | array already present in the holder chain (cyclic array, e.g. `a=[];a[0]=a;JSON.stringify(a)`) | `js_typeerror(J, "cyclic object value")` | [x] `t_json_cyclic` |
| 818 | json.c:359 | fmtvalue | value is `undefined`, or a callable (function) object, i.e. none of object/boolean/number/string/null matched | pops value and returns 0 ("no output") | [x] `t_json_skipped_values` |
| 819 | json.c:277 | fmtobject | `fmtvalue` returned 0 for a property (undefined/function valued property) | rewinds the buffer to `save` (`(*sb)->n = save`), property and its comma/indent are dropped | [x] `t_json_skipped_values` |
| 820 | json.c:305 | fmtarray | `fmtvalue` returned 0 for an array element (undefined/function/hole) | writes `"null"` in that slot | [x] `t_json_skipped_values` |
| 821 | json.c:403 | JSON_stringify | top-level value is undefined or a function (`fmtvalue` returned 0), e.g. `JSON.stringify(undefined)` | pushes `undefined` (no string result, no error) | [x] `t_json_skipped_values` |
| 822 | json.c:380 | JSON_stringify | numeric `space` argument < 0 | clamped: `n = 0` (no indent) | [x] `t_json_space` |
| 823 | json.c:381 | JSON_stringify | numeric `space` argument > 10 | clamped: `n = 10` | [x] `t_json_space` |
| 824 | json.c:388 | JSON_stringify | string `space` argument longer than 10 chars | truncated to first 10 bytes (`n = 10`), no error | [x] `t_json_space` |
| 825 | jsdate.c:214 | MakeDay | month index outside 0..11 after `pmod(m,12)` — happens when `m` is NaN/Inf (`im < 0 \|\| im >= 12`) | returns `NAN` (whole date becomes Invalid Date) | [x] `t_date_makeday_nan` |
| 826 | jsdate.c:230 | TimeClip | `!isfinite(t)` (NaN or +/-Inf time value) | returns `NAN` | [x] `t_date_timeclip` |
| 827 | jsdate.c:232 | TimeClip | `fabs(t) > 8.64e15` (outside +/-100,000,000 days from epoch) | returns `NAN` | [x] `t_date_timeclip` |
| 828 | jsdate.c:242 | toint | a byte in the fixed-width field is not `'0'..'9'` (includes premature end-of-string, since NUL fails the test) | returns 0 without advancing `*sp` | [x] `t_date_parse_failures` |
| 829 | jsdate.c:259 | parseDateTime | first 4 chars are not digits (e.g. `Date.parse("abcd")`, `Date.parse("")`) | returns `NAN` -> `Date.parse` yields NaN / `new Date(str)` is Invalid Date | [x] `t_date_parse_failures` |
| 830 | jsdate.c:262 | parseDateTime | `-` present but `MM` is not 2 digits (e.g. `"1970-1-01"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 831 | jsdate.c:265 | parseDateTime | second `-` present but `DD` is not 2 digits (e.g. `"1970-01-1"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 832 | jsdate.c:271 | parseDateTime | after `T`, `HH` is not 2 digits (e.g. `"1970-01-01Tx"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 833 | jsdate.c:272 | parseDateTime | after `THH` the next char is not `:` (e.g. `"1970-01-01T12"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 834 | jsdate.c:274 | parseDateTime | after `T HH :`, `mm` is not 2 digits (e.g. `"1970-01-01T12:0"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 835 | jsdate.c:277 | parseDateTime | after seconds `:`, `ss` is not 2 digits (e.g. `"1970-01-01T12:00:0"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 836 | jsdate.c:280 | parseDateTime | after `.`, `sss` is not exactly 3 digits (e.g. `"1970-01-01T12:00:00.5"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 837 | jsdate.c:290 | parseDateTime | timezone sign present but `HH` is not 2 digits (e.g. `"1970-01-01T00:00+1"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 838 | jsdate.c:293 | parseDateTime | timezone `:` present but `mm` is not 2 digits (e.g. `"1970-01-01T00:00+01:0"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 839 | jsdate.c:295 | parseDateTime | `tzh > 23 \|\| tzm > 59` (e.g. `"1970-01-01T00:00+24:00"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 840 | jsdate.c:302 | parseDateTime | trailing unconsumed characters (`*s` non-NUL), e.g. `"1970-01-01 junk"`, `"1970-01-01T00:00:00.000Zx"` | returns `NAN` | [x] `t_date_parse_failures` |
| 841 | jsdate.c:304 | parseDateTime | month out of range: `m < 1 \|\| m > 12` (e.g. `"1970-13-01"`, `"1970-00-01"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 842 | jsdate.c:305 | parseDateTime | day out of range: `d < 1 \|\| d > 31` (e.g. `"1970-01-32"`, `"1970-01-00"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 843 | jsdate.c:306 | parseDateTime | hour out of range: `H < 0 \|\| H > 24` (e.g. `"1970-01-01T25:00"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 844 | jsdate.c:307 | parseDateTime | minute out of range: `M < 0 \|\| M > 59` (e.g. `"1970-01-01T00:60"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 845 | jsdate.c:308 | parseDateTime | second out of range: `S < 0 \|\| S > 59` (e.g. `"1970-01-01T00:00:60"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 846 | jsdate.c:309 | parseDateTime | millisecond out of range: `ms < 0 \|\| ms > 999` | returns `NAN` | [x] `t_date_parse_failures` |
| 847 | jsdate.c:310 | parseDateTime | `H == 24` with non-zero `M`, `S` or `ms` (e.g. `"1970-01-01T24:01"`) | returns `NAN` | [x] `t_date_parse_failures` |
| 848 | jsdate.c:324 | fmtdate | `!isfinite(t)` (Invalid Date) | returns the literal string `"Invalid Date"` (buf untouched) | [x] `t_date_invalid_format` |
| 849 | jsdate.c:338 | fmttime | `!isfinite(t)` | returns the literal string `"Invalid Date"` | [x] `t_date_invalid_format` |
| 850 | jsdate.c:352 | fmtdatetime | `!isfinite(t)` | returns the literal string `"Invalid Date"` | [x] `t_date_invalid_format` |
| 851 | jsdate.c:366 | js_todate | `this` (or the indexed object) has `type != JS_CDATE`, e.g. `Date.prototype.getTime.call({})` | `js_typeerror(J, "not a date")` | [x] `t_date_not_a_date` |
| 852 | jsdate.c:373 | js_setdate | target object has `type != JS_CDATE`, e.g. `Date.prototype.setTime.call({}, 0)` | `js_typeerror(J, "not a date")` | [x] `t_date_not_a_date` |
| 853 | jsdate.c:375 | js_setdate | any `Dp_set*` producing a non-finite or out-of-range (`>8.64e15`) time, or applied to an already-NaN date | stores `TimeClip(t)` = `NAN` in `self->u.number` and pushes `NaN` | [x] `t_date_timeclip` |
| 854 | jsdate.c:381 | D_parse | `Date.parse(s)` where `parseDateTime` rejected `s` (rows 99-117) | pushes `NaN` (no exception) | [x] `t_date_parse_failures` |
| 855 | jsdate.c:423 | jsB_new_Date | `new Date(str)` with an unparseable string | date value is `NaN` (Invalid Date object, no exception) | [x] `t_date_parse_failures` |
| 856 | jsdate.c:425 | jsB_new_Date | `new Date(n)` with non-finite or `\|n\| > 8.64e15` | `TimeClip` -> `NaN` date value | [x] `t_date_timeclip` |
| 857 | jsdate.c:437 | jsB_new_Date | `new Date(y,m,...)` whose component arithmetic is NaN/out of range | `TimeClip(UTC(t))` -> `NaN` date value | [x] `t_date_utc_and_components` |
| 858 | jsdate.c:397 | D_UTC | `Date.UTC(...)` with NaN/out-of-range components | `TimeClip(t)` -> pushes `NaN` | [x] `t_date_utc_and_components` |
| 859 | jsdate.c:485 | Dp_toISOString | `toISOString` on a date whose value is not finite (`!isfinite(t)`), e.g. `new Date(NaN).toISOString()` | `js_rangeerror(J, "invalid date")` (`RangeError: invalid date`) | [x] `t_date_toisostring_invalid` |
| 860 | jsdate.c:492 | Dp_getFullYear | `isnan(t)` (Invalid Date) | pushes `NAN` | [x] `t_date_getters_nan` |
| 861 | jsdate.c:501 | Dp_getMonth | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 862 | jsdate.c:510 | Dp_getDate | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 863 | jsdate.c:519 | Dp_getDay | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 864 | jsdate.c:528 | Dp_getHours | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 865 | jsdate.c:537 | Dp_getMinutes | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 866 | jsdate.c:546 | Dp_getSeconds | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 867 | jsdate.c:555 | Dp_getMilliseconds | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 868 | jsdate.c:564 | Dp_getUTCFullYear | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 869 | jsdate.c:573 | Dp_getUTCMonth | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 870 | jsdate.c:582 | Dp_getUTCDate | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 871 | jsdate.c:591 | Dp_getUTCDay | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 872 | jsdate.c:600 | Dp_getUTCHours | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 873 | jsdate.c:609 | Dp_getUTCMinutes | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 874 | jsdate.c:618 | Dp_getUTCSeconds | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 875 | jsdate.c:627 | Dp_getUTCMilliseconds | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 876 | jsdate.c:636 | Dp_getTimezoneOffset | `isnan(t)` | pushes `NAN` | [x] `t_date_getters_nan` |
| 877 | jsdate.c:748 | Dp_setUTCHours | `setUTCHours(h)` with the minutes argument omitted — default is taken from `HourFromTime(t)` instead of `MinFromTime(t)` (upstream C bug) | minutes silently become the hour-of-day value; must be replicated for parity | [x] `t_date_setutchours_bug` |
| 878 | jsdate.c:786 | Dp_toJSON | `this` coerces (JS_HNUMBER hint) to a non-finite number, e.g. `new Date(NaN).toJSON()` | pushes `null` and returns (no error) | [x] `t_date_tojson` |
| 879 | jsdate.c:793 | Dp_toJSON | `this.toISOString` is missing or not callable, e.g. `Date.prototype.toJSON.call({})` | `js_typeerror(J, "this.toISOString is not a function")` | [x] `t_date_tojson` |
| 880 | jsfunction.c:11 | jsB_Function | any throw from `js_tostring`, `js_puts`, `jsP_parsefunction` or `jsC_compilefunction` | `js_try` handler frees `sb`, calls `jsP_freeparse(J)`, and `js_throw(J)` rethrows the original error | [x] `t_function_ctor_errors` |
| 881 | jsfunction.c:31 | jsB_Function | `new Function(params, body)` with a malformed parameter list or body (e.g. `new Function("a b", "")`, `new Function("return")` bad body) | `jsP_parsefunction` raises the parser's `SyntaxError` (file name reported as `[string]`); note args are joined with `,` and terminated by a synthetic `)` | [x] `t_function_ctor_errors` |
| 882 | jsfunction.c:53 | Fp_toString | `Function.prototype.toString` on a non-callable `this` (e.g. `Function.prototype.toString.call({})`) | `js_typeerror(J, "not a function")` | [x] `t_function_not_callable` |
| 883 | jsfunction.c:100 | Fp_apply | `Function.prototype.apply` with a non-callable `this` | `js_typeerror(J, "not a function")` | [x] `t_function_not_callable` |
| 884 | jsfunction.c:110 | Fp_apply | `argArray.length` coerces to a negative value (`n < 0`) | clamped to `n = 0`; no TypeError even though arg 2 is not a real array | [x] `t_function_apply_negative_length` |
| 885 | jsfunction.c:123 | Fp_call | `Function.prototype.call` with a non-callable `this` | `js_typeerror(J, "not a function")` | [x] `t_function_not_callable` |
| 886 | jsfunction.c:186 | Fp_bind | `Function.prototype.bind` with a non-callable `this` | `js_typeerror(J, "not a function")` | [x] `t_function_not_callable` |
| 887 | jsfunction.c:189 | Fp_bind | more bound arguments than the target's `length` (`n <= top - 2`) | bound function's `length` clamped to 0 instead of going negative | [x] `t_function_bind_length_clamp` |
| 888 | jsfunction.c:145 | callbound | `__BoundArguments__.length` coerces negative (`n < 0`) | clamped to `n = 0` | [-] jsfunction.c:145 callbound `n < 0` is unreachable. `args` is always the __BoundArguments__ property, which Fp_bind (jsfunction.c:207-212) creates with js_newarray and fills via js_setindex, so its length is exactly top-2 (small, non-negative), and it is defined JS_READONLY|JS_DONTENUM|JS_DONTCONF so no script can replace, delete or negate it. t_function_bound_calls drives both functions and proves the property cannot be subverted; the reachable sibling clamp (jsfunction.c:110, row 884) is driven by t_function_apply_negative_length. |
| 889 | jsfunction.c:169 | constructbound | `__BoundArguments__.length` coerces negative (`n < 0`) | clamped to `n = 0` | [-] jsfunction.c:169 constructbound `n < 0` is unreachable. `args` is always the __BoundArguments__ property, which Fp_bind (jsfunction.c:207-212) creates with js_newarray and fills via js_setindex, so its length is exactly top-2 (small, non-negative), and it is defined JS_READONLY|JS_DONTENUM|JS_DONTCONF so no script can replace, delete or negate it. t_function_bound_calls drives both functions and proves the property cannot be subverted; the reachable sibling clamp (jsfunction.c:110, row 884) is driven by t_function_apply_negative_length. |
| 890 | jsmath.c:13 | jsM_round | `isnan(x)` | returns `x` (NaN) -> `Math.round(NaN)` is NaN | [x] `t_math_round` |
| 891 | jsmath.c:14 | jsM_round | `isinf(x)` | returns `x` (+/-Infinity unchanged) | [x] `t_math_round` |
| 892 | jsmath.c:17 | jsM_round | `0 < x < 0.5` | returns `0` (not `floor(x+0.5)`) | [x] `t_math_round` |
| 893 | jsmath.c:18 | jsM_round | `-0.5 <= x < 0` | returns `-0` (negative zero) | [x] `t_math_round` |
| 894 | jsmath.c:78 | Math_pow | `!isfinite(y) && fabs(x) == 1`, e.g. `Math.pow(1, Infinity)`, `Math.pow(-1, -Infinity)` | pushes `NAN` (overrides C `pow`, which returns 1.0) | [x] `t_math_pow_edge` |
| 895 | jsmath.c:127 | Math_max | `Math.max()` with no arguments | pushes `-INFINITY` | [x] `t_math_minmax` |
| 896 | jsmath.c:130 | Math_max | any argument coerces to NaN (e.g. `Math.max(1, "x")`) | loop breaks with `x = NaN`; pushes `NaN` | [x] `t_math_minmax` |
| 897 | jsmath.c:145 | Math_min | `Math.min()` with no arguments | pushes `INFINITY` | [x] `t_math_minmax` |
| 898 | jsmath.c:148 | Math_min | any argument coerces to NaN | loop breaks with `x = NaN`; pushes `NaN` | [x] `t_math_minmax` |
| 899 | jsmath.c:29 | Math_acos | `\|x\| > 1` — no explicit domain check | pushes whatever libm `acos` returns (NaN) | [x] `t_math_domain` |
| 900 | jsmath.c:34 | Math_asin | `\|x\| > 1` — no explicit domain check | pushes whatever libm `asin` returns (NaN) | [x] `t_math_domain` |
| 901 | jsmath.c:71 | Math_log | `x < 0` (and `-0`/`0` -> `-Infinity`) — no explicit domain check | pushes whatever libm `log` returns (NaN / -Infinity) | [x] `t_math_domain` |
| 902 | jsmath.c:116 | Math_sqrt | `x < 0` — no explicit domain check | pushes whatever libm `sqrt` returns (NaN) | [x] `t_math_domain` |
| 903 | jsrepr.c:88 | reprobject | object identical to one already on the repr stack (`js_toobject(J,i) == js_toobject(J,-1)` for `0 <= i < top-1`) — cyclic object | writes `"{}"` and returns; no exception | [x] `t_repr_cyclic` |
| 904 | jsrepr.c:118 | reprarray | array identical to one already on the repr stack — cyclic array | writes `"[]"` and returns; no exception | [x] `t_repr_cyclic` |
| 905 | jsrepr.c:129 | reprarray | `!js_hasindex(J, -1, i)` — sparse-array hole | element is skipped entirely, but the `", "` separator at line 127 was already emitted (produces `[1, , 3]`-style output) | [x] `t_repr_sparse_array` |
| 906 | jsrepr.c:76 | reprident | property name is not a bare identifier / all-digits run terminated by NUL (`p == name \|\| *p != 0`), e.g. `"a b"`, `""`, `"1a"` | falls back to `reprstr` (emits a quoted, escaped string key) | [x] `t_repr_ident_fallback` |
| 907 | jsrepr.c:231 | reprvalue | value is a `JS_CITERATOR` object | writes `"[iterator "` with no closing `]` (upstream C bug; unbalanced output) | [x] `t_repr_iterator_unbalanced` |
| 908 | jsrepr.c:247 | js_repr | any throw inside `reprvalue` (getter throwing, OOM in `js_putc`, etc.) | `js_try` handler frees `sb` and rethrows via `js_throw(J)`; note `J->bot` is left modified | [x] `t_repr_throwing_getter` |
| 909 | jsrepr.c:262 | js_repr | `reprvalue` emitted nothing at all (`sb == NULL`) | pushes the literal string `"undefined"` | [-] jsrepr.c:262's `sb ? sb->s : "undefined"` fallback is dead code. jsrepr.c:261 runs js_putc(J,&sb,0) first and js_putc (jsintern.c:5-11) allocates the buffer when *sbp is NULL, so sb is never NULL at line 262. t_repr_empty_buffer_is_dead shows reprvalue (jsrepr.c:151) covers every js_Value type and every branch writes at least one byte, so js_tryrepr never returns an empty string. |
| 910 | jsrepr.c:279 | js_tryrepr | any error thrown while computing the repr of `idx` | catches it, pops the exception, and returns the caller-supplied `error` fallback string | [x] `t_repr_throwing_getter` |
| 911 | utf.c:58 | chartorune | bytes `0xC0 0x80` (overlong/modified-UTF-8 NUL) | `*rune = 0` and returns 2 (accepted, not an error) | [x] `t_utf_chartorune_sentinels` |
| 912 | utf.c:78 | chartorune | second byte is not a continuation byte (`c1 & Testx`), including a truncated sequence at end of string | `goto bad`: `*rune = Runeerror` (0xFFFD) and returns 1 | [x] `t_utf_chartorune_sentinels` |
| 913 | utf.c:81 | chartorune | lead byte in `0x80..0xBF` (`c < T2`) — stray continuation byte | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 914 | utf.c:84 | chartorune | 2-byte sequence decoding to `<= Rune1` (0x7F) — overlong (e.g. `0xC1 0xBF`) | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 915 | utf.c:95 | chartorune | third byte is not a continuation byte (`c2 & Testx`) | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 916 | utf.c:99 | chartorune | 3-byte sequence decoding to `<= Rune2` (0x7FF) — overlong (e.g. `0xE0 0x80 0xAF`) | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 917 | utf.c:111 | chartorune | fourth byte is not a continuation byte (`c3 & Testx`) | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 918 | utf.c:113 | chartorune | lead byte `>= T5` (0xF8..0xFF) — no 5-byte forms accepted | falls through to `bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 919 | utf.c:116 | chartorune | 4-byte sequence decoding to `<= Rune3` (0xFFFF) — overlong (e.g. `0xF0 0x80 0x80 0x80`) | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 920 | utf.c:117 | chartorune | 4-byte sequence decoding to `> Runemax` (0x10FFFF), e.g. `0xF7 0xBF 0xBF 0xBF` | `goto bad`: `*rune = 0xFFFD`, returns 1 | [x] `t_utf_chartorune_sentinels` |
| 921 | utf.c:127 | chartorune | `bad:` label — common exit for rows 182-190 | sets `*rune = Bad` (`Runeerror` = 0xFFFD) and returns 1, i.e. consumes exactly one byte | [x] `t_utf_chartorune_sentinels` |
| 922 | utf.c:138 | runetochar | `*rune == 0` | encodes the 2-byte overlong NUL `0xC0 0x80` and returns 2 (never a bare `\0`) | [x] `t_utf_runetochar_sentinels` |
| 923 | utf.c:148 | runetochar | negative `*rune` (signed `Rune`) — `c <= Rune1` is true | writes the truncated low byte `str[0] = c` and returns 1; no range rejection | [x] `t_utf_runetochar_sentinels` |
| 924 | utf.c:167 | runetochar | `c > Runemax` (0x10FFFF) | silently substitutes `c = Runeerror` (0xFFFD) and emits its 3-byte encoding, returning 3 | [x] `t_utf_runetochar_sentinels` |
| 925 | utf.c:194 | runelen | any `c`; delegates to `runetochar` with a local 10-byte buffer | returns 1/2/3/4 only — 2 for `c == 0`, 3 for `c > Runemax`; never returns an error code (no `-1`) | [x] `t_utf_runelen_sentinels` |
| 926 | utf.c:212 | ucd_bsearch | binary search finds no table entry with `c >= t[0]` (`n == 0` or `c < t[0]`) | returns 0 (NULL) | [x] `t_utf_table_misses` |
| 927 | utf.c:228 | tolowerrune | `c` has no entry in `ucd_tolower2`/`ucd_tolower1` | returns `c` unchanged | [x] `t_utf_table_misses` |
| 928 | utf.c:242 | toupperrune | `c` has no entry in `ucd_toupper2`/`ucd_toupper1` | returns `c` unchanged | [x] `t_utf_table_misses` |
| 929 | utf.c:256 | islowerrune | `c` not found in the `ucd_toupper*` tables | returns 0 | [x] `t_utf_table_misses` |
| 930 | utf.c:270 | isupperrune | `c` not found in the `ucd_tolower*` tables | returns 0 | [x] `t_utf_table_misses` |
| 931 | utf.c:284 | isalpharune | `c` not found in `ucd_alpha2`/`ucd_alpha1` | returns 0 | [x] `t_utf_table_misses` |
| 932 | utf.c:294 | tolowerrune_full | `c` has no full-lowercase mapping (`!p \|\| c != p[0]`) | returns `NULL` | [x] `t_utf_full_case_null` |
| 933 | utf.c:305 | toupperrune_full | `c` has no full-uppercase mapping (`!p \|\| c != p[0]`) | returns `NULL` | [x] `t_utf_full_case_null` |
