//! Allocation-failure paths.
//!
//! Installing custom hooks has two effects: allocations can be made to fail at
//! an arbitrary point (exercising every `goto fail` / early-return cleanup
//! path), and `global_hooks.reallocate` becomes NULL, which switches `ensure()`
//! and `print()` onto their manual-copy code paths.
//!
//! Hooks are process-global inside each library, so this test binary contains a
//! single serialized test.
mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static FAIL_AT: AtomicUsize = AtomicUsize::new(usize::MAX);

unsafe extern "C" fn counting_malloc(n: usize) -> *mut c_void {
    let i = COUNTER.fetch_add(1, Ordering::SeqCst);
    if i == FAIL_AT.load(Ordering::SeqCst) {
        return std::ptr::null_mut();
    }
    unsafe { libc_malloc(n) }
}

unsafe extern "C" fn counting_free(p: *mut c_void) {
    unsafe { libc_free(p) }
}

fn install(api: &Api, fail_at: usize) {
    let mut hooks = cJSON_Hooks {
        malloc_fn: Some(counting_malloc),
        free_fn: Some(counting_free),
    };
    COUNTER.store(0, Ordering::SeqCst);
    FAIL_AT.store(fail_at, Ordering::SeqCst);
    unsafe { api.cJSON_InitHooks(&mut hooks) };
}

fn restore(api: &Api) {
    FAIL_AT.store(usize::MAX, Ordering::SeqCst);
    unsafe { api.cJSON_InitHooks(std::ptr::null_mut()) };
}

/// How many allocations does the whole operation need when nothing fails?
fn alloc_count(api: &Api, op: impl Fn(&Api)) -> usize {
    install(api, usize::MAX);
    op(api);
    let n = COUNTER.load(Ordering::SeqCst);
    restore(api);
    n
}

const DOCS: &[&str] = &[
    "null",
    "1",
    "1.5",
    "\"str\"",
    "\"esc\\n\\u0041\"",
    "[]",
    "[1,2,3]",
    "{}",
    "{\"a\":1}",
    "{\"a\":[1,2],\"b\":{\"c\":\"d\"}}",
    "[[[[1]]]]",
    "[\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\"]",
];

/// Summary of an operation's observable result.
#[derive(PartialEq, Debug)]
enum Res {
    Null,
    Tree(String),
    Text(Vec<u8>),
    Bool(c_int),
}

#[test]
fn allocation_failures_parse() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            let s = CString::new(*doc).unwrap();
            let total = alloc_count(&a.c, |api| {
                let t = api.cJSON_Parse(s.as_ptr());
                api.cJSON_Delete(t);
            });
            for fail_at in 0..=total {
                let mut results = Vec::new();
                for api in [&a.c, &a.rust] {
                    install(api, fail_at);
                    let t = api.cJSON_Parse(s.as_ptr());
                    let res = if t.is_null() {
                        Res::Null
                    } else {
                        Res::Tree(dump(t))
                    };
                    api.cJSON_Delete(t);
                    restore(api);
                    results.push(res);
                }
                assert_eq!(
                    results[0], results[1],
                    "Parse({doc:?}) with allocation {fail_at}/{total} failing"
                );
            }
        }
    }
}

#[test]
fn allocation_failures_print() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            let s = CString::new(*doc).unwrap();
            // build the trees with the default allocator
            let ct = a.c.cJSON_Parse(s.as_ptr());
            let rt = a.rust.cJSON_Parse(s.as_ptr());
            assert!(!ct.is_null() && !rt.is_null());

            for fmt in [0, 1] {
                let total = alloc_count(&a.c, |api| {
                    let p = if fmt == 1 {
                        api.cJSON_Print(ct)
                    } else {
                        api.cJSON_PrintUnformatted(ct)
                    };
                    api.cJSON_free(p as *mut c_void);
                });
                for fail_at in 0..=total {
                    let mut results = Vec::new();
                    for (api, tree) in [(&a.c, ct), (&a.rust, rt)] {
                        install(api, fail_at);
                        let p = if fmt == 1 {
                            api.cJSON_Print(tree)
                        } else {
                            api.cJSON_PrintUnformatted(tree)
                        };
                        let res = match cstr_bytes(p) {
                            None => Res::Null,
                            Some(b) => Res::Text(b),
                        };
                        api.cJSON_free(p as *mut c_void);
                        restore(api);
                        results.push(res);
                    }
                    assert_eq!(
                        results[0], results[1],
                        "Print({doc:?}, fmt={fmt}) with allocation {fail_at}/{total} failing"
                    );
                }

                // PrintBuffered with a small prebuffer forces growth
                for pre in [0, 1, 4, 64] {
                    let total = alloc_count(&a.c, |api| {
                        let p = api.cJSON_PrintBuffered(ct, pre, fmt);
                        api.cJSON_free(p as *mut c_void);
                    });
                    for fail_at in 0..=total {
                        let mut results = Vec::new();
                        for (api, tree) in [(&a.c, ct), (&a.rust, rt)] {
                            install(api, fail_at);
                            let p = api.cJSON_PrintBuffered(tree, pre, fmt);
                            let res = match cstr_bytes(p) {
                                None => Res::Null,
                                Some(b) => Res::Text(b),
                            };
                            api.cJSON_free(p as *mut c_void);
                            restore(api);
                            results.push(res);
                        }
                        assert_eq!(
                            results[0], results[1],
                            "PrintBuffered({doc:?}, pre={pre}, fmt={fmt}) with allocation {fail_at}/{total} failing"
                        );
                    }
                }
            }
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);
        }
    }
}

#[test]
fn allocation_failures_duplicate_and_create() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            let s = CString::new(*doc).unwrap();
            let ct = a.c.cJSON_Parse(s.as_ptr());
            let rt = a.rust.cJSON_Parse(s.as_ptr());
            for recurse in [0, 1] {
                let total = alloc_count(&a.c, |api| {
                    let d = api.cJSON_Duplicate(ct, recurse);
                    api.cJSON_Delete(d);
                });
                for fail_at in 0..=total {
                    let mut results = Vec::new();
                    for (api, tree) in [(&a.c, ct), (&a.rust, rt)] {
                        install(api, fail_at);
                        let d = api.cJSON_Duplicate(tree, recurse);
                        let res = if d.is_null() {
                            Res::Null
                        } else {
                            Res::Tree(dump(d))
                        };
                        api.cJSON_Delete(d);
                        restore(api);
                        results.push(res);
                    }
                    assert_eq!(
                        results[0], results[1],
                        "Duplicate({doc:?},{recurse}) with allocation {fail_at}/{total} failing"
                    );
                }
            }
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);
        }

        // constructors and the Add*ToObject helpers
        let text = CString::new("some string value").unwrap();
        let key = CString::new("key").unwrap();
        type Op = (&'static str, fn(&Api, *const c_char, *const c_char) -> Res);
        let ops: Vec<Op> = vec![
            ("CreateString", |api, _k, v| {
                let p = api.cJSON_CreateString(v);
                let r = if p.is_null() {
                    Res::Null
                } else {
                    Res::Tree(dump(p))
                };
                api.cJSON_Delete(p);
                r
            }),
            ("CreateRaw", |api, _k, v| {
                let p = api.cJSON_CreateRaw(v);
                let r = if p.is_null() {
                    Res::Null
                } else {
                    Res::Tree(dump(p))
                };
                api.cJSON_Delete(p);
                r
            }),
            ("CreateStringReference", |api, _k, v| {
                let p = api.cJSON_CreateStringReference(v);
                let r = if p.is_null() {
                    Res::Null
                } else {
                    Res::Tree(dump(p))
                };
                api.cJSON_Delete(p);
                r
            }),
            ("AddStringToObject", |api, k, v| {
                let o = api.cJSON_CreateObject();
                let p = api.cJSON_AddStringToObject(o, k, v);
                let r = if o.is_null() {
                    Res::Null
                } else if p.is_null() {
                    Res::Tree(format!("item=null obj={}", dump(o)))
                } else {
                    Res::Tree(dump(o))
                };
                api.cJSON_Delete(o);
                r
            }),
            ("AddNumberToObject", |api, k, _v| {
                let o = api.cJSON_CreateObject();
                let p = api.cJSON_AddNumberToObject(o, k, 1.5);
                let r = if o.is_null() {
                    Res::Null
                } else if p.is_null() {
                    Res::Tree(format!("item=null obj={}", dump(o)))
                } else {
                    Res::Tree(dump(o))
                };
                api.cJSON_Delete(o);
                r
            }),
            ("AddObjectToObject", |api, k, _v| {
                let o = api.cJSON_CreateObject();
                let p = api.cJSON_AddObjectToObject(o, k);
                let r = if o.is_null() {
                    Res::Null
                } else if p.is_null() {
                    Res::Tree(format!("item=null obj={}", dump(o)))
                } else {
                    Res::Tree(dump(o))
                };
                api.cJSON_Delete(o);
                r
            }),
            ("AddItemToObject", |api, k, _v| {
                let o = api.cJSON_CreateObject();
                let it = api.cJSON_CreateTrue();
                let ok = api.cJSON_AddItemToObject(o, k, it);
                let r = if o.is_null() {
                    Res::Null
                } else {
                    Res::Tree(format!("ok={ok} obj={}", dump(o)))
                };
                api.cJSON_Delete(o);
                if ok == 0 {
                    api.cJSON_Delete(it);
                }
                r
            }),
            ("SetValuestring", |api, _k, v| {
                let it = api.cJSON_CreateString(v);
                if it.is_null() {
                    return Res::Null;
                }
                let longer = CString::new("a much longer replacement string value").unwrap();
                let ret = api.cJSON_SetValuestring(it, longer.as_ptr());
                let r = Res::Tree(format!("ret_null={} item={}", ret.is_null(), dump(it)));
                api.cJSON_Delete(it);
                r
            }),
        ];

        for (name, op) in ops {
            let total = alloc_count(&a.c, |api| {
                op(api, key.as_ptr(), text.as_ptr());
            });
            for fail_at in 0..=total {
                let mut results = Vec::new();
                for api in [&a.c, &a.rust] {
                    install(api, fail_at);
                    let r = op(api, key.as_ptr(), text.as_ptr());
                    restore(api);
                    results.push(r);
                }
                assert_eq!(
                    results[0], results[1],
                    "{name} with allocation {fail_at}/{total} failing"
                );
            }
        }
    }
}

/// A huge `prebuffer` makes the very first allocation fail deterministically.
#[test]
fn print_buffered_allocation_refused() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let s = CString::new("[1,2,3]").unwrap();
        let ct = a.c.cJSON_Parse(s.as_ptr());
        let rt = a.rust.cJSON_Parse(s.as_ptr());
        for (api, tree) in [(&a.c, ct), (&a.rust, rt)] {
            install(api, 0);
            let p = api.cJSON_PrintBuffered(tree, c_int::MAX, 1);
            assert!(p.is_null());
            restore(api);
        }
        a.c.cJSON_Delete(ct);
        a.rust.cJSON_Delete(rt);
    }
}
