//! Phase C — the allocator-injection rows of ERRORS.md.
//!
//! Every `!ptr` / `hashtable_init(...) != 0` / `strbuffer_init(...) != 0` branch
//! in the C is only reachable when an allocation fails. `json_set_alloc_funcs*`
//! is a per-library global, so each `.so` gets its own *budgeted* allocator:
//! the first `n` allocations succeed and every later one returns NULL.
//!
//! For each operation the budget is swept from 0 upwards, so EVERY allocation
//! site inside that operation is made to fail in turn. C and Rust must agree at
//! every budget — which additionally proves they perform the same number of
//! allocations, itself observable to any C consumer with a constrained
//! allocator.
//!
//! These tests mutate process-global state in both libraries and therefore must
//! run single-threaded (`run_tests.sh` passes `--test-threads=1`).

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---------------------------------------------------------------------------
// Two independent budgeted allocators, one per library.
// ---------------------------------------------------------------------------

/// Remaining successful allocations; `isize::MAX` means "unlimited".
static C_BUDGET: AtomicIsize = AtomicIsize::new(isize::MAX);
static R_BUDGET: AtomicIsize = AtomicIsize::new(isize::MAX);
/// Number of allocation requests actually issued (for reporting).
static C_CALLS: AtomicUsize = AtomicUsize::new(0);
static R_CALLS: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn c_malloc(n: usize) -> *mut c_void {
    C_CALLS.fetch_add(1, Ordering::SeqCst);
    if C_BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        std::ptr::null_mut()
    } else {
        malloc(n)
    }
}
unsafe extern "C" fn r_malloc(n: usize) -> *mut c_void {
    R_CALLS.fetch_add(1, Ordering::SeqCst);
    if R_BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        std::ptr::null_mut()
    } else {
        malloc(n)
    }
}
unsafe extern "C" fn c_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    C_CALLS.fetch_add(1, Ordering::SeqCst);
    if C_BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        std::ptr::null_mut()
    } else {
        realloc(p, n)
    }
}
unsafe extern "C" fn r_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    R_CALLS.fetch_add(1, Ordering::SeqCst);
    if R_BUDGET.fetch_sub(1, Ordering::SeqCst) <= 0 {
        std::ptr::null_mut()
    } else {
        realloc(p, n)
    }
}
unsafe extern "C" fn any_free(p: *mut c_void) {
    free(p)
}

/// Install the budgeted allocators. `with_realloc == false` uses the 2-arg
/// `json_set_alloc_funcs`, which NULLs `do_realloc` and thereby forces the
/// malloc+memcpy+free realloc *emulation* path in `jsonp_realloc` (ERRORS F4,
/// F5, F11) — a completely different code path from the 3-arg form.
fn install(with_realloc: bool) {
    let p = pair();
    unsafe {
        if with_realloc {
            p.c.json_set_alloc_funcs2(Some(c_malloc), Some(c_realloc), Some(any_free));
            p.r.json_set_alloc_funcs2(Some(r_malloc), Some(r_realloc), Some(any_free));
        } else {
            p.c.json_set_alloc_funcs(Some(c_malloc), Some(any_free));
            p.r.json_set_alloc_funcs(Some(r_malloc), Some(any_free));
        }
    }
}

/// Restore the stock allocators.
fn restore() {
    let p = pair();
    unsafe {
        p.c.json_set_alloc_funcs2(Some(malloc_shim), Some(realloc_shim), Some(any_free));
        p.r.json_set_alloc_funcs2(Some(malloc_shim), Some(realloc_shim), Some(any_free));
    }
    C_BUDGET.store(isize::MAX, Ordering::SeqCst);
    R_BUDGET.store(isize::MAX, Ordering::SeqCst);
}

unsafe extern "C" fn malloc_shim(n: usize) -> *mut c_void {
    malloc(n)
}
unsafe extern "C" fn realloc_shim(p: *mut c_void, n: usize) -> *mut c_void {
    realloc(p, n)
}

fn set_budget(n: isize) {
    C_BUDGET.store(n, Ordering::SeqCst);
    R_BUDGET.store(n, Ordering::SeqCst);
    C_CALLS.store(0, Ordering::SeqCst);
    R_CALLS.store(0, Ordering::SeqCst);
}

fn unlimited() {
    C_BUDGET.store(isize::MAX, Ordering::SeqCst);
    R_BUDGET.store(isize::MAX, Ordering::SeqCst);
}

/// Run `op` under every allocation budget in `0..=max_budget`, on both libraries,
/// and require the reported outcomes to be identical.
///
/// `op` receives the library and must return a short, comparable description of
/// the outcome (return codes, NULL-ness, rendered value, error struct …). It
/// must NOT allocate through the library after it has finished measuring.
fn sweep_budgets<F>(name: &str, max_budget: isize, with_realloc: bool, mut op: F)
where
    F: FnMut(&Lib, bool) -> String,
{
    let p = pair();
    install(with_realloc);
    for budget in 0..=max_budget {
        set_budget(budget);
        let cout = op(&p.c, true);
        set_budget(budget);
        let rout = op(&p.r, false);
        unlimited();
        assert_eq!(
            cout, rout,
            "{name}: budget {budget} (realloc={with_realloc}) diverged\n  C   = {cout}\n  RUST= {rout}"
        );
    }
    restore();
}

/// Render a value for outcome comparison, using an allocation-free path where
/// possible. `json_dumps` itself allocates, so it is called with the budget
/// already lifted.
unsafe fn describe(l: &Lib, j: json_ptr) -> String {
    if j.is_null() {
        return "NULL".to_string();
    }
    unlimited();
    let d = l.dumps(j, JSON_ENCODE_ANY | JSON_SORT_KEYS);
    format!("type={} dump={}", typeof_json(j), show(&d))
}

fn esnap(e: &json_error_t) -> String {
    let s = e.snap();
    format!(
        "line={} col={} pos={} src={:?} code={} text={:?}",
        s.line,
        s.column,
        s.position,
        String::from_utf8_lossy(&s.source),
        s.code,
        String::from_utf8_lossy(&s.text)
    )
}

// ===========================================================================
// A1, A2, B1, B2, C3, C21, C25, H5 — constructors
// ===========================================================================

#[test]
fn oom_constructors() {
    for with_realloc in [true, false] {
        // A1/A2 + H5: json_object() allocates the object AND the bucket array.
        sweep_budgets("json_object", 4, with_realloc, |l, _| unsafe {
            let j = l.json_object();
            let out = describe(l, j);
            l.json_decref(j);
            out
        });
        // B1/B2: json_array() allocates the array AND the 8-slot table.
        sweep_budgets("json_array", 4, with_realloc, |l, _| unsafe {
            let j = l.json_array();
            let out = describe(l, j);
            l.json_decref(j);
            out
        });
        // C2/C3: json_stringn allocates the strndup buffer AND the json_string_t.
        for payload in ["", "a", "hello world"] {
            let b = payload.as_bytes();
            sweep_budgets("json_stringn", 4, with_realloc, |l, _| unsafe {
                let j = l.json_stringn(b.as_ptr() as *const c_char, b.len());
                let out = describe(l, j);
                l.json_decref(j);
                out
            });
            sweep_budgets("json_stringn_nocheck", 4, with_realloc, |l, _| unsafe {
                let j = l.json_stringn_nocheck(b.as_ptr() as *const c_char, b.len());
                let out = describe(l, j);
                l.json_decref(j);
                out
            });
        }
        // C21: json_integer
        sweep_budgets("json_integer", 3, with_realloc, |l, _| unsafe {
            let j = l.json_integer(42);
            let out = describe(l, j);
            l.json_decref(j);
            out
        });
        // C25: json_real
        sweep_budgets("json_real", 3, with_realloc, |l, _| unsafe {
            let j = l.json_real(1.5);
            let out = describe(l, j);
            l.json_decref(j);
            out
        });
        // The immortal singletons never allocate.
        sweep_budgets("json_true/false/null", 2, with_realloc, |l, _| unsafe {
            format!(
                "{} {} {}",
                l.json_true().is_null(),
                l.json_false().is_null(),
                l.json_null().is_null()
            )
        });
    }
}

/// F2, F5, F6, F7: the raw allocator wrappers under failure.
#[test]
fn oom_raw_allocator_wrappers() {
    for with_realloc in [true, false] {
        sweep_budgets("jsonp_malloc", 3, with_realloc, |l, _| unsafe {
            let a = l.jsonp_malloc(16);
            let b = l.jsonp_malloc(32);
            let out = format!("{} {}", a.is_null(), b.is_null());
            l.jsonp_free(a);
            l.jsonp_free(b);
            out
        });
        // F5/F6: realloc failure. `jsonp_realloc` must NOT free the original in
        // the emulation path.
        sweep_budgets("jsonp_realloc", 4, with_realloc, |l, _| unsafe {
            unlimited();
            let p0 = l.jsonp_malloc(16);
            std::ptr::write_bytes(p0 as *mut u8, 0xA5, 16);
            set_budget(0);
            let p1 = l.jsonp_realloc(p0, 16, 64);
            let out = format!("realloc-null={}", p1.is_null());
            unlimited();
            l.jsonp_free(if p1.is_null() { p0 } else { p1 });
            out
        });
        // F7: jsonp_strndup
        sweep_budgets("jsonp_strndup", 3, with_realloc, |l, _| unsafe {
            let src = b"abcdef\0";
            let a = l.jsonp_strndup(src.as_ptr() as *const c_char, 6);
            let out = format!("null={} val={:?}", a.is_null(), cstr_bytes(a));
            l.jsonp_free(a as *mut c_void);
            out
        });
        // G1: strbuffer_init
        sweep_budgets("strbuffer_init", 3, with_realloc, |l, _| unsafe {
            let mut sb = strbuffer_t::zeroed();
            let r = l.strbuffer_init(&mut sb as *mut _ as *mut c_void);
            let out = format!("ret={r} value-null={}", sb.value.is_null());
            unlimited();
            if r == 0 {
                l.strbuffer_close(&mut sb as *mut _ as *mut c_void);
            }
            out
        });
        // G5: strbuffer_append_bytes realloc failure at each growth step
        sweep_budgets("strbuffer_grow", 6, with_realloc, |l, _| unsafe {
            unlimited();
            let mut sb = strbuffer_t::zeroed();
            assert_eq!(l.strbuffer_init(&mut sb as *mut _ as *mut c_void), 0);
            set_budget(0);
            let mut rets = String::new();
            for i in 0..6 {
                let data = vec![b'x'; 8 << i];
                let r = l.strbuffer_append_bytes(
                    &mut sb as *mut _ as *mut c_void,
                    data.as_ptr() as *const c_char,
                    data.len(),
                );
                rets.push_str(&format!("{r},"));
            }
            let out = format!("rets={rets} len={} size={}", sb.length, sb.size);
            unlimited();
            l.strbuffer_close(&mut sb as *mut _ as *mut c_void);
            out
        });
        // H5: hashtable_init
        sweep_budgets("hashtable_init", 3, with_realloc, |l, _| unsafe {
            let mut slab = [0u8; 256];
            let h = slab.as_mut_ptr() as *mut c_void;
            let r = l.hashtable_init(h);
            let out = format!("ret={r}");
            unlimited();
            if r == 0 {
                l.hashtable_close(h);
            }
            out
        });
    }
}

// ===========================================================================
// A10, A18, H4, H7, H8, H9 — object insertion and rehash under OOM
// ===========================================================================

#[test]
fn oom_object_insert_and_rehash() {
    for with_realloc in [true, false] {
        // A10 / H7 / H9: the pair allocation fails.
        sweep_budgets("object_set", 8, with_realloc, |l, _| unsafe {
            unlimited();
            let o = l.json_object();
            set_budget(0);
            let mut rets = String::new();
            for i in 0..4usize {
                let k = format!("k{i}");
                let v = l.json_integer(i as i64);
                let r = l.json_object_setn_new_nocheck(
                    o,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    v,
                );
                rets.push_str(&format!("{r},"));
            }
            let out = format!("rets={rets} size={} {}", l.json_object_size(o), describe(l, o));
            unlimited();
            l.json_decref(o);
            out
        });
        // H4 / H8: the rehash allocation fails when inserting the 9th key.
        sweep_budgets("object_rehash", 6, with_realloc, |l, _| unsafe {
            unlimited();
            let o = l.json_object();
            for i in 0..8usize {
                let k = format!("k{i:02}");
                l.json_object_setn_new_nocheck(
                    o,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    l.json_integer(i as i64),
                );
            }
            assert_eq!(l.json_object_size(o), 8);
            set_budget(0);
            let mut rets = String::new();
            for i in 8..14usize {
                let k = format!("k{i:02}");
                unlimited();
                let v = l.json_integer(i as i64);
                set_budget(C_BUDGET.load(Ordering::SeqCst).min(0));
                let r = l.json_object_setn_new_nocheck(
                    o,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    v,
                );
                rets.push_str(&format!("{r},"));
            }
            let out = format!("rets={rets} size={} {}", l.json_object_size(o), describe(l, o));
            unlimited();
            l.json_decref(o);
            out
        });
        // A18: json_object_update failing part-way leaves a PARTIAL update.
        sweep_budgets("object_update", 10, with_realloc, |l, _| unsafe {
            unlimited();
            let a = l.json_object();
            let b = l.json_object();
            for i in 0..4usize {
                let k = format!("b{i}");
                l.json_object_setn_new_nocheck(
                    b,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    l.json_integer(i as i64),
                );
            }
            set_budget(0);
            let r = l.json_object_update(a, b);
            let out = format!("ret={r} target={}", describe(l, a));
            unlimited();
            l.json_decref(a);
            l.json_decref(b);
            out
        });
        // A24, A25: update_recursive (hashtable_init for `parents`, plus inserts)
        sweep_budgets("object_update_recursive", 12, with_realloc, |l, _| unsafe {
            unlimited();
            let a = l.json_object();
            let b = l.json_object();
            let inner = l.json_object();
            let ki = cs("inner");
            let kx = cs("x");
            l.json_object_set_new(inner, kx.as_ptr(), l.json_integer(1));
            l.json_object_set_new(b, ki.as_ptr(), inner);
            let ka = cs("a");
            l.json_object_set_new(b, ka.as_ptr(), l.json_integer(2));
            let inner2 = l.json_object();
            l.json_object_set_new(a, ki.as_ptr(), inner2);
            set_budget(0);
            let r = l.json_object_update_recursive(a, b);
            let out = format!("ret={r} target={}", describe(l, a));
            unlimited();
            l.json_decref(a);
            l.json_decref(b);
            out
        });
        for which in 0..2 {
            sweep_budgets("object_update_existing/missing", 8, with_realloc, move |l, _| unsafe {
                unlimited();
                let a = l.json_object();
                let b = l.json_object();
                let k1 = cs("shared");
                let k2 = cs("only-b");
                l.json_object_set_new(a, k1.as_ptr(), l.json_integer(0));
                l.json_object_set_new(b, k1.as_ptr(), l.json_integer(1));
                l.json_object_set_new(b, k2.as_ptr(), l.json_integer(2));
                set_budget(0);
                let r = if which == 0 {
                    l.json_object_update_existing(a, b)
                } else {
                    l.json_object_update_missing(a, b)
                };
                let out = format!("ret={r} target={}", describe(l, a));
                unlimited();
                l.json_decref(a);
                l.json_decref(b);
                out
            });
        }
        // C13: json_string_setn_nocheck's strndup fails.
        sweep_budgets("string_setn", 4, with_realloc, |l, _| unsafe {
            unlimited();
            let s = l.json_string(cs("original").as_ptr());
            set_budget(0);
            let v = b"replacement";
            let r = l.json_string_setn_nocheck(s, v.as_ptr() as *const c_char, v.len());
            let out = format!("ret={r} {}", describe(l, s));
            unlimited();
            l.json_decref(s);
            out
        });
    }
}

// ===========================================================================
// B9, B12, B16, B21 — array growth under OOM
// ===========================================================================

#[test]
fn oom_array_growth() {
    for with_realloc in [true, false] {
        // B12: append across the 8 -> 16 growth boundary.
        sweep_budgets("array_append_grow", 6, with_realloc, |l, _| unsafe {
            unlimited();
            let a = l.json_array();
            for i in 0..8i64 {
                l.json_array_append_new(a, l.json_integer(i));
            }
            set_budget(0);
            let mut rets = String::new();
            for i in 8..12i64 {
                unlimited();
                let v = l.json_integer(i);
                set_budget(0);
                let r = l.json_array_append_new(a, v);
                rets.push_str(&format!("{r},"));
            }
            let out = format!("rets={rets} size={} {}", l.json_array_size(a), describe(l, a));
            unlimited();
            l.json_decref(a);
            out
        });
        // B16: insert across the growth boundary.
        sweep_budgets("array_insert_grow", 6, with_realloc, |l, _| unsafe {
            unlimited();
            let a = l.json_array();
            for i in 0..8i64 {
                l.json_array_append_new(a, l.json_integer(i));
            }
            let v = l.json_integer(99);
            set_budget(0);
            let r = l.json_array_insert_new(a, 0, v);
            let out = format!("ret={r} size={} {}", l.json_array_size(a), describe(l, a));
            unlimited();
            l.json_decref(a);
            out
        });
        // B21: extend, which grows straight to size + amount.
        sweep_budgets("array_extend_grow", 6, with_realloc, |l, _| unsafe {
            unlimited();
            let a = l.json_array();
            let b = l.json_array();
            for i in 0..4i64 {
                l.json_array_append_new(a, l.json_integer(i));
            }
            for i in 0..40i64 {
                l.json_array_append_new(b, l.json_integer(100 + i));
            }
            set_budget(0);
            let r = l.json_array_extend(a, b);
            let out = format!("ret={r} size={} {}", l.json_array_size(a), describe(l, a));
            unlimited();
            l.json_decref(a);
            l.json_decref(b);
            out
        });
    }
}

// ===========================================================================
// A38, A40, A41, A43, B24, B26, B27, C36 — copying under OOM
// ===========================================================================

#[test]
fn oom_copying() {
    let shapes: Vec<Node> = vec![
        Node::Obj(vec![]),
        Node::Arr(vec![]),
        Node::Obj(vec![(b"a".to_vec(), Node::Int(1))]),
        Node::Arr(vec![Node::Int(1), Node::Int(2)]),
        Node::Obj(vec![
            (b"a".to_vec(), Node::Arr(vec![Node::Int(1), Node::Str(b"s".to_vec())])),
            (b"b".to_vec(), Node::Obj(vec![(b"c".to_vec(), Node::Real(1.5))])),
        ]),
        Node::Str(b"str".to_vec()),
        Node::Int(1),
        Node::Real(1.5),
        Node::Null,
    ];
    for with_realloc in [true, false] {
        for shape in &shapes {
            let sh = shape.clone();
            sweep_budgets("json_copy", 8, with_realloc, move |l, _| unsafe {
                unlimited();
                let src = sh.build(l);
                set_budget(0);
                let cp = l.json_copy(src);
                let out = format!("{}", describe(l, cp));
                unlimited();
                l.json_decref(cp);
                l.json_decref(src);
                out
            });
            let sh = shape.clone();
            sweep_budgets("json_deep_copy", 16, with_realloc, move |l, _| unsafe {
                unlimited();
                let src = sh.build(l);
                set_budget(0);
                let cp = l.json_deep_copy(src);
                let out = format!("{}", describe(l, cp));
                unlimited();
                l.json_decref(cp);
                l.json_decref(src);
                out
            });
        }
    }
}

// ===========================================================================
// D1, D26, D36, D37, D44 — encoding under OOM
// ===========================================================================

#[test]
fn oom_encoding() {
    let nodes: Vec<Node> = vec![
        Node::Arr(vec![]),
        Node::Arr(vec![Node::Int(1), Node::Int(2), Node::Int(3)]),
        Node::Obj(vec![
            (b"alpha".to_vec(), Node::Str(b"one".to_vec())),
            (b"beta".to_vec(), Node::Arr(vec![Node::Int(1), Node::Real(2.5)])),
            (b"gamma".to_vec(), Node::Obj(vec![(b"x".to_vec(), Node::Null)])),
        ]),
        // Long enough to force several strbuffer growths.
        Node::Arr((0..40).map(|i| Node::Str(format!("value{i:03}").into_bytes())).collect()),
    ];
    for with_realloc in [true, false] {
        for node in &nodes {
            for flags in [
                0usize,
                JSON_COMPACT,
                JSON_INDENT(4),
                JSON_SORT_KEYS, // D26: the qsort key array is a separate malloc
                JSON_ENSURE_ASCII,
                JSON_ENCODE_ANY,
            ] {
                let nd = node.clone();
                // D36/D37/D1: json_dumps — strbuffer_init, every growth, and the
                // final shrink-to-fit realloc.
                sweep_budgets("json_dumps", 14, with_realloc, move |l, _| unsafe {
                    unlimited();
                    let j = nd.build(l);
                    set_budget(0);
                    let raw = l.json_dumps_raw(j, flags);
                    let out = if raw.is_null() {
                        "NULL".to_string()
                    } else {
                        format!("{:?}", cstr_bytes(raw))
                    };
                    unlimited();
                    if !raw.is_null() {
                        l.jsonp_free(raw as *mut c_void);
                    }
                    l.json_decref(j);
                    out
                });
                let nd = node.clone();
                // D44 + D26: json_dumpb / json_dump_callback only need the
                // `parents_set` hashtable and (for SORT_KEYS) the key array.
                sweep_budgets("json_dumpb", 10, with_realloc, move |l, _| unsafe {
                    unlimited();
                    let j = nd.build(l);
                    let mut buf = vec![0x5Au8; 4096];
                    set_budget(0);
                    let n = l.json_dumpb(j, buf.as_mut_ptr() as *mut c_char, buf.len(), flags);
                    let out = format!(
                        "n={n} buf={:?}",
                        String::from_utf8_lossy(&buf[..n.min(buf.len())])
                    );
                    unlimited();
                    l.json_decref(j);
                    out
                });
            }
        }
    }
}

// ===========================================================================
// L19, L38, M1, M8, M10, M12, M18, M23, M25 — decoding under OOM
// ===========================================================================

#[test]
fn oom_decoding() {
    let docs: Vec<&str> = vec![
        "[]",
        "{}",
        "[1,2,3]",
        "{\"a\":1,\"b\":2}",
        "[\"str\",1.5,true,null]",
        // deep enough to allocate several nested containers
        "{\"a\":[{\"b\":[1,2,{\"c\":\"d\"}]}]}",
        // long strings force strbuffer growth in the lexer
        "[\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"]",
        // escapes force the second decoding pass
        "[\"\\u00e9\\u20ac\\ud83d\\ude00\"]",
        // enough keys to force a rehash
        "{\"k0\":0,\"k1\":1,\"k2\":2,\"k3\":3,\"k4\":4,\"k5\":5,\"k6\":6,\"k7\":7,\"k8\":8,\"k9\":9}",
        // malformed, so the error path allocates too
        "[1,",
        "{\"a\"",
        "[\"\\ud800\"]",
    ];
    for with_realloc in [true, false] {
        for doc in &docs {
            for flags in [0usize, JSON_DECODE_ANY, JSON_REJECT_DUPLICATES, JSON_ALLOW_NUL] {
                let d = doc.to_string();
                // M23/L38: lex_init's strbuffer_init, plus every later allocation.
                sweep_budgets("json_loadb", 20, with_realloc, move |l, _| unsafe {
                    let b = d.as_bytes();
                    let mut e = json_error_t::default();
                    let j = l.json_loadb(b.as_ptr() as *const c_char, b.len(), flags, &mut e);
                    let out = format!("{} err={}", describe(l, j), esnap(&e));
                    unlimited();
                    l.json_decref(j);
                    out
                });
                let d = doc.to_string();
                sweep_budgets("json_loads", 20, with_realloc, move |l, _| unsafe {
                    let z = raw_z(d.as_bytes());
                    let mut e = json_error_t::default();
                    let j = l.json_loads(z.as_ptr() as *const c_char, flags, &mut e);
                    let out = format!("{} err={}", describe(l, j), esnap(&e));
                    unlimited();
                    l.json_decref(j);
                    out
                });
            }
        }
    }
}

/// M27, M29, M34: the stream decoders' `lex_init` failure, and
/// `json_load_callback`'s buffer.
#[test]
fn oom_stream_decoders() {
    let doc = b"{\"a\":[1,2,3],\"b\":\"x\"}";
    let path = {
        let mut d = std::env::temp_dir();
        d.push(format!("jansson_oom_{}", std::process::id()));
        d
    };
    std::fs::write(&path, doc).unwrap();
    let ps = cs(path.to_str().unwrap());

    for with_realloc in [true, false] {
        // M34: json_load_callback
        let d = doc.to_vec();
        sweep_budgets("json_load_callback", 20, with_realloc, move |l, _| unsafe {
            let mut f = Feeder {
                data: d.clone(),
                pos: 0,
            };
            let mut e = json_error_t::default();
            let j = l.json_load_callback(
                Some(feed_cb),
                &mut f as *mut _ as *mut c_void,
                0,
                &mut e,
            );
            let out = format!("{} err={}", describe(l, j), esnap(&e));
            unlimited();
            l.json_decref(j);
            out
        });
        // M27: json_loadf
        let psc = ps.clone();
        sweep_budgets("json_loadf", 20, with_realloc, move |l, _| unsafe {
            unlimited();
            let m = cs("rb");
            let fp = fopen(psc.as_ptr(), m.as_ptr());
            set_budget(0);
            let mut e = json_error_t::default();
            let j = l.json_loadf(fp, 0, &mut e);
            let out = format!("{} err={}", describe(l, j), esnap(&e));
            unlimited();
            fclose(fp);
            l.json_decref(j);
            out
        });
        // M29: json_loadfd
        let psc = path.clone();
        sweep_budgets("json_loadfd", 20, with_realloc, move |l, _| unsafe {
            use std::os::unix::io::AsRawFd;
            unlimited();
            let file = std::fs::File::open(&psc).unwrap();
            set_budget(0);
            let mut e = json_error_t::default();
            let j = l.json_loadfd(file.as_raw_fd(), 0, &mut e);
            let out = format!("{} err={}", describe(l, j), esnap(&e));
            unlimited();
            l.json_decref(j);
            out
        });
        // json_load_file (fopen succeeds, then lex_init / parse allocations fail)
        let psc = ps.clone();
        sweep_budgets("json_load_file", 20, with_realloc, move |l, _| unsafe {
            let mut e = json_error_t::default();
            let j = l.json_load_file(psc.as_ptr(), 0, &mut e);
            let out = format!("{} err={}", describe(l, j), esnap(&e));
            unlimited();
            l.json_decref(j);
            out
        });
    }
    let _ = std::fs::remove_file(&path);
}

struct Feeder {
    data: Vec<u8>,
    pos: usize,
}
unsafe extern "C" fn feed_cb(buffer: *mut c_void, buflen: usize, data: *mut c_void) -> usize {
    let f = &mut *(data as *mut Feeder);
    let n = (f.data.len() - f.pos).min(buflen);
    if n > 0 {
        std::ptr::copy_nonoverlapping(f.data[f.pos..].as_ptr(), buffer as *mut u8, n);
        f.pos += n;
    }
    n
}
extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut c_void;
    fn fclose(f: *mut c_void) -> c_int;
}

// ===========================================================================
// C17, N5, N7, N12, N15, N19, N20, O1, O9 — sprintf / pack / unpack under OOM
// ===========================================================================

#[test]
fn oom_sprintf() {
    for with_realloc in [true, false] {
        // C17: the jsonp_malloc(length + 1) for the formatted buffer, plus the
        // string object itself.
        sweep_budgets("json_sprintf", 6, with_realloc, |l, _| unsafe {
            let sp = l.json_sprintf_sym();
            let f = cs("hello %s %d");
            let a = cs("world");
            let j = sp(f.as_ptr(), a.as_ptr(), 42i32);
            let out = describe(l, j);
            unlimited();
            l.json_decref(j);
            out
        });
        sweep_budgets("json_sprintf_empty", 4, with_realloc, |l, _| unsafe {
            let sp = l.json_sprintf_sym();
            let f = cs("");
            let j = sp(f.as_ptr());
            let out = describe(l, j);
            unlimited();
            l.json_decref(j);
            out
        });
    }
}

#[test]
fn oom_pack() {
    // Formats chosen so that every allocating pack helper is exercised:
    // containers, plain strings, `s#`/`s%`, `s+` (which uses a strbuffer),
    // integers and reals.
    for with_realloc in [true, false] {
        for fmt in [
            "[]", "{}", "n", "i", "I", "f", "b", "s", "[i,i,i]", "{s:i,s:s}",
            "s#", "s%", "s+", "s++",
            "[{s:[i,i]},{s:s}]",
        ] {
            let fs = fmt.to_string();
            sweep_budgets("json_pack_ex", 20, with_realloc, move |l, _| unsafe {
                let pk = l.json_pack_ex_sym();
                let f = cs(&fs);
                let k1 = cs("k1");
                let k2 = cs("k2");
                let sv = cs("value");
                let mut e = json_error_t::default();
                // One generous, correctly-typed vararg list per format.
                let j = match fs.as_str() {
                    "[]" | "{}" | "n" => pk(&mut e, 0, f.as_ptr()),
                    "i" | "b" => pk(&mut e, 0, f.as_ptr(), 7i32),
                    "I" => pk(&mut e, 0, f.as_ptr(), 7i64),
                    "f" => pk(&mut e, 0, f.as_ptr(), 1.5f64),
                    "s" => pk(&mut e, 0, f.as_ptr(), sv.as_ptr()),
                    "[i,i,i]" => pk(&mut e, 0, f.as_ptr(), 1i32, 2i32, 3i32),
                    "{s:i,s:s}" => {
                        pk(&mut e, 0, f.as_ptr(), k1.as_ptr(), 1i32, k2.as_ptr(), sv.as_ptr())
                    }
                    "s#" => pk(&mut e, 0, f.as_ptr(), sv.as_ptr(), 3i32),
                    "s%" => pk(&mut e, 0, f.as_ptr(), sv.as_ptr(), 3usize),
                    "s+" => pk(&mut e, 0, f.as_ptr(), sv.as_ptr(), sv.as_ptr()),
                    "s++" => pk(&mut e, 0, f.as_ptr(), sv.as_ptr(), sv.as_ptr(), sv.as_ptr()),
                    _ => pk(
                        &mut e, 0, f.as_ptr(), k1.as_ptr(), 1i32, 2i32, k2.as_ptr(),
                        sv.as_ptr(),
                    ),
                };
                let out = format!("{} err={}", describe(l, j), esnap(&e));
                unlimited();
                l.json_decref(j);
                out
            });
        }
    }
}

#[test]
fn oom_unpack() {
    for with_realloc in [true, false] {
        // O1: unpack_object's `hashtable_init(&key_set)`.
        // O9: the leftover-key strbuffer in strict mode.
        for (shape, fmt, flags) in [
            (
                Node::Obj(vec![(b"a".to_vec(), Node::Int(1))]),
                "{s:i}",
                0usize,
            ),
            (
                Node::Obj(vec![
                    (b"a".to_vec(), Node::Int(1)),
                    (b"b".to_vec(), Node::Int(2)),
                    (b"c".to_vec(), Node::Int(3)),
                ]),
                "{s:i}",
                JSON_STRICT,
            ),
            (
                Node::Obj(vec![
                    (b"a".to_vec(), Node::Obj(vec![(b"x".to_vec(), Node::Int(1))])),
                ]),
                "{s:{s:i}}",
                JSON_STRICT,
            ),
            (
                Node::Arr(vec![Node::Int(1), Node::Int(2)]),
                "[i]",
                JSON_STRICT,
            ),
        ] {
            let sh = shape.clone();
            let fs = fmt.to_string();
            sweep_budgets("json_unpack_ex", 12, with_realloc, move |l, _| unsafe {
                unlimited();
                let root = sh.build(l);
                let f = cs(&fs);
                let ka = cs("a");
                let kx = cs("x");
                let mut v1: i64 = -1;
                let mut e = json_error_t::default();
                let un = l.json_unpack_ex_sym();
                set_budget(0);
                let r = if fs == "{s:{s:i}}" {
                    un(root, &mut e, flags, f.as_ptr(), ka.as_ptr(), kx.as_ptr(), &mut v1 as *mut i64)
                } else if fs.starts_with('[') {
                    un(root, &mut e, flags, f.as_ptr(), &mut v1 as *mut i64)
                } else {
                    un(root, &mut e, flags, f.as_ptr(), ka.as_ptr(), &mut v1 as *mut i64)
                };
                let out = format!("ret={r} v1={v1} err={}", esnap(&e));
                unlimited();
                l.json_decref(root);
                out
            });
        }
    }
}

// ===========================================================================
// The allocation COUNT itself must match, which is a stronger statement than
// "both eventually fail".
// ===========================================================================

#[test]
fn oom_allocation_counts_match() {
    let p = pair();
    install(true);
    let ops: Vec<(&str, Box<dyn Fn(&Lib)>)> = vec![
        ("json_object", Box::new(|l: &Lib| unsafe {
            let j = l.json_object();
            l.json_decref(j);
        })),
        ("json_array", Box::new(|l: &Lib| unsafe {
            let j = l.json_array();
            l.json_decref(j);
        })),
        ("json_integer", Box::new(|l: &Lib| unsafe {
            let j = l.json_integer(1);
            l.json_decref(j);
        })),
        ("json_real", Box::new(|l: &Lib| unsafe {
            let j = l.json_real(1.0);
            l.json_decref(j);
        })),
        ("json_string", Box::new(|l: &Lib| unsafe {
            let j = l.json_string(cs("hello").as_ptr());
            l.json_decref(j);
        })),
        ("loadb_small", Box::new(|l: &Lib| unsafe {
            let d = b"[1,2,3]";
            let mut e = json_error_t::default();
            let j = l.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e);
            l.json_decref(j);
        })),
        ("loadb_object", Box::new(|l: &Lib| unsafe {
            let d = b"{\"a\":1,\"b\":[2,3],\"c\":\"str\"}";
            let mut e = json_error_t::default();
            let j = l.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e);
            l.json_decref(j);
        })),
        ("dumps_object", Box::new(|l: &Lib| unsafe {
            let d = b"{\"a\":1,\"b\":[2,3],\"c\":\"str\"}";
            let mut e = json_error_t::default();
            let j = l.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e);
            C_CALLS.store(0, Ordering::SeqCst);
            R_CALLS.store(0, Ordering::SeqCst);
            let s = l.json_dumps_raw(j, JSON_SORT_KEYS);
            l.jsonp_free(s as *mut c_void);
            l.json_decref(j);
        })),
        ("deep_copy", Box::new(|l: &Lib| unsafe {
            let d = b"{\"a\":1,\"b\":[2,3],\"c\":\"str\"}";
            let mut e = json_error_t::default();
            let j = l.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e);
            C_CALLS.store(0, Ordering::SeqCst);
            R_CALLS.store(0, Ordering::SeqCst);
            let cp = l.json_deep_copy(j);
            l.json_decref(cp);
            l.json_decref(j);
        })),
    ];
    for (name, op) in ops {
        unlimited();
        C_CALLS.store(0, Ordering::SeqCst);
        op(&p.c);
        let cn = C_CALLS.load(Ordering::SeqCst);
        R_CALLS.store(0, Ordering::SeqCst);
        op(&p.r);
        let rn = R_CALLS.load(Ordering::SeqCst);
        assert_eq!(
            cn, rn,
            "{name}: allocation count differs (C {cn}, RUST {rn}) — a C consumer \
             with a budgeted allocator would observe different behaviour"
        );
    }
    restore();
}

// ===========================================================================
// Meta-test: prove the injection harness is NOT vacuous — at budget 0 the
// allocating entry points really do fail, and the failure point moves as the
// budget rises. Without this, every test above could be passing trivially.
// ===========================================================================

#[test]
fn zz_injection_harness_is_effective() {
    let p = pair();
    install(true);

    // Budget 0: nothing may be allocated.
    for (name, lib) in [("C", &p.c), ("RUST", &p.r)] {
        set_budget(0);
        unsafe {
            assert!(
                lib.json_object().is_null(),
                "{name}: json_object() must fail at budget 0"
            );
            set_budget(0);
            assert!(
                lib.json_array().is_null(),
                "{name}: json_array() must fail at budget 0"
            );
            set_budget(0);
            assert!(
                lib.json_integer(1).is_null(),
                "{name}: json_integer() must fail at budget 0"
            );
            set_budget(0);
            assert!(
                lib.json_string(cs("x").as_ptr()).is_null(),
                "{name}: json_string() must fail at budget 0"
            );
            set_budget(0);
            let mut e = json_error_t::default();
            let d = b"[1]";
            assert!(
                lib.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e).is_null(),
                "{name}: json_loadb() must fail at budget 0"
            );
        }
    }

    // json_object() needs exactly 2 allocations (the struct + the bucket array),
    // so budget 1 must still fail and budget 2 must succeed — in BOTH libraries.
    for (name, lib) in [("C", &p.c), ("RUST", &p.r)] {
        unsafe {
            set_budget(1);
            assert!(
                lib.json_object().is_null(),
                "{name}: json_object() must still fail at budget 1"
            );
            set_budget(2);
            let ok = lib.json_object();
            assert!(
                !ok.is_null(),
                "{name}: json_object() must succeed at budget 2"
            );
            unlimited();
            lib.json_decref(ok);
        }
    }

    // The failure point must move with the budget for a multi-allocation
    // operation, and move identically in both libraries.
    let mut c_first_ok = None;
    let mut r_first_ok = None;
    for budget in 0..40isize {
        unsafe {
            set_budget(budget);
            let mut e = json_error_t::default();
            let d = b"{\"a\":1,\"b\":[2,3]}";
            let cj = p.c.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e);
            let c_ok = !cj.is_null();
            unlimited();
            p.c.json_decref(cj);

            set_budget(budget);
            let mut e = json_error_t::default();
            let rj = p.r.json_loadb(d.as_ptr() as *const c_char, d.len(), 0, &mut e);
            let r_ok = !rj.is_null();
            unlimited();
            p.r.json_decref(rj);

            if c_ok && c_first_ok.is_none() {
                c_first_ok = Some(budget);
            }
            if r_ok && r_first_ok.is_none() {
                r_first_ok = Some(budget);
            }
        }
    }
    assert!(
        c_first_ok.is_some(),
        "the harness never let json_loadb succeed — budget range too small"
    );
    assert!(
        c_first_ok.unwrap() > 0,
        "json_loadb succeeded at budget 0, so the harness is not injecting failures"
    );
    assert_eq!(
        c_first_ok, r_first_ok,
        "C and Rust need a different number of allocations to parse the same \
         document (C first succeeds at budget {c_first_ok:?}, Rust at {r_first_ok:?})"
    );

    restore();

    // And after restore(), everything works again.
    unsafe {
        let j = p.c.json_object();
        assert!(!j.is_null(), "restore() did not reinstate a working allocator");
        p.c.json_decref(j);
        let j = p.r.json_object();
        assert!(!j.is_null());
        p.r.json_decref(j);
    }
}
