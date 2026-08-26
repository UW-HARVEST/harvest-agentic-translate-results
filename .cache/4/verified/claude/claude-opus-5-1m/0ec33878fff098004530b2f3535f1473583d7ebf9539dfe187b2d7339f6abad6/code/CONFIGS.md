# CONFIGS.md — configuration-surface table for `c_src/`

The mirror of `ERRORS.md`: every axis of VALID input the C code actually
branches on (runtime options/flags, and distinct input shapes), pruned to the
combinations the code treats differently. Derived by reading the C sources,
not from happy-path docs.

**Total rows: 450** — all **450** have a passing differential test.

## Phase B status

| | count |
|---|---|
| rows with a PASSING differential test (`[x]`) | **450** |
| rows with no test (`[ ]`) | **0** |

Every row is driven with MANY randomized inputs from a fixed seed (not a single
hand-picked value), through the exported symbols of BOTH shared objects, and the
outputs are compared byte-for-byte. The `cov` column names the owning test file:

| CONFIGS.md rows | test file | what it drives |
|---|---|---|
| 1-40, 183-197 | `tests/api_state.rs` | `js_newstate` flags/allocator/limits, report+panic hooks, load/eval/call variants, refs/registry/globals, GC |
| 41-70, 142-182 | `tests/api_stack.rs` | the whole stack API as randomized operation sequences, the pushes and the SHRSTR/LITSTR/MEMSTR axis, every conversion/predicate/comparison |
| 71-141 | `tests/api_props.rs` | object creation, the property API x the `JS_READONLY\|JS_DONTENUM\|JS_DONTCONF` cross-product, the flat-vs-hashed array representation, iterators |
| 198-284 | `tests/ll_regexp.rs` | `js_regcomp`/`js_regcompx`/`js_regexec`/`js_regfree`/`js_regfreex`: cflags x eflags x `sub` x allocator x every pattern shape |
| 285-307 | `tests/js_builtins.rs`, `tests/gaps.rs` | `js_newregexp` flag matrix, `RegExp.prototype.exec` lastIndex progression, `js_toregexp` |
| 308-355 | `tests/ll_num.rs` | `js_grisu2`/`js_fmtexp`/`js_itoa`/`js_strtod`/`js_strtol`/`js_stringtofloat`/`jsV_numberto*`/`jsV_numbertostring`/`jsV_stringtonumber` |
| 356-376 | `tests/ll_utf.rs` | `jsU_*`, `js_utflen`, `js_utfptrtoidx`, `js_runeat` over every UTF-8 width and boundary rune |
| 377-403 | `tests/ll_lex.rs` | both lexer modes, `jsY_findword`/`jsY_tokenstring`/`jsY_ishex`/`jsY_tohex`/`jsY_iswhite`/`jsY_isnewline` |
| 404-419 | `tests/js_builtins.rs` | `JSON.parse` reviver x value shapes, `JSON.stringify` replacer x space x value types |
| 420-450 | `tests/js_builtins.rs` | `Sp_split`/`Sp_replace`/`Sp_match`, `Ap_sort`, Date construction/parsing/formatting |

## The axes

### Runtime options settable through the public API

| option | set by | values |
|---|---|---|
| state flags | `js_newstate(alloc, actx, flags)` | `0`, `JS_STRICT` |
| allocator | `js_newstate` arg 1 | `NULL` (default realloc/free) or custom `js_Alloc` + `actx` |
| run limit | `js_setlimit(J, runlimit, memlimit)` | `runlimit` 0 (off) / >0 |
| mem limit | `js_setlimit` | `memlimit` 0 (off) / >0 |
| report hook | `js_setreport` | default (stderr) / custom / `NULL` |
| panic hook | `js_atpanic` | default / custom |
| user context | `js_setcontext` / `js_getcontext` | any pointer |
| eval-vs-script | `js_loadstring` / `js_loadeval` | `iseval` 0/1: picks `default_strict` vs `strict`, and scope `GE` vs `E` vs `NULL` |
| property attrs | `js_defproperty` / `js_defglobal` / `js_defaccessor` | cross-product of `JS_READONLY`, `JS_DONTENUM`, `JS_DONTCONF` |
| iterator scope | `js_pushiterator(J, idx, own)` | `own` 0 (walk prototype chain) / 1 (own only) |
| GC reporting | `js_gc(J, report)` | 0 / 1 |
| regexp flags (JS) | `js_newregexp(J, pat, flags)` | cross-product of `JS_REGEXP_G`, `JS_REGEXP_I`, `JS_REGEXP_M` (8 combos) |
| regexp compile flags | `js_regcomp` / `js_regcompx` `cflags` | `0`, `REG_ICASE`, `REG_NEWLINE`, both |
| regexp exec flags | `js_regexec` `eflags` | `0`, `REG_NOTBOL` (merged with `prog->flags`) |
| regexp captures out | `js_regexec` `sub` | `NULL` / real `Resub` (nsub 0..`REG_MAXSUB`=16) |
| radix | `js_strtol` | 0, 1, 2..36 |
| lexer mode | `jsY_lex` vs `jsY_lexjson` | JS vs JSON token grammar |

### Input-shape axes the code special-cases

| axis | distinct shapes |
|---|---|
| string representation | `JS_TSHRSTR` (inline, n <= 15), `JS_TLITSTR` (borrowed literal), `JS_TMEMSTR` (heap `js_String`) |
| value tag | undefined, null, boolean, number, shrstr, litstr, memstr, object |
| object class | `JS_COBJECT`, `JS_CARRAY`, `JS_CFUNCTION`, `JS_CSCRIPT`, `JS_CCFUNCTION`, `JS_CERROR`, `JS_CBOOLEAN`, `JS_CNUMBER`, `JS_CSTRING`, `JS_CREGEXP`, `JS_CDATE`, `JS_CMATH`, `JS_CJSON`, `JS_CARGUMENTS`, `JS_CITERATOR`, `JS_CUSERDATA` |
| array representation | `simple` (flat) vs unflattened hashed property tree; the conversion triggers |
| stack index sign | positive (from `bot`) vs negative (from `top`) |
| argument count | 0, 1, many; fewer / more than declared arity |
| UTF-8 width | 1, 2, 3, 4 byte sequences; boundary runes 0x7F/0x80/0x7FF/0x800/0xFFFF/0x10000/0x10FFFF; surrogates; overlong; truncated |
| double shape | 0, -0, subnormal, integral, needs-17-digits, inf, nan, INT_MIN/INT_MAX/UINT_MAX, 2^31, 2^32, 2^53 |
| number->string form | fixed vs exponential: exponential iff `point < -5 \|\| point > 21`; exact-int `js_itoa` fast path |
| empty / one / many | 0-length strings and arrays, single element, many elements |

## Rows

| # | entry point(s) | configuration (options set + input shape) | cov |
|---|----------------|--------------------------------------------|-----|
| 1 | js_newstate | alloc=NULL, actx=NULL, flags=0; default js_defaultalloc (realloc/free), J->strict=J->default_strict=0 | [x] `api_state.rs` |
| 2 | js_newstate | alloc=NULL, flags=JS_STRICT; J->strict=J->default_strict=1 set at construction | [x] `api_state.rs` |
| 3 | js_newstate + js_freestate | custom js_Alloc with non-NULL actx; the allocator is used for the js_State, for the JS_STACKSIZE*16-byte value stack and for every js_malloc/js_realloc/js_free | [x] `api_state.rs` |
| 4 | js_newstate | flags containing bits beyond JS_STRICT (e.g. 0x6); only `flags & JS_STRICT` is tested, extra bits ignored | [x] `api_state.rs` |
| 5 | js_setcontext + js_getcontext | uctx set after construction; wholly independent of actx (actx goes to the allocator, uctx to the embedder) | [x] `api_state.rs` |
| 6 | js_setreport + js_report | report left at the js_newstate default (js_defaultreport writes to stderr) vs a custom js_Report installed | [x] `api_state.rs` |
| 7 | js_setreport + js_report + js_gc | js_setreport(J, NULL); every js_report becomes a silent no-op via the `if (J->report)` guard | [x] `api_state.rs` |
| 8 | js_atpanic + js_throw | panic left at the default js_defaultpanic and a throw with trytop==0; it reports "uncaught exception", returns, and js_throw falls through to abort() | [x] `api_state.rs` |
| 9 | js_atpanic + js_throw | custom js_Panic installed (return value is the previous panic); throw with trytop==0 invokes it | [x] `api_state.rs` |
| 10 | js_setlimit | runlimit=0 and memlimit=0, or negative values for either; both checks are `> 0` so negatives behave exactly like unlimited | [x] `api_state.rs` |
| 11 | js_setlimit + js_dostring | runlimit>0, memlimit=0; one decrement per VM instruction inside jsR_run only, so pure C-API sequences never consume budget | [x] `api_state.rs` |
| 12 | js_setlimit + js_pushstring/js_newobject | memlimit>0, runlimit=0; every js_malloc and js_realloc subtracts `size` while js_free credits nothing back, so the budget is monotonic | [x] `api_state.rs` |
| 13 | js_setlimit | runlimit>0 and memlimit>0 simultaneously (two independent counters) | [x] `api_state.rs` |
| 14 | js_loadstring | flags=0 (default_strict=0), source without a "use strict" prologue; jsC_compilescript(default_strict=0), script env = J->GE | [x] `api_state.rs` |
| 15 | js_loadstring | flags=JS_STRICT (default_strict=1); jsC_compilescript(default_strict=1), script env still J->GE | [x] `api_state.rs` |
| 16 | js_loadstring | flags=0 but the source begins with a "use strict" directive prologue; F->strict forced to 1 by the compiler | [x] `api_state.rs` |
| 17 | js_loadeval | iseval=1 while J->strict==0; compiled with strict=0 and captured env = NULL, so the script runs in the caller's environment with no savescope | [x] `api_state.rs` |
| 18 | js_loadeval | iseval=1 while J->strict==1 (called from inside a strict function); compiled strict and captured env = J->E | [x] `api_state.rs` |
| 19 | js_ploadstring | valid source with trytop < JS_TRYLIMIT; returns 0 and leaves the script object on the stack | [x] `api_state.rs` |
| 20 | js_ploadstring | invoked at trytop == JS_TRYLIMIT (64); js_ptry pushes the "exception stack overflow" literal and returns 1 without parsing | [x] `api_state.rs` |
| 21 | js_dostring | valid source; loadstring + js_pushundefined as `this` + js_call(J,0) + pop, returns 0. Because `this` is undefined, non-strict OP_THIS substitutes the global object | [x] `api_state.rs` |
| 22 | js_eval | string of any tag on top (loadeval + rot2pop1 + copy(0) as this + call(0)) vs non-string on top (early return, stack untouched) | [x] `api_state.rs` |
| 23 | js_call | n=0 on a JS_CCFUNCTION declared with u.c.length==0 | [x] `api_state.rs` |
| 24 | js_call | n < u.c.length on a cfunction; jsR_callcfunction pads with js_pushundefined up to `min` | [x] `api_state.rs` |
| 25 | js_call | n > u.c.length on a cfunction; the surplus args stay addressable and js_gettop reports the real count | [x] `api_state.rs` |
| 26 | js_call | cfunction body that pushes nothing (TOP == save_top, result undefined) vs one that pushes several values (only the topmost survives TOP = --BOT) | [x] `api_state.rs` |
| 27 | js_call | JS_CFUNCTION with F->lightweight==1 and n == F->numparams | [x] `api_state.rs` |
| 28 | js_call | lightweight function with n > numparams; jsR_calllwfunction pops n-numparams then clamps n | [x] `api_state.rs` |
| 29 | js_call | lightweight function with n < numparams; undefined pushed for slots n..F->varlen-1 | [x] `api_state.rs` |
| 30 | js_call | non-lightweight JS_CFUNCTION with F->arguments==0; fresh environment plus jsR_savescope, params bound by js_initvar with JS_DONTENUM+JS_DONTCONF | [x] `api_state.rs` |
| 31 | js_call | non-lightweight JS_CFUNCTION, F->arguments==1, J->strict==0; the arguments object gains 'callee' (JS_DONTENUM) and 'length' | [x] `api_state.rs` |
| 32 | js_call | non-lightweight JS_CFUNCTION, F->arguments==1, J->strict==1; 'callee' omitted, 'length' still defined | [x] `api_state.rs` |
| 33 | js_call | JS_CSCRIPT with scope==J->GE (produced by js_loadstring); all n args popped, vars defined only where js_hasvar reports them absent | [x] `api_state.rs` |
| 34 | js_call | JS_CSCRIPT with scope==NULL (produced by non-strict js_loadeval); jsR_callscript skips savescope/restorescope and mutates the caller's environment | [x] `api_state.rs` |
| 35 | js_pcall | n=0/1/many with a normal return (0, result at savetop=TOP-n-2) vs a throwing callee (1, stack trimmed to savetop+1 holding only the error) | [x] `api_state.rs` |
| 36 | js_construct | JS_CCFUNCTION with u.c.constructor set, n=0 (null `this` pushed, no rot) vs n>0 (null pushed then js_rot(n+1) slides it under the args) | [x] `api_state.rs` |
| 37 | js_construct | JS_CCFUNCTION with u.c.constructor==NULL (a plain js_newcfunction); falls into the generic prototype+newobject path | [x] `api_state.rs` |
| 38 | js_construct | JS_CFUNCTION whose 'prototype' property is an object (used as newobj->prototype) vs missing or non-object (falls back to J->Object_prototype) | [x] `api_state.rs` |
| 39 | js_construct | constructor body returning an object (js_rot2pop1 discards the auto-created one) vs returning a primitive or nothing (auto-created object kept) | [x] `api_state.rs` |
| 40 | js_pconstruct | success (returns 0) vs throwing (returns 1 with the stack trimmed to savetop=TOP-n-2 plus one slot) | [x] `api_state.rs` |
| 41 | js_gettop | called at top level (BOT==0) vs inside a cfunction frame (BOT>0, so the callee and `this` slots are excluded) | [x] `api_stack.rs` |
| 42 | js_pop | n=0 (no-op) and n == js_gettop() (frame emptied exactly down to BOT) | [x] `api_stack.rs` |
| 43 | js_pop | n > js_gettop(); TOP clamped back to BOT and js_error "stack underflow!" | [x] `api_stack.rs` |
| 44 | js_copy | idx >= 0 (BOT-relative, js_copy(J,0) duplicates `this`) vs idx < 0 (TOP-relative) | [x] `api_stack.rs` |
| 45 | js_copy | idx out of range in either direction (99 or -99); stackidx returns the shared static undefined and undefined is pushed with no error | [x] `api_stack.rs` |
| 46 | js_remove | idx=-1 (top, the shift loop never runs) vs idx=0 with values above it (every slot shifted down one) | [x] `api_stack.rs` |
| 47 | js_remove | idx resolving below BOT or at/after TOP; js_error "stack error!" | [x] `api_stack.rs` |
| 48 | js_replace | idx=-1 (self-assign then --TOP, a net pop) vs idx=0 (overwrites `this` and pops) | [x] `api_stack.rs` |
| 49 | js_replace | idx resolving outside the current frame; js_error "stack error!" | [x] `api_stack.rs` |
| 50 | js_insert | any idx with any stack shape; unconditionally js_error "not implemented yet" | [x] `api_stack.rs` |
| 51 | js_dup + js_dup2 | one value present for dup, two for dup2 (dup2 needs two free slots, CHECKSTACK(2)) | [x] `api_stack.rs` |
| 52 | js_rot2 / js_rot3 / js_rot4 | exactly 2/3/4 values in the frame; there is no bounds checking at all, so fewer values makes these read slots below BOT | [x] `api_stack.rs` |
| 53 | js_rot2pop1 / js_rot3pop2 | 2 resp. 3 values present; the top value is moved down and TOP reduced, with no CHECKSTACK | [x] `api_stack.rs` |
| 54 | js_rot | n=0 and n=1 (loop never advances, top rewritten with itself, a no-op) vs n=2 (identical to js_rot2) vs n=k>2 | [x] `api_stack.rs` |
| 55 | js_pushundefined and every other push | invoked when TOP+1 >= JS_STACKSIZE (4096); CHECKSTACK throws "stack overflow" as a JS_TLITSTR value | [x] `api_stack.rs` |
| 56 | js_currentfunction + js_currentfunctiondata | BOT>0 inside a call (STACK[BOT-1], and u.c.data for a js_newcfunctionx callee) vs BOT==0 at top level (undefined, and NULL) | [x] `api_stack.rs` |
| 57 | js_pushundefined + js_pushnull | JS_TUNDEFINED vs JS_TNULL as seen by js_isundefined/js_isdefined/js_isnull/js_iscoercible/js_typeof (null typeofs as "object") | [x] `api_stack.rs` |
| 58 | js_pushboolean | v=0 vs v=nonzero (e.g. 42); stored normalised as !!v so js_strictequal against true still matches | [x] `api_stack.rs` |
| 59 | js_pushnumber + js_tostring | integral value inside [INT_MIN,INT_MAX] (js_itoa fast path) vs non-integral or out-of-range (0.1, 1e21, 1e-7, 1e300) taking grisu2 with the point<-5 or point>21 exponent form | [x] `api_stack.rs` |
| 60 | js_pushnumber + js_tostring + js_repr | NaN, +Infinity, -Infinity and -0.0; tostring gives "NaN"/"Infinity"/"-Infinity"/"0" while js_repr renders negative zero as "-0" | [x] `api_stack.rs` |
| 61 | js_pushstring + js_tostring | strlen(v) <= 15, including exactly 15; JS_TSHRSTR stored inline with the NUL terminator landing in (and doubling as) the t.type byte | [x] `api_stack.rs` |
| 62 | js_pushstring | strlen(v) == 16, the first size above the inline capacity; JS_TMEMSTR with jsV_newmemstring allocating and bumping J->gccounter | [x] `api_stack.rs` |
| 63 | js_pushstring | empty string; JS_TSHRSTR with shrstr[0]==0, js_toboolean==0 | [x] `api_stack.rs` |
| 64 | js_pushstring / js_pushlstring | length greater than JS_STRLIMIT (1<<28); js_rangeerror "invalid string length" raised before any copying | [x] `api_stack.rs` |
| 65 | js_pushlstring | n=0 with any pointer; the empty JS_TSHRSTR results and v is never dereferenced | [x] `api_stack.rs` |
| 66 | js_pushlstring | n=15 vs n=16; the SHRSTR/MEMSTR boundary is keyed on the caller-supplied n, not on strlen(v) | [x] `api_stack.rs` |
| 67 | js_pushlstring + js_tostring + js_equal | n spanning embedded NUL bytes (e.g. "a\0b", n=3); all n bytes are stored but every consumer (jsV_tostring, the strcmp in js_equal, js_toboolean) stops at the first NUL | [x] `api_stack.rs` |
| 68 | js_pushliteral | JS_TLITSTR; only the pointer is recorded, with no copy and no GC ownership, so the buffer must outlive the value | [x] `api_stack.rs` |
| 69 | js_pushliteral + js_pushstring + js_strictequal/js_equal | the same bytes pushed as LITSTR and as SHRSTR/MEMSTR; JSV_ISSTRING makes both comparisons strcmp-based so the differing tags still compare equal | [x] `api_stack.rs` |
| 70 | js_pushglobal | pushes J->G itself (JS_TOBJECT / JS_COBJECT), so global properties are mutable through the ordinary property API | [x] `api_stack.rs` |
| 71 | js_newobject | JS_COBJECT with prototype = J->Object_prototype, extensible=1, count=0 | [x] `api_props.rs` |
| 72 | js_newobjectx | an object on top (popped and used as the prototype) vs a non-object on top (prototype = NULL, no error, argument still popped) | [x] `api_props.rs` |
| 73 | js_newarray | JS_CARRAY with u.a.simple=1, flat_length=0, flat_capacity=0, array=NULL, length=0, prototype=J->Array_prototype | [x] `api_props.rs` |
| 74 | js_newboolean + js_newnumber | v=0 vs nonzero for boolean, ordinary/NaN/Infinity for number; JS_CBOOLEAN and JS_CNUMBER wrappers whose js_toboolean is always 1 | [x] `api_props.rs` |
| 75 | js_newstring | strlen(v) < 16; the string is stored inline in obj->u.s.shrstr and u.s.string aliases it, so jsG_freeobject must not free it | [x] `api_props.rs` |
| 76 | js_newstring | strlen(v) >= 16; js_strdup heap copy which jsG_freeobject does free because u.s.string != u.s.shrstr | [x] `api_props.rs` |
| 77 | js_newstring + js_getproperty(idx,"length") | multi-byte UTF-8 including astral (>= 0x10000) runes; u.s.length comes from js_utflen so astral runes count as 2 | [x] `api_props.rs` |
| 78 | js_newcfunction | length=0 vs length>0; both define 'length' with JS_READONLY+JS_DONTENUM+JS_DONTCONF plus a fresh 'prototype' object (JS_DONTENUM+JS_DONTCONF) carrying 'constructor' (JS_DONTENUM) | [x] `api_props.rs` |
| 79 | js_newcfunctionx | data!=NULL with finalize!=NULL (data via js_currentfunctiondata, finalize run from jsG_freeobject, and also run before the rethrow if construction throws) vs finalize==NULL (no finalizer registered) | [x] `api_props.rs` |
| 80 | js_newcconstructor | prototype object on top; builds a JS_CCFUNCTION with both u.c.function and u.c.constructor, cross-links 'constructor' and 'prototype', and leaves the constructor on the stack | [x] `api_props.rs` |
| 81 | js_newuserdata | object on top used as the prototype vs a non-object on top (prototype NULL, so the userdata inherits nothing); has/put/delete all NULL so property ops fall through to the generic tree | [x] `api_props.rs` |
| 82 | js_newuserdatax + js_hasproperty/js_getproperty | has != NULL; the hook is consulted before the property tree and a nonzero return means the hook already pushed the value | [x] `api_props.rs` |
| 83 | js_newuserdatax + js_setproperty/js_defproperty | put != NULL; a nonzero return short-circuits before any tree insert, attribute check or readonly handling | [x] `api_props.rs` |
| 84 | js_newuserdatax + js_delproperty | delete != NULL; a nonzero return short-circuits and jsR_delproperty reports success | [x] `api_props.rs` |
| 85 | js_isuserdata + js_touserdata | tag matching exactly (strcmp==0) vs a different tag; the mismatch makes js_touserdata raise typeerror "not a <tag>" | [x] `api_props.rs` |
| 86 | js_getproperty | plain JS_COBJECT with the property owned directly vs found only by walking obj->prototype | [x] `api_props.rs` |
| 87 | js_getproperty | the property has a getter; getter object and obj are pushed and js_call(J,0) runs, its result becoming the value | [x] `api_props.rs` |
| 88 | js_getproperty | name absent from the object and the whole chain; jsR_getproperty pushes undefined | [x] `api_props.rs` |
| 89 | js_getproperty | idx holds a primitive string/number/boolean (js_toobject builds a wrapper AND rewrites the stack slot to JS_TOBJECT) vs undefined/null (typeerror "cannot convert ... to object") | [x] `api_props.rs` |
| 90 | js_getproperty + js_getindex | JS_CSTRING target with "length" (from u.s.length), an in-range index name (js_pushrune/js_runeat) and an out-of-range index (falls through to the tree); JS_CREGEXP target with source/global/ignoreCase/multiline/lastIndex synthesised ahead of the tree | [x] `api_props.rs` |
| 91 | js_setproperty | plain object, brand-new name; jsV_setproperty inserts into the AA-tree and bumps obj->count and J->gccounter | [x] `api_props.rs` |
| 92 | js_setproperty | a setter exists anywhere in the prototype chain; setter, obj and value pushed, js_call(J,1), result popped, and attributes are not consulted | [x] `api_props.rs` |
| 93 | js_setproperty | the resolved property has a getter but no setter; J->strict=1 raises a typeerror, J->strict=0 silently drops the write | [x] `api_props.rs` |
| 94 | js_setproperty | the resolved property carries JS_READONLY; J->strict=1 typeerror "'x' is read-only", J->strict=0 silent no-op | [x] `api_props.rs` |
| 95 | js_setproperty | property found only on the prototype (own==0) and writable; a NEW own property is created on the receiver rather than the prototype being modified | [x] `api_props.rs` |
| 96 | js_setproperty | idx holds a primitive so transient==1; J->strict=1 typeerror "cannot create property on transient object", J->strict=0 discards the write silently | [x] `api_props.rs` |
| 97 | js_setproperty | new name on an object made non-extensible by Object.preventExtensions/seal/freeze; J->strict=1 typeerror "object is non-extensible", J->strict=0 gets a NULL ref and drops the write | [x] `api_props.rs` |
| 98 | js_setproperty | JS_CSTRING target with "length" or an in-range index name (readonly path); JS_CREGEXP target with "lastIndex" (writes u.r.last via jsV_tointeger) vs the other four regexp names (readonly path) | [x] `api_props.rs` |
| 99 | js_defproperty | atts=0 on a fresh name | [x] `api_props.rs` |
| 100 | js_defproperty | atts=JS_READONLY only; a later js_setproperty is refused | [x] `api_props.rs` |
| 101 | js_defproperty | atts=JS_DONTENUM only; invisible to js_pushiterator, Object.keys and js_repr but still readable and writable | [x] `api_props.rs` |
| 102 | js_defproperty | atts=JS_DONTCONF only; js_delproperty refuses to remove it | [x] `api_props.rs` |
| 103 | js_defproperty | atts=JS_READONLY+JS_DONTENUM | [x] `api_props.rs` |
| 104 | js_defproperty | atts=JS_READONLY+JS_DONTCONF | [x] `api_props.rs` |
| 105 | js_defproperty | atts=JS_DONTENUM+JS_DONTCONF (the js_initvar pattern used for function locals) | [x] `api_props.rs` |
| 106 | js_defproperty | atts=JS_READONLY+JS_DONTENUM+JS_DONTCONF (the jsB_propn pattern used for NaN, Infinity and undefined) | [x] `api_props.rs` |
| 107 | js_defproperty | name that already exists with attributes; the atts are only ever OR-ed into ref->atts, so attributes can never be cleared by redefinition | [x] `api_props.rs` |
| 108 | js_defproperty | a value supplied for a property already marked JS_READONLY; a typeerror is raised only when J->strict=1, because the value branch ignores the throw flag | [x] `api_props.rs` |
| 109 | js_defproperty | reserved name on an array ("length"), a JS_CSTRING ("length" or an in-range index) or a regexp flag name; throw=1 so the typeerror fires even with J->strict=0 | [x] `api_props.rs` |
| 110 | js_defaccessor | getter at -2 and setter at -1 both callable objects; both installed, atts OR-ed in, both operands popped | [x] `api_props.rs` |
| 111 | js_defaccessor | one side callable and the other undefined or null; jsR_tofunction returns NULL so that slot is left untouched (an existing accessor survives) | [x] `api_props.rs` |
| 112 | js_defaccessor | an operand that is a non-callable object or a primitive; jsR_tofunction raises typeerror "not a function" | [x] `api_props.rs` |
| 113 | js_defaccessor | target property already carries JS_DONTCONF; J->strict=1 typeerror "non-configurable", J->strict=0 silently keeps the old accessor | [x] `api_props.rs` |
| 114 | js_delproperty | configurable own property (node unlinked, obj->count decremented), absent name or prototype-only name (both reported as success), and a JS_DONTCONF property (strict typeerror vs non-strict quiet failure) | [x] `api_props.rs` |
| 115 | js_hasproperty | return 1 also leaves the value on the stack for the caller to pop, return 0 pushes nothing; a JS_CUSERDATA `has` hook may return nonzero without pushing anything | [x] `api_props.rs` |
| 116 | js_getlength + js_setlength | array target; js_setlength with a negative idx relies on the idx-1 compensation for the number it just pushed. js_getlength on a plain object with no 'length' yields tointeger(undefined)==0, and on a JS_CSTRING yields the rune count | [x] `api_props.rs` |
| 117 | js_getindex / js_setindex / js_hasindex / js_delindex | plain JS_COBJECT target with names formatted by js_itoa, including a negative k where "-1" is a legal property name that js_isarrayindex rejects; js_setindex with a primitive idx propagates transient=1 | [x] `api_props.rs` |
| 118 | js_newarray + js_setindex | dense append with k == flat_length repeated; the array stays simple and flat_capacity grows 0 -> 8 -> 16 -> 32 ... by doubling inside jsR_setarrayindex | [x] `api_props.rs` |
| 119 | js_setindex / js_setproperty | simple array with 0 <= k < flat_length; in-place overwrite of u.a.array[k] leaving flat_length and length unchanged | [x] `api_props.rs` |
| 120 | js_setindex / js_setproperty | simple array with a sparse write at k > flat_length; jsR_unflattenarray migrates every flat slot into the property tree, frees u.a.array, clears simple/flat_length/flat_capacity, and then the value is stored as a normal property with u.a.length = k+1 | [x] `api_props.rs` |
| 121 | js_setproperty(idx,"length") / js_setlength | simple array with newlen <= flat_length; u.a.length is set and flat_length truncated, dropping the tail values outright | [x] `api_props.rs` |
| 122 | js_setproperty(idx,"length") / js_setlength | simple array with newlen > flat_length; only u.a.length grows while flat_length is untouched, leaving an array that is logically sparse yet still flagged simple | [x] `api_props.rs` |
| 123 | js_setproperty(idx,"length") | unflattened array shrinking with u.a.length > obj->count*2; jsV_resizearray takes the own-iterator path and deletes only names that round-trip through jsV_numbertostring as integers >= newlen | [x] `api_props.rs` |
| 124 | js_setproperty(idx,"length") | unflattened array shrinking with u.a.length <= obj->count*2; jsV_resizearray takes the dense loop calling jsV_delproperty for every k in [newlen, length) | [x] `api_props.rs` |
| 125 | js_setproperty(idx,"length") / js_setindex | non-integral or negative length (1.5, -1, NaN) gives rangeerror "invalid array length"; any resulting length above JS_ARRAYLIMIT (1<<26) gives rangeerror "array too large" from jsR_setproperty or jsR_setarrayindex | [x] `api_props.rs` |
| 126 | js_delindex | simple array with k == flat_length-1 (the last flat element); flat_length is simply decremented and the array stays flat | [x] `api_props.rs` |
| 127 | js_delindex | simple array with k < flat_length-1; routed through jsR_delproperty, which unflattens the entire array before deleting | [x] `api_props.rs` |
| 128 | js_delproperty | array target with name "length"; the dontconf path (strict typeerror vs non-strict silent failure) | [x] `api_props.rs` |
| 129 | js_defproperty / js_defaccessor | simple array target with any name at all; jsR_defproperty unflattens unconditionally before touching the tree | [x] `api_props.rs` |
| 130 | js_setproperty | simple array with a non-index name such as "foo"; js_isarrayindex fails, the generic tree insert runs, and the flat part is PRESERVED so both representations are live at once | [x] `api_props.rs` |
| 131 | js_setproperty / js_getproperty | array with index-like names that js_isarrayindex accepts vs rejects: "0" accepted, while "", "01", "1.5", "-1", " 1" and decimal strings at or above INT_MAX/10 are rejected and become plain properties that leave u.a.length alone | [x] `api_props.rs` |
| 132 | js_gc + js_setproperty | reachable simple array whose flat slots are scanned by jsG_scanobject and freed by jsG_freeobject only when u.a.simple; plus a jsR_unflattenarray interrupted by a throw, where the js_try handler resets obj->properties to NULL and rethrows | [x] `api_props.rs` |
| 133 | js_pushiterator + js_nextiterator | own=1 over a plain object; itwalk over obj->properties only with seen=NULL, prototype chain ignored entirely | [x] `api_props.rs` |
| 134 | js_pushiterator + js_nextiterator | own=0 over an object whose prototype also has enumerable properties; itflatten walks prototypes first and skips names shadowed by jsV_getenumproperty on the prototype | [x] `api_props.rs` |
| 135 | js_pushiterator + js_nextiterator | own=0 and own=1 with JS_DONTENUM properties present; skipped in both modes by the itwalk attribute test | [x] `api_props.rs` |
| 136 | js_pushiterator + js_nextiterator | simple array target; u.iter.n = flat_length so index names "0".."n-1" are produced first, then the tree property names | [x] `api_props.rs` |
| 137 | js_pushiterator + js_nextiterator | unflattened array target; u.iter.n = 0 so every index name comes from the AA-tree in string order ("10" before "9") | [x] `api_props.rs` |
| 138 | js_pushiterator + js_nextiterator | JS_CSTRING target; u.iter.n = u.s.length so every character index is enumerated before the own properties | [x] `api_props.rs` |
| 139 | js_pushiterator | idx holds a primitive string; js_toobject wraps it first and the stack slot is rewritten to the wrapper object | [x] `api_props.rs` |
| 140 | js_pushiterator + js_delproperty + js_nextiterator | property deleted after the name list was snapshotted (the jsV_getproperty re-check skips the stale name), iteration driven to NULL, and index names invalidated by the next call because J->scratch is one shared 12-byte buffer | [x] `api_props.rs` |
| 141 | js_nextiterator + js_gc | idx holds an object whose class is not JS_CITERATOR (typeerror "not an iterator"); a live iterator has its u.iter.target marked by jsG_scanobject and its name list released by jsG_freeiterator | [x] `api_props.rs` |
| 142 | js_toboolean | JS_TUNDEFINED and JS_TNULL both 0; JS_TBOOLEAN returned verbatim | [x] `api_stack.rs` |
| 143 | js_toboolean | JS_TNUMBER 0, -0.0 and NaN all 0; every other number 1 | [x] `api_stack.rs` |
| 144 | js_toboolean | JS_TSHRSTR, JS_TLITSTR and JS_TMEMSTR, each empty vs non-empty; only byte 0 is inspected so a leading NUL from js_pushlstring reads as falsy | [x] `api_stack.rs` |
| 145 | js_toboolean | JS_TOBJECT of any class, including new Boolean(false), new Number(0), new String("") and an empty array; always 1 | [x] `api_stack.rs` |
| 146 | js_tonumber | JS_TUNDEFINED gives NaN, JS_TNULL gives 0, JS_TBOOLEAN gives 0 or 1 | [x] `api_stack.rs` |
| 147 | js_tonumber | string shapes through jsV_stringtonumber: "0x1f" hex, "Infinity"/"+Infinity"/"-Infinity", leading and trailing whitespace or newlines, "" giving 0, "12abc" giving NaN, and the float ("1.5", "1e3") vs integer ("12") parse paths | [x] `api_stack.rs` |
| 148 | js_tonumber | JS_CNUMBER, JS_CSTRING and JS_CBOOLEAN wrappers and JS_CDATE; jsV_toprimitive is called with JS_HNUMBER so valueOf is tried before toString | [x] `api_stack.rs` |
| 149 | js_tonumber + js_tostring | object with neither a callable toString nor a callable valueOf (e.g. Object.create(null)); J->strict=0 substitutes the literal "[object]" so tonumber is NaN, J->strict=1 raises typeerror "cannot convert object to primitive" | [x] `api_stack.rs` |
| 150 | js_tostring | each string tag; the returned pointer aliases the value itself, so for JS_TSHRSTR it points INTO the stack slot and is invalidated by any subsequent push or pop | [x] `api_stack.rs` |
| 151 | js_tostring | JS_TNUMBER whose textual form is <= 15 chars (memoised back into the stack slot as JS_TSHRSTR) vs > 15 chars such as 1.2345678901234567e+300 (memoised as a freshly allocated JS_TMEMSTR) | [x] `api_stack.rs` |
| 152 | js_tostring | JS_TOBJECT with JS_HSTRING so toString is tried before valueOf; JS_CARRAY going through Array.prototype.toString/join with the Ap_join_cycle guard; JS_CDATE where strings are already the JS_HNONE default | [x] `api_stack.rs` |
| 153 | js_tointeger | NaN and ±0 give 0, fractional values truncate toward zero, and values below INT_MIN or above INT_MAX (including ±Infinity) clamp to INT_MIN/INT_MAX | [x] `api_stack.rs` |
| 154 | js_toint32 | 0 and non-finite values give 0; 2^31 wraps to -2^31; 2^32+5 gives 5; -1 stays -1; fractional values go through floor/ceil | [x] `api_stack.rs` |
| 155 | js_touint32 | -1 gives 4294967295 and 2^32 gives 0 (the toint32 result reinterpreted unsigned) | [x] `api_stack.rs` |
| 156 | js_toint16 + js_touint16 | values that need truncation of the int32 result: 65535, 65536, -1, 32768 | [x] `api_stack.rs` |
| 157 | js_typeof + js_type | every tag: SHRSTR/LITSTR/MEMSTR give "string"/JS_ISSTRING, undefined, null gives "object"/JS_ISNULL, boolean, number, and objects give "object"/JS_ISOBJECT | [x] `api_stack.rs` |
| 158 | js_typeof + js_iscallable | JS_CFUNCTION and JS_CCFUNCTION give "function"/JS_ISFUNCTION, while JS_CSCRIPT gives "object"/JS_ISOBJECT even though js_iscallable reports it as callable | [x] `api_stack.rs` |
| 159 | js_isarray / js_isregexp / js_iserror / js_isnumberobject / js_isstringobject / js_isbooleanobject / js_isdateobject / js_isprimitive / js_isobject | class discrimination across JS_COBJECT, JS_CARRAY, JS_CREGEXP, JS_CERROR, JS_CNUMBER, JS_CSTRING, JS_CBOOLEAN, JS_CDATE, JS_CUSERDATA and the non-object tags | [x] `api_stack.rs` |
| 160 | js_trystring / js_trynumber / js_tryinteger / js_tryboolean | value that converts without throwing; the real result is returned and trytop is restored by js_endtry | [x] `api_stack.rs` |
| 161 | js_trystring / js_trynumber / js_tryinteger / js_tryboolean | value whose valueOf/toString throws; the caller-supplied default is returned after popping exactly one value | [x] `api_stack.rs` |
| 162 | js_trystring / js_trynumber / js_tryinteger / js_tryboolean | invoked while trytop == JS_TRYLIMIT (64); the js_ptry pre-check pops and returns the default without attempting the conversion at all | [x] `api_stack.rs` |
| 163 | js_toregexp + js_touserdata | JS_TOBJECT of the expected class vs any other value or class; the mismatch cases raise typeerror "not a regexp" resp. "not a <tag>" | [x] `api_stack.rs` |
| 164 | js_repr + js_torepr | primitives including -0 rendered "-0"; string escaping for quote, backslash, \b \f \n \r \t, control bytes below 0x20 as \xHH, runes below 0x10000 as \uHHHH and astral runes copied raw; keys emitted bare when reprident accepts them, quoted otherwise | [x] `api_stack.rs` |
| 165 | js_repr | JS_COBJECT via reprobject (own enumerable keys through an own=1 iterator) and JS_CARRAY via reprarray (js_getlength plus js_hasindex, so holes are skipped) | [x] `api_stack.rs` |
| 166 | js_repr | self-referential object or array; the frame scan (bot moved to top-1) detects the cycle and emits "{}" or "[]" | [x] `api_stack.rs` |
| 167 | js_repr | JS_CFUNCTION/JS_CSCRIPT via reprfun (numparams and vartab names) vs JS_CCFUNCTION ("[native code]"); JS_CBOOLEAN, JS_CNUMBER and JS_CSTRING wrappers; JS_CREGEXP with its g/i/m letters; JS_CDATE, JS_CMATH, JS_CJSON, JS_CITERATOR and JS_CUSERDATA (tag) | [x] `api_stack.rs` |
| 168 | js_repr + js_torepr + js_tryrepr | JS_CERROR with and without a 'message' property; js_torepr replaces the value at idx (with the idx<0 adjustment); js_tryrepr returns the caller default when repr throws | [x] `api_stack.rs` |
| 169 | js_compare | both operands string after toprimitive(JS_HNUMBER) so strcmp is used, vs both numeric; okay=1 in each case | [x] `api_stack.rs` |
| 170 | js_compare | either operand NaN, or a string that converts to NaN; okay=0 and every relational opcode therefore yields false | [x] `api_stack.rs` |
| 171 | js_compare | number vs string, and object vs primitive; both sides are pushed through toprimitive with JS_HNUMBER so even Dates compare numerically here | [x] `api_stack.rs` |
| 172 | js_equal | both operands string-ish across mixed tags (SHRSTR vs LITSTR vs MEMSTR); JSV_ISSTRING plus strcmp | [x] `api_stack.rs` |
| 173 | js_equal | identical tags: undefined/undefined, null/null, number/number (NaN never equal), boolean/boolean, and object/object compared by pointer | [x] `api_stack.rs` |
| 174 | js_equal | null vs undefined in either order; always 1 | [x] `api_stack.rs` |
| 175 | js_equal | number vs string in either order; the string is converted with jsV_tonumber | [x] `api_stack.rs` |
| 176 | js_equal | boolean vs number or string; the boolean stack VALUE is rewritten in place to JS_TNUMBER before the retry, mutating the caller's slot | [x] `api_stack.rs` |
| 177 | js_equal | object vs string/number (jsV_toprimitive with JS_HNONE mutates the slot then retries, so Date prefers string and everything else number) vs object against boolean, undefined or null where no coercion rule applies and the answer is 0 | [x] `api_stack.rs` |
| 178 | js_strictequal | mixed string tags with equal bytes give 1; operands of different kinds such as number vs string give 0 without any coercion | [x] `api_stack.rs` |
| 179 | js_strictequal | NaN vs NaN gives 0, +0 vs -0 gives 1, two handles to the same object give 1, and two structurally equal but distinct objects give 0 | [x] `api_stack.rs` |
| 180 | js_instanceof | callable rhs with a hit somewhere up the prototype chain gives 1; a non-object lhs gives 0 without rhs.prototype ever being read | [x] `api_stack.rs` |
| 181 | js_instanceof | rhs not callable (typeerror "invalid operand") and rhs.prototype not an object (typeerror "'prototype' property is not an object") | [x] `api_stack.rs` |
| 182 | js_concat | either operand string-ish after toprimitive(JS_HNONE), so a malloc'd join is re-pushed through js_pushstring and re-classified SHRSTR (<=15 bytes) or MEMSTR; both operands numeric/boolean/null/undefined take numeric addition instead; object operands under JS_HNONE let JS_CDATE prefer toString while other classes prefer valueOf, which decides join vs add | [x] `api_stack.rs` |
| 183 | js_ref | undefined, null, true or false on top; the fixed ref names "_Undefined", "_Null", "_True" and "_False" with no interning | [x] `api_state.rs` |
| 184 | js_ref | JS_TOBJECT on top; "%p" of the object pointer, interned, so the same object always yields the same ref string | [x] `api_state.rs` |
| 185 | js_ref | number or any string tag on top; the sequential J->nextref counter formatted and interned, producing a fresh ref on every call even for equal values | [x] `api_state.rs` |
| 186 | js_ref + js_unref + js_getregistry | round-trip through J->R; js_unref just delegates to js_delregistry | [x] `api_state.rs` |
| 187 | js_setregistry + js_getregistry + js_delregistry | setregistry consumes (pops) its value; getregistry for a name never set pushes undefined; delregistry on a missing name is a no-op | [x] `api_state.rs` |
| 188 | js_getglobal + js_setglobal + js_delglobal | ordinary names on J->G; setglobal pops its value and getglobal of a missing name pushes undefined | [x] `api_state.rs` |
| 189 | js_defglobal | atts=0, atts=JS_DONTENUM (the jsB_globalf pattern) and atts=JS_READONLY+JS_DONTENUM+JS_DONTCONF (the NaN/Infinity/undefined pattern); the value is popped in every case | [x] `api_state.rs` |
| 190 | js_gc | report=0; the collection runs silently with no snprintf and no js_report call | [x] `api_state.rs` |
| 191 | js_gc | report=1 with a report callback installed (the 256-byte "garbage collected (n%)" summary reaches it) vs report=1 after js_setreport(J,NULL) (summary formatted then discarded) | [x] `api_state.rs` |
| 192 | js_gc + js_dostring | two consecutive collections so J->gcmark alternates 1 -> 2 -> 1; afterwards gccounter=remaining and gcthresh=remaining*JS_GCFACTOR (5.0), and jsR_run auto-collects whenever gccounter > gcthresh, which for a fresh state (gcthresh==0) fires on the very first instruction | [x] `api_state.rs` |
| 193 | js_gc | unreachable JS_CUSERDATA and js_newcfunctionx objects; their js_Finalize callbacks run from jsG_freeobject during the sweep | [x] `api_state.rs` |
| 194 | js_gc | values reachable only from the value stack (jsG_markstack over 0..TOP), only from J->E / J->GE / J->envstack[0..envtop), or only from J->R or J->G | [x] `api_state.rs` |
| 195 | js_freestate | every gc list freed unconditionally so all finalizers run, interned strings released, then the value stack and the js_State itself released through J->alloc(actx, ptr, 0) | [x] `api_state.rs` |
| 196 | js_savetry + js_endtry | js_try used at depth 0..JS_TRYLIMIT-1 (each frame snapshots E, envtop, tracetop, top, bot, strict and pc) vs invoked at trytop == JS_TRYLIMIT where js_trystackoverflow pushes a literal and throws to the enclosing frame | [x] `api_state.rs` |
| 197 | js_throw + js_endtry | throw with trytop>0; strict, top, bot, E, envtop and tracetop are all restored from the frame, so a strict-mode change made by the aborted call is rolled back | [x] `api_state.rs` |
| 198 | js_regcomp + js_regexec + js_regfree | cflags=0; eflags=0; pattern `a` (one literal rune); sub=non-NULL; default allocator (the realloc/free wrapper) | [x] `ll_regexp.rs` |
| 199 | js_regcompx + js_regexec + js_regfreex | cflags=0; caller-supplied `alloc(ctx,p,n)` with a non-NULL ctx; pattern `a`; freed via js_regfreex with the same allocator/ctx | [x] `ll_regexp.rs` |
| 200 | js_regcompx | successful compile with errorp=NULL (the trailing `if (errorp) *errorp = NULL` store is skipped) vs errorp=non-NULL (`*errorp` explicitly cleared) | [x] `ll_regexp.rs` |
| 201 | js_regcomp + js_regexec | pattern `""` — strlen*2 == 0 so the Renode parse list is never allocated (g.pstart stays NULL); parsealt() returns NULL; the program is only the prologue split/I_ANYNL/I_JUMP plus I_LPAR/I_RPAR/I_END | [x] `ll_regexp.rs` |
| 202 | js_regcomp + js_regexec | cflags=REG_ICASE; pattern `abc` (ASCII letters) — each I_CHAR.c is stored already canon()-folded at compile time | [x] `ll_regexp.rs` |
| 203 | js_regcomp + js_regexec | cflags=REG_ICASE; pattern containing U+017F (ſ, whose toupperrune is ASCII 'S') — canon() keeps the original rune because `c >= 128 && u < 128` | [x] `ll_regexp.rs` |
| 204 | js_regcomp + js_regexec | cflags=REG_NEWLINE only; pattern `^a$` (no compile-time effect; only I_BOL/I_EOL behaviour changes) | [x] `ll_regexp.rs` |
| 205 | js_regcomp + js_regexec | cflags=REG_ICASE\|REG_NEWLINE; pattern `^Abc$` | [x] `ll_regexp.rs` |
| 206 | js_regcompx | pattern with no character class (`abc`) — g.ncclass==0, prog->cclass left NULL, the class-array allocation is skipped entirely | [x] `ll_regexp.rs` |
| 207 | js_regcompx | pattern with exactly one class (`[a-z]`), and pattern with REG_MAXCLASS (128) distinct classes — Reclass array memcpy'd and every `end` pointer rebased onto the copied `spans` | [x] `ll_regexp.rs` |
| 208 | js_regcomp + js_regexec | nsub extremes: pattern with 0 capture groups (prog->nsub == 1, only sub[0] meaningful) and pattern with 15 groups (prog->nsub == REG_MAXSUB == 16, the largest accepted) | [x] `ll_regexp.rs` |
| 209 | js_regcomp + js_regexec | pattern `a` (single P_CHAR) and `abc` (parsecat's right-leaning P_CAT chain spliced at the tail) | [x] `ll_regexp.rs` |
| 210 | js_regcomp + js_regexec | pattern containing a non-ASCII multi-byte literal rune (e.g. `é`, `字`) — decoded by chartorune in nextrune, compared whole-rune by I_CHAR | [x] `ll_regexp.rs` |
| 211 | js_regcomp + js_regexec | pattern `.` — P_ANY/I_ANY, matches any rune except \n, \r, U+2028, U+2029 | [x] `ll_regexp.rs` |
| 212 | js_regcomp + js_regexec | pattern `^a$` — P_BOL and P_EOL via parserep's pre-atom branches | [x] `ll_regexp.rs` |
| 213 | js_regcomp + js_regexec | pattern `a^b` — P_BOL in a non-initial position (matchable only at sp==bol, or after a newline under REG_NEWLINE) | [x] `ll_regexp.rs` |
| 214 | js_regcomp + js_regexec | pattern `\b` — L_WORD/P_WORD/I_WORD | [x] `ll_regexp.rs` |
| 215 | js_regcomp + js_regexec | pattern `\B` — L_NWORD/P_NWORD/I_NWORD | [x] `ll_regexp.rs` |
| 216 | js_regcomp + js_regexec | pattern `\d` — newcclass + addranges_d, one span '0'-'9', returned as L_CCLASS | [x] `ll_regexp.rs` |
| 217 | js_regcomp + js_regexec | pattern `\D` — addranges_d **then** L_NCCLASS (negated at instruction level; addranges_D is never used at top level) | [x] `ll_regexp.rs` |
| 218 | js_regcomp + js_regexec | pattern `\s` — addranges_s: 0x9-0xD, 0x20, 0xA0, 0x2028-0x2029, 0xFEFF | [x] `ll_regexp.rs` |
| 219 | js_regcomp + js_regexec | pattern `\S` — addranges_s + L_NCCLASS | [x] `ll_regexp.rs` |
| 220 | js_regcomp + js_regexec | pattern `\w` — addranges_w: '0'-'9', 'A'-'Z', '_', 'a'-'z' | [x] `ll_regexp.rs` |
| 221 | js_regcomp + js_regexec | pattern `\W` — addranges_w + L_NCCLASS | [x] `ll_regexp.rs` |
| 222 | js_regcomp + js_regexec | pattern `\0` — lex's `case '0'` sets yychar=0, giving an L_CHAR that matches the NUL rune | [x] `ll_regexp.rs` |
| 223 | js_regcomp + js_regexec | control escapes `\f` `\n` `\r` `\t` `\v` and `\cA` — nextrune returns quoted=0, so they reach lex() as ordinary unquoted L_CHARs (`\cX` yields X & 31) | [x] `ll_regexp.rs` |
| 224 | js_regcomp + js_regexec | two-hex escapes: `\x41` (quoted=1, falls past the shorthand switch to L_CHAR 'A') and `\x00` (value 0 rewritten to the character `'0'` and returned quoted, hitting lex's `case '0'`) | [x] `ll_regexp.rs` |
| 225 | js_regcomp + js_regexec | `\x62` — the hex value is `'b'` returned quoted, so lex's `case 'b'` fires and it compiles to a `\b` word boundary rather than the letter b | [x] `ll_regexp.rs` |
| 226 | js_regcomp + js_regexec | four-hex escapes `\u`+`HHHH`: a normal BMP value (quoted L_CHAR) and the all-zero form, which is rewritten to `'0'` and yields rune 0 | [x] `ll_regexp.rs` |
| 227 | js_regcomp + js_regexec | ESCAPES-table identity escapes `\^ \$ \. \* \+ \? \( \) \[ \] \{ \} \\ \-` and `\` followed by a vertical bar — strchr(ESCAPES) hit, returned quoted as L_CHAR | [x] `ll_regexp.rs` |
| 228 | js_regcomp + js_regexec | identity escape of a non-letter char outside ESCAPES (`\/`, `\,`, `\` + space) — nextrune returns 0, so it becomes an unquoted L_CHAR | [x] `ll_regexp.rs` |
| 229 | js_regcomp + js_regexec | `[abc]` (plain positive class, three singleton spans) and `[^abc]` (leading unquoted `^` ⇒ L_NCCLASS) | [x] `ll_regexp.rs` |
| 230 | js_regcomp + js_regexec | `[a-z]` — one explicit range built from havesave + havedash + a literal endpoint | [x] `ll_regexp.rs` |
| 231 | js_regcomp + js_regexec | dash placement: `[a-]` (loop exits with havesave&&havedash ⇒ addrange(save,save) plus addrange('-','-')) and `[-a]` (leading dash with no pending save ⇒ save='-') | [x] `ll_regexp.rs` |
| 232 | js_regcomp + js_regexec | `[+--]` — dash used as the range endpoint (havesave&&havedash with the current char `-` ⇒ addrange('+','-')) | [x] `ll_regexp.rs` |
| 233 | js_regcomp + js_regexec | `[\d\s]` — quoted shorthand escapes unioned inside a class (pending `save` flushed, then addranges_*) | [x] `ll_regexp.rs` |
| 234 | js_regcomp + js_regexec | `[\D\S\W]` and `[^\d]` — inside a class the negated shorthands go through addranges_D/S/W (explicit complement spans up to 0xFFFF), unlike the top-level `\D` path | [x] `ll_regexp.rs` |
| 235 | js_regcomp + js_regexec | class-internal escapes: `[\b]` (quoted `b` rewritten to backspace 0x08, not a word boundary), `[\0]` (rewritten to the NUL rune), and `[\]\\\-]` (identity escapes making `]`, `\`, `-` literal) | [x] `ll_regexp.rs` |
| 236 | js_regcompx | addrange merge branches 1 and 2: `[a-zc]` (new range completely inside an existing span ⇒ dropped) and `[b-ya-z]` (new range swallows the old span ⇒ overwritten in place) | [x] `ll_regexp.rs` |
| 237 | js_regcompx | addrange merge branches 3 and 4: `[b-za-c]` (extend at start: b >= p[0]-1, b <= p[1], a < p[0]) and `[a-cb-z]` (extend at end: a >= p[0], a <= p[1]+1, b > p[1]) | [x] `ll_regexp.rs` |
| 238 | js_regcompx | class with 32 pairwise-disjoint singleton spans — exactly REG_MAXSPAN/2 spans, the maximum a single Reclass holds | [x] `ll_regexp.rs` |
| 239 | js_regcomp + js_regexec | `a*` — P_REP m=0 n=REPINF greedy ⇒ split / body / jump-back with split->x = body | [x] `ll_regexp.rs` |
| 240 | js_regcomp + js_regexec | `a*?` — same shape with ng=1 ⇒ split->y = body and split->x = exit | [x] `ll_regexp.rs` |
| 241 | js_regcomp + js_regexec | `a+` — m=1 n=REPINF ⇒ one unrolled body copy then a trailing split whose x points back at the last copy | [x] `ll_regexp.rs` |
| 242 | js_regcomp + js_regexec | `a+?` — m=1 n=REPINF with ng=1 ⇒ trailing split with x/y swapped | [x] `ll_regexp.rs` |
| 243 | js_regcomp + js_regexec | `a?` — m=0 n=1 ⇒ a single I_SPLIT over one body copy | [x] `ll_regexp.rs` |
| 244 | js_regcomp + js_regexec | `a??` — m=0 n=1 with ng=1 | [x] `ll_regexp.rs` |
| 245 | js_regcomp + js_regexec | `a{3}` and `a{3}?` — lexcount min==max ⇒ 3 unrolled copies and an immediate `break`, so the `?` is parsed into ng but never read (identical programs) | [x] `ll_regexp.rs` |
| 246 | js_regcomp + js_regexec | `a{2,}` — `,}` sets max=REPINF ⇒ 2 unrolled copies plus a trailing back-split | [x] `ll_regexp.rs` |
| 247 | js_regcomp + js_regexec | `a{2,}?` — non-greedy with m>0 and an unbounded max | [x] `ll_regexp.rs` |
| 248 | js_regcomp + js_regexec | `a{2,5}` — min<max<REPINF ⇒ 2 unrolled copies then 3 split+body pairs | [x] `ll_regexp.rs` |
| 249 | js_regcomp + js_regexec | `a{2,5}?` — bounded non-greedy (each split's x/y swapped) | [x] `ll_regexp.rs` |
| 250 | js_regcomp + js_regexec | `a{0}` (m==n==0 ⇒ the unroll loop runs zero times and nothing is emitted for the atom) and `a{0,3}` (m=0 with a finite max ⇒ only split+body pairs) | [x] `ll_regexp.rs` |
| 251 | js_regcomp | `a{12,34}` (multi-digit lexcount, both accumulation loops) and `a{254}` (254 == REPINF-1, the largest count accepted) | [x] `ll_regexp.rs` |
| 252 | js_regcomp + js_regexec | quantified compound atoms: `(ab)+` (LPAR/RPAR duplicated per unrolled copy), `[a-z]{2,4}`, `.*` | [x] `ll_regexp.rs` |
| 253 | js_regcomp + js_regexec | `(a)` — P_PAR n=1 ⇒ I_LPAR/I_RPAR pair, prog->nsub == 2 | [x] `ll_regexp.rs` |
| 254 | js_regcomp + js_regexec | `(?:a)` — L_NC ⇒ fully transparent: no LPAR/RPAR emitted, nsub unchanged | [x] `ll_regexp.rs` |
| 255 | js_regcomp + js_regexec | `(?=a)` — P_PLA ⇒ I_PLA + body + I_END, with split->y pointing past the body; matched on the caller's Resub so captures inside a *successful* lookahead survive | [x] `ll_regexp.rs` |
| 256 | js_regcomp + js_regexec | `(?!a)` — P_NLA ⇒ I_NLA + body + I_END, matched on a *scratch* Resub copy so captures inside the (failed) body are discarded | [x] `ll_regexp.rs` |
| 257 | js_regcomp + js_regexec | nested capturing `((a)(b))` — nsub=4 with numbers assigned in open-paren order by `g->nsub++` | [x] `ll_regexp.rs` |
| 258 | js_regcomp + js_regexec | mixed nesting `(?:(a)\|(b))` — captures inside a non-capturing group inside an alternation | [x] `ll_regexp.rs` |
| 259 | js_regcomp + js_regexec | `a\|b` — P_ALT ⇒ I_SPLIT + left branch + I_JUMP + right branch | [x] `ll_regexp.rs` |
| 260 | js_regcomp + js_regexec | `a\|b\|c` — the left-leaning P_ALT chain built by the accept('\|') loop | [x] `ll_regexp.rs` |
| 261 | js_regcomp + js_regexec | empty alternatives `a\|`, `\|a`, `\|` — parsecat returns NULL and compile(NULL) emits nothing for that branch | [x] `ll_regexp.rs` |
| 262 | js_regcomp + js_regexec | `(a\|)` — empty alternative nested inside a capture; the group still records sp/ep | [x] `ll_regexp.rs` |
| 263 | js_regcomp + js_regexec | `(a)\1` and a 9-group pattern ending in `\9` — single-digit back-references to already-closed groups (P_REF n, x = g->sub[n]) | [x] `ll_regexp.rs` |
| 264 | js_regcomp + js_regexec | pattern with ≥12 groups ending in `\12` — lex consumes a second decimal digit whenever `*source` is 0-9 | [x] `ll_regexp.rs` |
| 265 | js_regcomp + js_regexec | `(?:(a)\|b)\1` matched against text that takes the `b` branch — sub[1].sp/.ep are both NULL, so I_REF compares 0 bytes and succeeds (back-reference to a non-participating group matches empty) | [x] `ll_regexp.rs` |
| 266 | js_regexec | eflags=0 with `sub`=NULL — the internal `scratch` Resub is used, capture positions are unobservable, only 0/1 is returned | [x] `ll_regexp.rs` |
| 267 | js_regexec | eflags=0 with `sub`=non-NULL — sub->nsub set from prog->nsub and all REG_MAXSUB slots pre-cleared to NULL before matching | [x] `ll_regexp.rs` |
| 268 | js_regexec | eflags=REG_NOTBOL; cflags=0; pattern `^a`; subject "a" — I_BOL rejects even at sp==bol; with subject "x\na" it fails everywhere | [x] `ll_regexp.rs` |
| 269 | js_regexec | eflags=REG_NOTBOL; cflags=REG_NEWLINE; subject "x\na" — I_BOL still succeeds via the `sp > bol && isnewline(sp[-1])` branch | [x] `ll_regexp.rs` |
| 270 | js_regexec | flag supplied only at match time: eflags=REG_ICASE over a pattern compiled with cflags=0 (subject rune canon()'d but pc->c stored unfolded), and eflags=REG_NEWLINE over cflags=0 (multiline without recompiling) — prog->flags \| eflags are merged | [x] `ll_regexp.rs` |
| 271 | js_regexec | pattern `$` with cflags=REG_NEWLINE against a subject containing each of \n (0xA), \r (0xD), U+2028, U+2029; and with cflags=0 where only the NUL terminator satisfies I_EOL | [x] `ll_regexp.rs` |
| 272 | js_regexec | pattern `.` against a subject whose leading rune is a newline — I_ANY refuses it but the prologue I_ANYNL skip loop walks past it | [x] `ll_regexp.rs` |
| 273 | js_regexec | subject "" (empty) — every consuming opcode (I_ANYNL, I_ANY, I_CHAR, I_CCLASS, I_NCCLASS) takes its `!*sp` early return | [x] `ll_regexp.rs` |
| 274 | js_regexec | unanchored search where the match begins at offset > 0 — driven entirely by the split/I_ANYNL/I_JUMP prologue that regcompx emits | [x] `ll_regexp.rs` |
| 275 | js_regexec | `\b` at sp==bol (the `sp > bol` term is 0, so i = 0 ^ iswordchar(sp[0])) and `\b` at end of subject (iswordchar reads the NUL terminator) | [x] `ll_regexp.rs` |
| 276 | js_regexec | `\B` in the middle of a word (both sides word chars ⇒ xor 0) | [x] `ll_regexp.rs` |
| 277 | js_regexec | REG_ICASE with class `[a-z]` and with negated class `[^a-z]` against uppercase subjects — incclasscanon() enumerates and canon()s every rune of every span | [x] `ll_regexp.rs` |
| 278 | js_regexec | no ICASE with a class — plain incclass() span scan (`p[0] <= c && c <= p[1]`) | [x] `ll_regexp.rs` |
| 279 | js_regexec | REG_ICASE with a back-reference — strncmpcanon() rune-by-rune fold compare, including its short-subject (`!*a`) and short-capture (`!*b`) early returns | [x] `ll_regexp.rs` |
| 280 | js_regexec | no ICASE with a back-reference — plain strncmp over exactly `ep - sp` bytes | [x] `ll_regexp.rs` |
| 281 | js_regexec | greedy vs non-greedy made observable through captures: `a.*b` vs `a.*?b` on "axbxb" give different sub[0].ep | [x] `ll_regexp.rs` |
| 282 | js_regexec | one Reprog executed twice with different subjects and different eflags (0 then REG_NOTBOL) — prog is stateless and sub is fully re-initialised on each call | [x] `ll_regexp.rs` |
| 283 | js_regfree / js_regfreex | prog == NULL (the `if (prog)` guard makes it a no-op) | [x] `ll_regexp.rs` |
| 284 | js_regfreex | prog->cclass == NULL (no character class in the pattern) ⇒ only start and prog are handed to alloc(...,0); prog->cclass != NULL ⇒ three frees in cclass/start/prog order, with a custom allocator | [x] `ll_regexp.rs` |
| 285 | js_newregexp | flags=0; pattern `a` ⇒ opts=0 passed to js_regcompx | [x] `js_builtins.rs + gaps.rs` |
| 286 | js_newregexp | flags=JS_REGEXP_G only ⇒ opts still 0 (G never reaches the compiler; it only enables `last` bookkeeping) | [x] `js_builtins.rs + gaps.rs` |
| 287 | js_newregexp | flags=JS_REGEXP_I ⇒ opts=REG_ICASE | [x] `js_builtins.rs + gaps.rs` |
| 288 | js_newregexp | flags=JS_REGEXP_M ⇒ opts=REG_NEWLINE | [x] `js_builtins.rs + gaps.rs` |
| 289 | js_newregexp | flags=JS_REGEXP_G\|JS_REGEXP_I ⇒ opts=REG_ICASE | [x] `js_builtins.rs + gaps.rs` |
| 290 | js_newregexp | flags=JS_REGEXP_G\|JS_REGEXP_M ⇒ opts=REG_NEWLINE | [x] `js_builtins.rs + gaps.rs` |
| 291 | js_newregexp | flags=JS_REGEXP_I\|JS_REGEXP_M ⇒ opts=REG_ICASE\|REG_NEWLINE | [x] `js_builtins.rs + gaps.rs` |
| 292 | js_newregexp | flags=JS_REGEXP_G\|JS_REGEXP_I\|JS_REGEXP_M ⇒ opts=REG_ICASE\|REG_NEWLINE, last initialised to 0 | [x] `js_builtins.rs + gaps.rs` |
| 293 | js_newregexp | pattern containing `/` — escaperegexp() pre-counts the slashes and stores `\/` in obj->u.r.source (is_clone=0) | [x] `js_builtins.rs + gaps.rs` |
| 294 | js_newregexp (via `new RegExp(re)`) | is_clone=1 — source js_strdup'd verbatim with no slash escaping, flags inherited, argument 2 required to be undefined | [x] `js_builtins.rs + gaps.rs` |
| 295 | js_newregexp (via `new RegExp()` / `new RegExp("")`) | undefined or zero-length pattern substituted with `(?:)`; the RegExp_prototype's own `(?:)` program is compiled in jsB_init with cflags=0 and errorp=NULL | [x] `js_builtins.rs + gaps.rs` |
| 296 | js_RegExp_prototype_exec | flags without G, match found ⇒ result array plus `input` and `index` (js_utfptrtoidx of sub[0].sp) plus m.nsub indexed captures; re->last never touched | [x] `js_builtins.rs + gaps.rs` |
| 297 | js_RegExp_prototype_exec | flags without G, no match ⇒ null; re->last never touched | [x] `js_builtins.rs + gaps.rs` |
| 298 | js_RegExp_prototype_exec | G set with re->last == 0 ⇒ haystack = text and opts = 0 | [x] `js_builtins.rs + gaps.rs` |
| 299 | js_RegExp_prototype_exec | G set, re->last > 0, M clear ⇒ haystack = text + last and opts \|= REG_NOTBOL | [x] `js_builtins.rs + gaps.rs` |
| 300 | js_RegExp_prototype_exec | G set, re->last > 0, M set, haystack[-1] == '\n' ⇒ REG_NOTBOL deliberately NOT set | [x] `js_builtins.rs + gaps.rs` |
| 301 | js_RegExp_prototype_exec | G set, re->last > 0, M set, haystack[-1] != '\n' ⇒ REG_NOTBOL set | [x] `js_builtins.rs + gaps.rs` |
| 302 | js_RegExp_prototype_exec | G set with re->last == strlen(text) exactly (the `>` test fails) ⇒ regexec is still run, against the empty tail | [x] `js_builtins.rs + gaps.rs` |
| 303 | js_RegExp_prototype_exec | G set with re->last > strlen(text) ⇒ last reset to 0 and null returned without calling js_regexec at all | [x] `js_builtins.rs + gaps.rs` |
| 304 | js_RegExp_prototype_exec | G set, match found ⇒ re->last = m.sub[0].ep - text (an absolute offset into the original text, not into the shifted haystack) | [x] `js_builtins.rs + gaps.rs` |
| 305 | js_RegExp_prototype_exec | G set, no match ⇒ re->last reset to 0 | [x] `js_builtins.rs + gaps.rs` |
| 306 | js_RegExp_prototype_exec | pattern with a capture group that did not participate ⇒ js_pushlstring(NULL, 0) stored at that index | [x] `js_builtins.rs + gaps.rs` |
| 307 | js_toregexp | idx holds a JS_TOBJECT whose class is JS_CREGEXP ⇒ returns &obj->u.r with prog/source/flags/last all readable | [x] `js_builtins.rs + gaps.rs` |
| 308 | js_fmtexp | e == 0 ⇒ the `while (i < 1)` pad emits a single '0', giving "e+0" | [x] `ll_num.rs` |
| 309 | js_fmtexp | e > 0 single-digit and multi-digit (21, 308) ⇒ '+' then reversed digits; e < 0 (-7, -324) ⇒ '-' then digits of -e | [x] `ll_num.rs` |
| 310 | js_grisu2 | v an exact power of two (1.0, 2.0) ⇒ normalized_boundaries sees significand_is_zero and uses m_minus = (f<<2)-1 at e-2; v with a non-zero significand ⇒ (f<<1)-1 at e-1 | [x] `ll_num.rs` |
| 311 | js_grisu2 | v subnormal (5e-324, Number.MIN_VALUE) ⇒ the double2diy_fp biased_e==0 branch plus normalize_boundary's shift loop | [x] `ll_num.rs` |
| 312 | js_grisu2 | v a small integer in 1..999 ⇒ digit_gen returns from inside the `kappa > 0` integer loop | [x] `ll_num.rs` |
| 313 | js_grisu2 | v requiring 17 significant digits (0.1+0.2) ⇒ digit_gen falls through into the fractional `p2 *= 10` do/while loop | [x] `ll_num.rs` |
| 314 | js_strtod | leading blanks (' ', '\t', '\n', '\r') and a leading '-', a leading '+', or no sign | [x] `ll_num.rs` |
| 315 | js_strtod | integer with no '.' and no exponent (decPt = mantSize, fracExp = 0) and fraction-only ".5" (decPt = 0 with mantSize decremented for the point) | [x] `ll_num.rs` |
| 316 | js_strtod | mantissa of ≤ 9 digits (only the frac2 loop runs) and of 10..18 digits (frac1 and frac2 both used) | [x] `ll_num.rs` |
| 317 | js_strtod | mantissa longer than 18 digits ⇒ truncated to 18 with fracExp = decPt - 18, so the dropped digits are absorbed into the exponent | [x] `ll_num.rs` |
| 318 | js_strtod | mantSize == 0 (no digits at all) ⇒ fraction 0.0 and *endPtr rewound to the original `string` | [x] `ll_num.rs` |
| 319 | js_strtod | 'e'/'E' exponent with '+', with '-', and with no sign; plus an exponent digit run long enough to hit the `exp < INT_MAX/100` guard, after which the second loop consumes digits without accumulating | [x] `ll_num.rs` |
| 320 | js_strtod | combined exp > 511 (maxExponent) ⇒ clamped with expSign=FALSE, errno=ERANGE, overflowing toward inf; combined exp < -511 ⇒ clamped with expSign=TRUE, errno=ERANGE, underflowing toward 0 | [x] `ll_num.rs` |
| 321 | js_strtod | combined exp == 0 (the powersOf10 accumulation loop body never runs, dblExp stays 1.0); endPtr==NULL (the jslex lexnumber/lexjsonnumber call shape) vs endPtr non-NULL (the js_stringtofloat shape) | [x] `ll_num.rs` |
| 322 | js_strtol | base == 10 ⇒ the dedicated fast loop with no table lookup; base == 16 with mixed-case a-f/A-F ⇒ the table path | [x] `ll_num.rs` |
| 323 | js_strtol | base == 2 (smallest radix parseInt and Np_toString permit) and base == 36 ('z' == 35, the largest table value below the 80 sentinel) | [x] `ll_num.rs` |
| 324 | js_strtol | base == 0 ⇒ `table[c] < 0` is never true so nothing is consumed: returns 0 with *p == s; base == 1 ⇒ only '0' is accepted | [x] `ll_num.rs` |
| 325 | js_strtol | p == NULL (end-pointer store skipped); and a first character that is not a digit in the given base ⇒ 0 with *p == s, the "no progress" signal parseInt and js_stringtofloat test for | [x] `ll_num.rs` |
| 326 | js_strtol | digit run long enough that the double accumulator loses precision or reaches inf (there is no overflow check at all) | [x] `ll_num.rs` |
| 327 | js_itoa | v == 0 ⇒ the `i == 0` fallback writes a single '0'; v > 0 multi-digit ⇒ reverse-then-flip buffer | [x] `ll_num.rs` |
| 328 | js_itoa | v < 0 ⇒ '-' then a = -(unsigned)v; v == INT_MIN exercises the unsigned cast that avoids signed-overflow UB; v == INT_MAX | [x] `ll_num.rs` |
| 329 | js_stringtofloat | integer shape with no sign ⇒ isflt=0 ⇒ js_strtol base 10; leading '-' ⇒ -js_strtol(s+1); leading '+' ⇒ js_strtol(s+1) (js_strtol itself never parses a sign) | [x] `ll_num.rs` |
| 330 | js_stringtofloat | contains '.' and/or an 'e'/'E' exponent with optional sign ⇒ isflt=1 ⇒ js_strtod | [x] `ll_num.rs` |
| 331 | js_stringtofloat | the pre-scan end `e` and the parser's `end` disagree ⇒ returns 0 with *ep rewound to `s` | [x] `ll_num.rs` |
| 332 | jsV_stringtonumber | leading and trailing jsY_iswhite / jsY_isnewline runs (0x9, 0xB, 0xC, 0x20, 0xA0, 0xFEFF, 0xA, 0xD) stripped on both sides | [x] `ll_num.rs` |
| 333 | jsV_stringtonumber | "0x…"/"0X…" with s[2] != 0 ⇒ js_strtol(s+2, base 16); exactly "0x" (s[2] == 0) ⇒ hex branch declined, falls through to js_stringtofloat and ends as NaN | [x] `ll_num.rs` |
| 334 | jsV_stringtonumber | "Infinity" (8 chars consumed), "+Infinity" (9), "-Infinity" (9) — strncmp prefix branches yielding ±INFINITY | [x] `ll_num.rs` |
| 335 | jsV_stringtonumber | "" or all-whitespace ⇒ js_stringtofloat finds nothing, e == s and *e == 0 ⇒ result 0, not NaN; trailing non-whitespace garbage ⇒ *e != 0 ⇒ NaN | [x] `ll_num.rs` |
| 336 | jsV_numbertostring | f == 0 ⇒ the literal "0" (buf untouched); -0 takes the same path since signbit is never consulted; NaN ⇒ "NaN"; ±Infinity ⇒ "Infinity"/"-Infinity" (all literals, so jsV_tostring's `p == buf` interning is skipped) | [x] `ll_num.rs` |
| 337 | jsV_numbertostring | f an exactly-representable int in [INT_MIN, INT_MAX] ⇒ the js_itoa fast path, including -2147483648 (INT_MIN) and 2147483647 (INT_MAX) | [x] `ll_num.rs` |
| 338 | jsV_numbertostring | f == 2^31 (2147483648) — just past INT_MAX so the fast path is skipped; grisu2 gives point=10 ⇒ fixed form | [x] `ll_num.rs` |
| 339 | jsV_numbertostring | f == 2^32 (4294967296) and f == 2^32-1 (UINT_MAX) ⇒ grisu2 fixed form, trailing-zero fill vs all-significant digits | [x] `ll_num.rs` |
| 340 | jsV_numbertostring | f == 2^53 (9007199254740992) ⇒ point=16 with ndigits < point, so the `while (point-- > 0)` zero-fill tail runs | [x] `ll_num.rs` |
| 341 | jsV_numbertostring | 0 < point ≤ 21 with ndigits > point ⇒ '.' inserted mid-digit-run by the `--point == 0 && ndigits > 0` test (1.5, 3.14159); point == 21 exactly (1e20) is still fixed form | [x] `ll_num.rs` |
| 342 | jsV_numbertostring | point == 22 (1e21) ⇒ `point > 21` ⇒ exponential notation (the upper switchover boundary) | [x] `ll_num.rs` |
| 343 | jsV_numbertostring | point == 0 (0.5) ⇒ the `point <= 0` branch emits "0." with the zero-pad loop not taken; point == -5 (1e-6) ⇒ still fixed, "0.000001" with 5 pad zeros | [x] `ll_num.rs` |
| 344 | jsV_numbertostring | point == -6 (1e-7) ⇒ `point < -5` ⇒ exponential notation (the lower switchover boundary) | [x] `ll_num.rs` |
| 345 | jsV_numbertostring | exponential form with ndigits == 1 ⇒ no '.' emitted ("1e+21"); with ndigits > 1 ⇒ "d.ddd" followed by js_fmtexp(point-1) | [x] `ll_num.rs` |
| 346 | jsV_numbertostring | negative non-integer ⇒ signbit(f) writes '-' before the digit run (added after the zero/NaN/inf/itoa early exits) | [x] `ll_num.rs` |
| 347 | jsV_numbertostring | subnormal 5e-324 ⇒ exponential form with a 3-digit negative exponent; a 17-significant-digit value (0.30000000000000004) ⇒ ndigits == 17, nearly filling digits[32] | [x] `ll_num.rs` |
| 348 | jsV_numbertointeger | n == 0 ⇒ 0 (tested before isnan) and NaN ⇒ 0 | [x] `ll_num.rs` |
| 349 | jsV_numbertointeger | positive fraction ⇒ floor(n); negative fraction ⇒ -floor(-n), i.e. truncation toward zero rather than floor | [x] `ll_num.rs` |
| 350 | jsV_numbertointeger | n < INT_MIN ⇒ INT_MIN; n > INT_MAX ⇒ INT_MAX; ±Infinity hits the same clamps because isfinite is never tested | [x] `ll_num.rs` |
| 351 | jsV_numbertoint32 | 0, -0, NaN, ±Infinity ⇒ 0 via the `!isfinite(n) \|\| n == 0` guard; n in [0, 2^31) ⇒ fmod then floor, returned unchanged | [x] `ll_num.rs` |
| 352 | jsV_numbertoint32 | n == 2^31 ⇒ `n >= two31` ⇒ n - 2^32 == INT_MIN; n == 2^32 ⇒ fmod yields 0; n == 2^32-1 ⇒ -1 | [x] `ll_num.rs` |
| 353 | jsV_numbertoint32 | n negative and non-integral ⇒ the ceil(n) + 2^32 branch (round toward zero, then wrap into the unsigned window) | [x] `ll_num.rs` |
| 354 | jsV_numbertouint32 | an argument whose int32 result is negative ⇒ reinterpreted unsigned (-1 ⇒ 4294967295); 2^32 ⇒ 0 | [x] `ll_num.rs` |
| 355 | jsV_numbertoint16 / jsV_numbertouint16 | the int32 result narrowed to `short` (65535 ⇒ -1, 32768 ⇒ -32768) and to `unsigned short` (-1 ⇒ 65535, 65536 ⇒ 0) | [x] `ll_num.rs` |
| 356 | jsU_chartorune | leading bytes 0xC0 0x80 ⇒ the overlong-NUL special case, checked before anything else: rune 0 with length 2 | [x] `ll_utf.rs` |
| 357 | jsU_chartorune | single byte 0x00 ⇒ rune 0 length 1; single byte 0x7F (Rune1, last 1-byte value) ⇒ length 1 | [x] `ll_utf.rs` |
| 358 | jsU_chartorune | 0xC2 0x80 ⇒ U+0080, the first 2-byte value (l > Rune1); 0xDF 0xBF ⇒ U+07FF == Rune2, the last | [x] `ll_utf.rs` |
| 359 | jsU_chartorune | overlong 2-byte 0xC0 0x81 and 0xC1 0xBF ⇒ l <= Rune1 ⇒ Runeerror (0xFFFD) with length 1 | [x] `ll_utf.rs` |
| 360 | jsU_chartorune | 0xE0 0xA0 0x80 ⇒ U+0800, the first 3-byte value (l > Rune2); 0xEF 0xBF 0xBF ⇒ U+FFFF == Rune3, the last | [x] `ll_utf.rs` |
| 361 | jsU_chartorune | overlong 3-byte 0xE0 0x80 0x80 ⇒ l <= Rune2 ⇒ Runeerror, length 1 | [x] `ll_utf.rs` |
| 362 | jsU_chartorune | surrogate encodings 0xED 0xA0 0x80 (U+D800) and 0xED 0xBF 0xBF (U+DFFF) ⇒ **accepted** as ordinary 3-byte runes; there is no surrogate rejection | [x] `ll_utf.rs` |
| 363 | jsU_chartorune | 0xF0 0x90 0x80 0x80 ⇒ U+10000, the first 4-byte value (l > Rune3); 0xF4 0x8F 0xBF 0xBF ⇒ U+10FFFF == Runemax, the last accepted | [x] `ll_utf.rs` |
| 364 | jsU_chartorune | 0xF4 0x90 0x80 0x80 (U+110000) ⇒ l > Runemax ⇒ Runeerror; overlong 4-byte 0xF0 0x80 0x80 0x80 ⇒ l <= Rune3 ⇒ Runeerror | [x] `ll_utf.rs` |
| 365 | jsU_chartorune | lead byte ≥ T5 (0xF8..0xFF) ⇒ falls out of the 4-byte block ⇒ Runeerror length 1; a continuation byte first (0x80..0xBF) ⇒ `c < T2` ⇒ Runeerror length 1 | [x] `ll_utf.rs` |
| 366 | jsU_chartorune | truncated sequences 0xC2+NUL, 0xE0 0xA0+NUL, 0xF0 0x90 0x80+NUL ⇒ the `c1/c2/c3 & Testx` checks fire ⇒ Runeerror with length 1, so the scan never runs past the terminator | [x] `ll_utf.rs` |
| 367 | jsU_runetochar + jsU_runelen | rune 0 ⇒ the overlong C0 80 encoding with length 2, so an embedded NUL rune never terminates the buffer | [x] `ll_utf.rs` |
| 368 | jsU_runetochar + jsU_runelen | each width boundary: 0x01 and 0x7F ⇒ 1 byte; 0x80 and 0x7FF ⇒ 2; 0x800 and 0xFFFF ⇒ 3; 0x10000 and 0x10FFFF ⇒ 4 | [x] `ll_utf.rs` |
| 369 | jsU_runetochar | a rune in 0xD800..0xDFFF ⇒ emitted as a plain 3-byte sequence (relied on by js_runeat and Sp_substring_imp surrogate splitting); a rune > Runemax ⇒ silently replaced by Runeerror and emitted in 3 bytes | [x] `ll_utf.rs` |
| 370 | jsU_isalpharune | c inside a ucd_alpha2 range pair, c equal to a ucd_alpha1 singleton, and c matching neither ⇒ 0 (drives regexp isunicodeletter and jsY_isidentifierstart/part) | [x] `ll_utf.rs` |
| 371 | jsU_islowerrune / jsU_isupperrune | islowerrune consults the **ucd_toupper** tables and isupperrune the **ucd_tolower** tables; exercise a range-table hit, a singleton-table hit, and a miss for each | [x] `ll_utf.rs` |
| 372 | jsU_tolowerrune / jsU_toupperrune | c inside a range triple ⇒ c + p[2]; c equal to a singleton-pair key ⇒ c + p[1]; c in neither table ⇒ identity; plus ucd_bsearch's n==1 and `n && c >= t[0]` edges | [x] `ll_utf.rs` |
| 373 | jsU_tolowerrune_full / jsU_toupperrune_full | exact hit in the 4-wide tolower table and the 5-wide toupper table (U+00DF ⇒ "SS", U+FB03 ⇒ 3 runes) returning a pointer to a NUL-terminated rune expansion; miss ⇒ NULL, so Sp_toLowerCase/Sp_toUpperCase fall back to the simple mapping | [x] `ll_utf.rs` |
| 374 | js_utflen | pure-ASCII string; string with BMP multi-byte runes (counted 1 each); string with astral runes ≥ 0x10000 (counted **2** each, i.e. UTF-16 length); empty string ⇒ 0 | [x] `ll_utf.rs` |
| 375 | js_utfptrtoidx | p == s ⇒ 0; p after some ASCII bytes; p after an astral rune ⇒ index advanced by 2 | [x] `ll_utf.rs` |
| 376 | js_runeat | i == 0 on ASCII; i on a BMP multi-byte rune; i past the end or a NUL reached ⇒ EOF; i landing on an astral rune's first half (loop leaves i == -2) ⇒ 0xD800 + high bits; on its second half (i == -1) ⇒ 0xDC00 + low 10 bits | [x] `ll_utf.rs` |
| 377 | jsY_initlex + jsY_lex | filename/source set, line=1, lasttoken=0, one lookahead rune preloaded; source "" ⇒ lexchar=EOF immediately and jsY_lex returns 0; "\r\n" consumed as one unit, bare "\r", bare "\n", U+2028, U+2029 all normalised to '\n' with J->line incremented | [x] `ll_lex.rs` |
| 378 | jsY_iswhite / jsY_isnewline | iswhite accepts exactly 0x9, 0xB, 0xC, 0x20, 0xA0, 0xFEFF; isnewline accepts exactly 0xA, 0xD, 0x2028, 0x2029 (0xA and 0xD are deliberately not iswhite) | [x] `ll_lex.rs` |
| 379 | jsY_ishex / jsY_tohex | '0'-'9' ⇒ 0-9, 'a'-'f' ⇒ 10-15, 'A'-'F' ⇒ 10-15, and a non-hex character ⇒ ishex 0 with tohex silently 0 | [x] `ll_lex.rs` |
| 380 | jsY_findword + jsY_tokenstring | findword hitting the midpoint, the left half, the right half, the first and the last element, plus a miss ⇒ -1; tokenstring for token 0, a single-char token, TK_IDENTIFIER/NUMBER/STRING/REGEXP, a multi-char operator, a keyword, and an index in the NULL filler band ⇒ "<unknown>" | [x] `ll_lex.rs` |
| 381 | jsY_lex (lexnumber) | plain decimal integer `123`, bare `0`, `123.456`, trailing-dot `1.` | [x] `ll_lex.rs` |
| 382 | jsY_lex (lexnumber + lexhex) | `0x1f` and `0X1F` ⇒ lexhex accumulates and returns immediately, skipping the '.', exponent and letter-suffix logic entirely | [x] `ll_lex.rs` |
| 383 | jsY_lex (lexnumber) | leading `0` followed by `.` (`0.5`) — the shape adjacent to the rejected octal-looking `0NNN`; and `.5` reached through the `case '.'` dispatch | [x] `ll_lex.rs` |
| 384 | jsY_lex (lexnumber) | `.` not followed by a digit ⇒ lexnumber returns the '.' token instead of a number | [x] `ll_lex.rs` |
| 385 | jsY_lex (lexnumber) | exponent forms `1e5`, `1E5`, `1e+5`, `1e-5`, `1.5e3`, `.5e-2`; the whole span is then re-parsed by js_strtod starting from `J->source - 1` | [x] `ll_lex.rs` |
| 386 | jsY_lex (lexstring) | single-quoted `'…'` vs double-quoted `"…"` — the terminator is whichever quote opened it, so the other quote is an ordinary character; and `\` + newline as a line continuation (lexescape returns 0 and pushes nothing) | [x] `ll_lex.rs` |
| 387 | jsY_lex (lexescape) | `\u`+`HHHH` (four hex digits accumulated then pushed as a rune), `\x`+`HH` (two hex digits), and `\0` (pushes rune 0, which runetochar then encodes as the 2-byte overlong form) | [x] `ll_lex.rs` |
| 388 | jsY_lex (lexescape) | the named escapes `\\` `\'` `\"` `\b` `\f` `\n` `\r` `\t` `\v`, and the `default:` arm where any other escaped character (`\q`, `\/`, `\1`, a non-ASCII rune) is pushed verbatim | [x] `ll_lex.rs` |
| 389 | jsY_lex | regexp/division ambiguity: `/` when lasttoken is `]`, `)`, `}`, TK_IDENTIFIER, TK_NUMBER, TK_STRING, TK_FALSE, TK_NULL, TK_THIS or TK_TRUE ⇒ isregexpcontext 0 ⇒ division ('/' or TK_DIV_ASS for `/=`) | [x] `ll_lex.rs` |
| 390 | jsY_lex (lexregexp) | `/` in any other context ⇒ regexp literal (at statement start, after `=`, after `(`, after an operator) | [x] `ll_lex.rs` |
| 391 | jsY_lex (lexlinecomment / lexcomment) | `//` to end-of-line or EOF; `/* … */`, the empty `/**/`, and `/***/` or `/*a**/` which exercise the inner `while (lexchar == '*')` skip | [x] `ll_lex.rs` |
| 392 | jsY_lex (lexregexp) | body containing `\/` ⇒ backslash dropped and a bare '/' pushed; body containing any other `\X` ⇒ both backslash and character preserved for regcomp; body containing `[/]` ⇒ `inclass` suppresses the '/' terminator until the ']' | [x] `ll_lex.rs` |
| 393 | jsY_lex (lexregexp) | flag suffix absent, `g`, `i`, `m`, and multi-flag orders `gi`, `im`, `gim`, `mig` ⇒ J->number carries the JS_REGEXP_G/I/M bits | [x] `ll_lex.rs` |
| 394 | jsY_lex | newline bookkeeping: J->newline set on any line break, and when lasttoken is TK_BREAK/TK_CONTINUE/TK_RETURN/TK_THROW the newline is returned as a virtual ';' (isnlthcontext) | [x] `ll_lex.rs` |
| 395 | jsY_lex (jsY_unescape) | identifier shapes: plain `abc`, `$`, `_`, a leading non-ASCII isalpharune, digits/`$`/`_` in the tail, and an identifier written with a `\u`+`HHHH` escape at the start and in the tail | [x] `ll_lex.rs` |
| 396 | jsY_lex (jsY_findkeyword + jsY_findword) | each of the 29 keywords (break case catch continue debugger default delete do else false finally for function if in instanceof new null return switch this throw true try typeof var void while with) ⇒ TK_BREAK + index; a non-keyword ⇒ TK_IDENTIFIER with J->text pointing at the lexbuf | [x] `ll_lex.rs` |
| 397 | jsY_lex | every multi-char operator: `<=` `>=` `==` `!=` `===` `!==` `<<` `>>` `>>>` `&&` `\|\|` `+=` `-=` `*=` `/=` `%=` `<<=` `>>=` `>>>=` `&=` `\|=` `^=` `++` `--` — each reachable only after its shorter prefix fails to accept | [x] `ll_lex.rs` |
| 398 | jsY_lex | every single-char token: `( ) , : ; ? [ ] { } ~ < > = ! + - * % & \| ^ /` plus '.' returned from lexnumber, and EOF ⇒ 0 | [x] `ll_lex.rs` |
| 399 | jsY_lexjson (lexjsonnumber) | `0`, `-0`, a `1`-`9` lead with more digits, fraction `1.5`, exponents `1e5` / `1E+5` / `1e-5`, and combined `-1.5e-3` ⇒ js_strtod over the whole span | [x] `ll_lex.rs` |
| 400 | jsY_lexjson (lexjsonstring) | the eight JSON escapes `\"` `\\` `\/` `\b` `\f` `\n` `\r` `\t` plus `\u`+`HHHH` — and the deliberate absence of `\'`, `\0`, `\x`+`HH` and line continuations that jsY_lex accepts | [x] `ll_lex.rs` |
| 401 | jsY_lexjson | literals `true`, `false`, `null` matched by fixed jsY_expect chains rather than the keyword table | [x] `ll_lex.rs` |
| 402 | jsY_lexjson | structural tokens `,` `:` `[` `]` `{` `}` and EOF ⇒ 0; whitespace skipping accepts jsY_iswhite plus '\n' (hence also \r, U+2028, U+2029 after jsY_next normalisation) | [x] `ll_lex.rs` |
| 403 | jsY_lexjson vs jsY_lex | the same source lexed in both modes: JSON mode has no comments, no regexp literals, no single-quoted strings, no identifiers or keywords, no `0x` hex and no leading-'.' numbers | [x] `ll_lex.rs` |
| 404 | JSON.parse (jsonvalue) | reviver argument absent or not callable ⇒ jsonvalue only, result returned directly; reviver callable ⇒ a wrapper object is built, the value stored under the "" key, then jsonrevive("") walks and rewrites it | [x] `js_builtins.rs` |
| 405 | JSON.parse (jsonrevive) | holder an array ⇒ children visited by js_itoa index; holder a plain object ⇒ children visited via js_pushiterator/js_nextiterator; a reviver returning undefined deletes the member, otherwise the returned value is stored back | [x] `js_builtins.rs` |
| 406 | JSON.parse | scalar top-level values: a string, a number, `true`, `false`, `null`; plus `{}` and `[]` via the early jsonaccept('}')/jsonaccept(']') returns | [x] `js_builtins.rs` |
| 407 | JSON.parse | object with one key and with several comma-separated keys; array with one and with several elements; objects nested in arrays nested in objects (recursive jsonvalue) | [x] `js_builtins.rs` |
| 408 | JSON.stringify | replacer absent (slot 2 undefined) ⇒ filterprop returns 1 for every key and no replacer call is made | [x] `js_builtins.rs` |
| 409 | JSON.stringify | replacer callable ⇒ invoked as replacer.call(holder, key, value) for every value including the "" root | [x] `js_builtins.rs` |
| 410 | JSON.stringify | replacer an array ⇒ filterprop allowlist: entries that are strings, numbers, String objects or Number objects are compared and others ignored; object keys absent from the list are dropped while array elements are unaffected | [x] `js_builtins.rs` |
| 411 | JSON.stringify | space absent ⇒ gap NULL ⇒ single-line output with no space after ':' | [x] `js_builtins.rs` |
| 412 | JSON.stringify | space a number: 0 ⇒ gap stays NULL; 1..10 ⇒ that many spaces; > 10 ⇒ clamped to 10; negative ⇒ clamped to 0 ⇒ gap NULL; also accepted as a Number wrapper object | [x] `js_builtins.rs` |
| 413 | JSON.stringify | space a string: "" ⇒ gap NULL; shorter than 10 ⇒ used verbatim; longer than 10 ⇒ truncated to the first 10 characters; also accepted as a String wrapper object | [x] `js_builtins.rs` |
| 414 | JSON.stringify (fmtnum) | NaN ⇒ "null"; ±Infinity ⇒ "null"; 0 and -0 ⇒ "0"; any other finite number ⇒ jsV_numbertostring | [x] `js_builtins.rs` |
| 415 | JSON.stringify (fmtstr) | the named escapes for `"`, `\`, \b, \f, \n, \r, \t; a control character < 0x20 ⇒ `\u00XX`; a lone surrogate in 0xD800..0xDFFF ⇒ `\uXXXX`; c < 128 ⇒ raw byte; c ≥ 128 ⇒ the original UTF-8 bytes copied through unescaped | [x] `js_builtins.rs` |
| 416 | JSON.stringify (fmtvalue) | value a boolean ⇒ "true"/"false"; null ⇒ "null"; a string; a finite number; and value undefined or callable ⇒ fmtvalue returns 0, so at the root the whole call yields undefined, inside an array it becomes "null", and inside an object the property is rewound out of the buffer | [x] `js_builtins.rs` |
| 417 | JSON.stringify (fmtvalue) | value a Number / String / Boolean wrapper object ⇒ unwrapped from obj->u; an Array object ⇒ fmtarray; any other non-callable object ⇒ fmtobject | [x] `js_builtins.rs` |
| 418 | JSON.stringify (fmtvalue) | value with a callable `toJSON` (e.g. a Date) ⇒ called with the key and its result serialised; value with a non-callable `toJSON` property ⇒ ignored | [x] `js_builtins.rs` |
| 419 | JSON.stringify (fmtindent) | gap set with a non-empty array/object ⇒ '\n' plus `level` copies of gap before each member and before the closer; gap set but the array/object empty ⇒ the `gap && n` guard suppresses the closing indent; deep nesting increments `level` per descent | [x] `js_builtins.rs` |
| 420 | Sp_split | separator argument undefined ⇒ neither split helper runs; a one-element array holding the whole string is built | [x] `js_builtins.rs` |
| 421 | Sp_split + Sp_split_string | string separator with limit absent (1<<30), limit 0 (empty array returned immediately), and limit smaller than the number of pieces | [x] `js_builtins.rs` |
| 422 | Sp_split + Sp_split_string | separator "" (strlen 0) ⇒ the per-rune chartorune loop instead of strstr; separator present in the subject (repeated strstr hits) vs absent (whole remainder pushed and str set to NULL) | [x] `js_builtins.rs` |
| 423 | Sp_split + Sp_split_regexp | regexp separator with limit absent, limit 0, and limit reached mid-loop (each of the three `len == limit` early returns) | [x] `js_builtins.rs` |
| 424 | Sp_split + Sp_split_regexp | empty subject (e == text): a pattern that matches ⇒ empty array; a pattern that does not match ⇒ [""] | [x] `js_builtins.rs` |
| 425 | Sp_split + Sp_split_regexp | separator regexp with capture groups ⇒ sub[1..nsub-1] interleaved into the result after each piece | [x] `js_builtins.rs` |
| 426 | Sp_split + Sp_split_regexp | zero-width match at the end of the previous match (b == c && b == p) ⇒ advance one rune and retry instead of emitting a piece | [x] `js_builtins.rs` |
| 427 | Sp_split + Sp_split_regexp | continuation searches pass REG_NOTBOL unless isbol() holds, and isbol() honours JS_REGEXP_M by treating a position just after '\n' as a bol | [x] `js_builtins.rs` |
| 428 | Sp_replace + Sp_replace_string | search value a plain string, replacement a string containing `$$`, `$&`, `` $` ``, `$'`, a trailing lone `$`, and `$X` for some other X — one row per switch arm reached | [x] `js_builtins.rs` |
| 429 | Sp_replace + Sp_replace_string | search value a plain string, replacement callable ⇒ called with exactly (match, offset, subject) | [x] `js_builtins.rs` |
| 430 | Sp_replace + Sp_replace_string | search string not found ⇒ argument 0 returned untouched with no buffer allocated | [x] `js_builtins.rs` |
| 431 | Sp_replace + Sp_replace_regexp | regexp search value, replacement a string using `$1`..`$9`, a two-digit `$NN` (the second digit consumed only when 0-9), `$0`, and an index ≥ m.nsub ⇒ out-of-range forms echoed literally by re-splitting `$NN` back into digits | [x] `js_builtins.rs` |
| 432 | Sp_replace + Sp_replace_regexp | regexp search value, replacement callable ⇒ called with the match, then one argument per participating capture (`while (m.sub[x].sp)`), then the offset, then the subject | [x] `js_builtins.rs` |
| 433 | Sp_replace + Sp_replace_regexp | regexp without JS_REGEXP_G ⇒ exactly one replacement then the tail appended | [x] `js_builtins.rs` |
| 434 | Sp_replace + Sp_replace_regexp | regexp with JS_REGEXP_G ⇒ the replacement loop; a zero-length match copies one byte forward (or ends the loop at the terminator); continuation searches use isbol()/REG_NOTBOL; re->last forced to 0 | [x] `js_builtins.rs` |
| 435 | Sp_replace + Sp_replace_regexp | regexp that matches nothing ⇒ argument 0 returned unchanged before any buffer work | [x] `js_builtins.rs` |
| 436 | Sp_match + js_newregexp | argument 1 already a regexp (copied), undefined (compiled from the empty pattern with flags 0), or any other value (compiled from its ToString with flags 0) | [x] `js_builtins.rs` |
| 437 | Sp_match | regexp without JS_REGEXP_G ⇒ delegates straight to js_RegExp_prototype_exec; with G ⇒ re->last zeroed and every match collected, empty matches advancing one rune, an empty result ⇒ null | [x] `js_builtins.rs` |
| 438 | Ap_sort | comparator argument undefined ⇒ elements compared with strcmp of their ToString forms | [x] `js_builtins.rs` |
| 439 | Ap_sort | comparator callable and returning a negative value, 0, a positive value, and NaN (NaN and 0 are both treated as "equal") | [x] `js_builtins.rs` |
| 440 | Ap_sort | array is `simple` and idx_b < flat_length ⇒ Ap_sort_cmp/Ap_sort_swap take the direct js_Value fast path with no property lookups | [x] `js_builtins.rs` |
| 441 | Ap_sort | array non-simple, or an index beyond flat_length ⇒ the generic js_hasindex/js_setindex/js_delindex path, including holes (all four has_a/has_b combinations) | [x] `js_builtins.rs` |
| 442 | Ap_sort | array containing undefined elements ⇒ sorted after all defined elements (the und_a/und_b branches exist in both the fast and generic paths) | [x] `js_builtins.rs` |
| 443 | Ap_sort | length 0 or 1 ⇒ the array is returned immediately without validating the comparator or heapsorting; length 2 ⇒ the smallest input that actually runs Ap_sort_heapsort/leaf/sift | [x] `js_builtins.rs` |
| 444 | jsB_new_Date | 0 arguments (top == 1) ⇒ t = Now(); 1 argument whose ToPrimitive(HNONE) is a string ⇒ parseDateTime | [x] `js_builtins.rs` |
| 445 | jsB_new_Date | 1 argument whose ToPrimitive(HNONE) is a number ⇒ TimeClip(ToNumber), covering 0, ±8.64e15 (the clip boundary) and a non-finite value ⇒ NaN | [x] `js_builtins.rs` |
| 446 | jsB_new_Date | 2..7 arguments ⇒ (y, m) required with d=1 and H=M=S=ms=0 supplied by js_optnumber; y < 100 ⇒ +1900; the result passed through UTC(), i.e. interpreted as local time | [x] `js_builtins.rs` |
| 447 | D_UTC | 2..7 arguments with the same defaulting and y<100 adjustment but **no** UTC() correction, since the arguments are already UTC | [x] `js_builtins.rs` |
| 448 | D_parse + parseDateTime | `YYYY`, `YYYY-MM`, `YYYY-MM-DD`, and the optional time sections `THH:mm`, `THH:mm:ss`, `THH:mm:ss.sss` (toint called with widths 4, 2 and 3) | [x] `js_builtins.rs` |
| 449 | D_parse + parseDateTime | timezone suffix `Z` ⇒ tza 0; `+HH`, `+HH:mm`, `-HH:mm` ⇒ signed offset; suffix omitted ⇒ tza = LocalTZA(); accepted field extremes m 1..12, d 1..31, H 0..24 (24 only with M==S==ms==0), M/S 0..59, ms 0..999, tzh ≤ 23, tzm ≤ 59 | [x] `js_builtins.rs` |
| 450 | fmtdatetime / fmtdate / fmttime (Dp_toString, Dp_toISOString, Dp_toUTCString, Dp_toDateString, Dp_toTimeString, jsB_Date) | tza == 0 ⇒ 'Z' suffix; tza < 0 ⇒ `-HH:MM`; tza > 0 ⇒ `+HH:MM`; non-finite t ⇒ the literal "Invalid Date" returned without writing buf | [x] `js_builtins.rs` |
