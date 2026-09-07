//! Phase B part 1 — CONFIGS rows 1-49: version, hooks, constructors,
//! accessors, predicates, setters.
mod harness;
use harness::*;
use std::ffi::{c_char, c_double, c_float, c_int, c_void};

const SEED: u64 = 0x5EED_C7501_u64;

/* ---------------- rows 1-2 ---------------- */

#[test]
fn row01_version() {
    let _g = lock();
    let p = pair();
    unsafe {
        assert_eq!(
            read_cstr((p.c.cJSON_Version)()),
            read_cstr((p.r.cJSON_Version)())
        );
    }
}

#[test]
fn row02_malloc_free() {
    let _g = lock();
    let p = pair();
    unsafe {
        for sz in [1usize, 8, 64, 4096, 0] {
            let a = (p.c.cJSON_malloc)(sz);
            let b = (p.r.cJSON_malloc)(sz);
            assert_eq!(a.is_null(), b.is_null(), "malloc({sz}) nullness differs");
            (p.c.cJSON_free)(a);
            (p.r.cJSON_free)(b);
        }
        // free(NULL) must be a no-op on both
        (p.c.cJSON_free)(std::ptr::null_mut());
        (p.r.cJSON_free)(std::ptr::null_mut());
    }
}

/* ---------------- rows 3-6: cJSON_InitHooks ---------------- */

unsafe extern "C" fn my_malloc(sz: usize) -> *mut c_void {
    libc_malloc(sz)
}
unsafe extern "C" fn my_free(ptr: *mut c_void) {
    libc_free(ptr)
}
extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(sz: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

/// Runs a full parse+print pipeline under a given hook configuration and
/// returns the outputs, then restores default hooks.
unsafe fn pipeline_under_hooks(api: &Api, hooks: Option<cJSON_Hooks>) -> Vec<Option<Vec<u8>>> {
    let mut h = hooks;
    match h.as_mut() {
        Some(hp) => (api.cJSON_InitHooks)(hp as *mut cJSON_Hooks),
        None => (api.cJSON_InitHooks)(std::ptr::null_mut()),
    }
    let mut rng = Rng::new(SEED ^ 0xABCD);
    let mut out = Vec::new();
    for _ in 0..40 {
        let doc = gen_json(&mut rng, 0);
        let cs = cstr(&doc);
        let item = (api.cJSON_Parse)(sp(&cs));
        out.push(take_print(api, (api.cJSON_Print)(item)));
        out.push(take_print(api, (api.cJSON_PrintUnformatted)(item)));
        out.push(take_print(api, (api.cJSON_PrintBuffered)(item, 1, 1)));
        (api.cJSON_Delete)(item);
    }
    // large document to force repeated ensure()/realloc growth
    let big = format!("[{}]", (0..2000).map(|i| i.to_string()).collect::<Vec<_>>().join(","));
    let bs = cstr(&big);
    let item = (api.cJSON_Parse)(sp(&bs));
    out.push(take_print(api, (api.cJSON_Print)(item)));
    out.push(take_print(api, (api.cJSON_PrintBuffered)(item, 2, 1)));
    (api.cJSON_Delete)(item);
    // restore defaults
    (api.cJSON_InitHooks)(std::ptr::null_mut());
    out
}

#[test]
fn rows03_06_init_hooks_variants() {
    let _g = lock();
    let p = pair();
    let variants: Vec<Option<cJSON_Hooks>> = vec![
        None, // row 3: reset to defaults
        Some(cJSON_Hooks {
            malloc_fn: Some(my_malloc),
            free_fn: Some(my_free),
        }), // row 4: both -> reallocate == NULL
        Some(cJSON_Hooks {
            malloc_fn: Some(my_malloc),
            free_fn: None,
        }), // row 5
        Some(cJSON_Hooks {
            malloc_fn: None,
            free_fn: Some(my_free),
        }), // row 6
        Some(cJSON_Hooks {
            malloc_fn: None,
            free_fn: None,
        }), // both NULL -> defaults, reallocate == realloc
    ];
    for (i, v) in variants.into_iter().enumerate() {
        unsafe {
            let co = pipeline_under_hooks(&p.c, v);
            let ro = pipeline_under_hooks(&p.r, v);
            assert_eq!(co.len(), ro.len());
            for (j, (a, b)) in co.iter().zip(ro.iter()).enumerate() {
                assert_eq!(a, b, "hook variant {i}, output {j}");
            }
        }
    }
}

/* ---------------- rows 7-8, 33-34: scalars & bools ---------------- */

#[test]
fn rows07_08_scalar_and_bool_constructors() {
    let _g = lock();
    let p = pair();
    unsafe {
        for (name, cf, rf) in [
            ("null", p.c.cJSON_CreateNull, p.r.cJSON_CreateNull),
            ("true", p.c.cJSON_CreateTrue, p.r.cJSON_CreateTrue),
            ("false", p.c.cJSON_CreateFalse, p.r.cJSON_CreateFalse),
        ] {
            let ci = cf();
            let ri = rf();
            assert_snap_eq(ci, ri, name);
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri), "print {name}");
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // row 8: non-0/1 booleans across FFI
        for b in [0i32, 1, 2, -1, 0x100, i32::MIN, i32::MAX] {
            let ci = (p.c.cJSON_CreateBool)(b);
            let ri = (p.r.cJSON_CreateBool)(b);
            assert_snap_eq(ci, ri, &format!("CreateBool({b})"));
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* ---------------- rows 9-14, 45, 47: numbers ---------------- */

fn number_corpus() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        1e5,
        1e-5,
        1e300,
        -1e300,
        1e-300,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        i32::MAX as f64,
        i32::MIN as f64,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64 - 1.0,
        i64::MAX as f64,
        1.0 / 3.0,
        2.0 / 3.0,
        0.1,
        0.2,
        0.3,
        1234567890.12345,
        1.7976931348623157e308,
        5e-324,
        -5e-324,
        123456789012345678.0,
        3.141592653589793,
        2.718281828459045,
    ];
    let mut rng = Rng::new(SEED ^ 0x1111);
    for _ in 0..800 {
        v.push(rng.nice_f64());
    }
    v
}

#[test]
fn rows09_14_number_constructor_and_all_print_paths() {
    let _g = lock();
    let p = pair();
    unsafe {
        for d in number_corpus() {
            let ci = (p.c.cJSON_CreateNumber)(d);
            let ri = (p.r.cJSON_CreateNumber)(d);
            assert_snap_eq(ci, ri, &format!("CreateNumber({d:?} bits {:#x})", d.to_bits()));
            assert_eq!(
                print_all(&p.c, ci),
                print_all(&p.r, ri),
                "print CreateNumber({d:?} bits {:#x})",
                d.to_bits()
            );
            // row 45
            let cg = (p.c.cJSON_GetNumberValue)(ci);
            let rg = (p.r.cJSON_GetNumberValue)(ri);
            assert_eq!(cg.to_bits(), rg.to_bits(), "GetNumberValue({d:?})");
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

#[test]
fn row47_set_number_helper() {
    let _g = lock();
    let p = pair();
    unsafe {
        for d in number_corpus() {
            let ci = (p.c.cJSON_CreateNumber)(1.0);
            let ri = (p.r.cJSON_CreateNumber)(1.0);
            let cv = (p.c.cJSON_SetNumberHelper)(ci, d);
            let rv = (p.r.cJSON_SetNumberHelper)(ri, d);
            assert_eq!(cv.to_bits(), rv.to_bits(), "SetNumberHelper ret ({d:?})");
            assert_snap_eq(ci, ri, &format!("SetNumberHelper({d:?})"));
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* ---------------- rows 15-21, 44, 48-49: strings & raw ---------------- */

fn string_corpus() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"quote\"inside".to_vec(),
        b"back\\slash".to_vec(),
        b"tab\there".to_vec(),
        b"nl\nhere".to_vec(),
        b"cr\rhere".to_vec(),
        b"bs\x08here".to_vec(),
        b"ff\x0chere".to_vec(),
        b"slash/here".to_vec(),
        vec![1, 2, 3, 0x1f],
        "\u{e9}\u{20ac}\u{1f600}".as_bytes().to_vec(),
        vec![0x80, 0xff, 0xc3],
        vec![b'x'; 5000],
    ];
    // all escape-triggering chars together
    let mut all = Vec::new();
    for b in 1u8..=0x20 {
        all.push(b);
    }
    all.extend_from_slice(b"\"\\/");
    v.push(all);
    let mut rng = Rng::new(SEED ^ 0x2222);
    for _ in 0..400 {
        v.push(rng.cstring(40));
    }
    v
}

#[test]
fn rows15_21_string_and_raw_constructors() {
    let _g = lock();
    let p = pair();
    unsafe {
        for s in string_corpus() {
            let cs = cbytes(&s);
            for which in 0..3 {
                let (ci, ri) = match which {
                    0 => (
                        (p.c.cJSON_CreateString)(sp(&cs)),
                        (p.r.cJSON_CreateString)(sp(&cs)),
                    ),
                    1 => (
                        (p.c.cJSON_CreateStringReference)(sp(&cs)),
                        (p.r.cJSON_CreateStringReference)(sp(&cs)),
                    ),
                    _ => (
                        (p.c.cJSON_CreateRaw)(sp(&cs)),
                        (p.r.cJSON_CreateRaw)(sp(&cs)),
                    ),
                };
                assert_snap_eq(ci, ri, &format!("ctor {which} {:?}", &s[..s.len().min(16)]));
                assert_eq!(
                    print_all(&p.c, ci),
                    print_all(&p.r, ri),
                    "print ctor {which} {:?}",
                    &s[..s.len().min(16)]
                );
                // row 44
                assert_eq!(
                    read_cstr((p.c.cJSON_GetStringValue)(ci)),
                    read_cstr((p.r.cJSON_GetStringValue)(ri))
                );
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}


#[test]
fn rows48_49_set_valuestring() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x3333);
    unsafe {
        for _ in 0..300 {
            let orig = rng.cstring(30);
            let newv = rng.cstring(30);
            let o = cbytes(&orig);
            let n = cbytes(&newv);
            let ci = (p.c.cJSON_CreateString)(sp(&o));
            let ri = (p.r.cJSON_CreateString)(sp(&o));
            let cr = (p.c.cJSON_SetValuestring)(ci, sp(&n));
            let rr = (p.r.cJSON_SetValuestring)(ri, sp(&n));
            assert_eq!(
                cr.is_null(),
                rr.is_null(),
                "SetValuestring nullness {orig:?} -> {newv:?}"
            );
            assert_eq!(read_cstr(cr), read_cstr(rr));
            assert_snap_eq(ci, ri, "SetValuestring");
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // explicit shorter / equal / longer
        for (a, b) in [
            ("abcdef", "xyz"),
            ("abc", "abc"),
            ("abc", "abcdefghij"),
            ("", ""),
            ("", "z"),
            ("z", ""),
        ] {
            let o = cstr(a);
            let n = cstr(b);
            let ci = (p.c.cJSON_CreateString)(sp(&o));
            let ri = (p.r.cJSON_CreateString)(sp(&o));
            let cr = (p.c.cJSON_SetValuestring)(ci, sp(&n));
            let rr = (p.r.cJSON_SetValuestring)(ri, sp(&n));
            assert_eq!(read_cstr(cr), read_cstr(rr), "{a} -> {b}");
            assert_snap_eq(ci, ri, &format!("{a} -> {b}"));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* ---------------- rows 22-23: containers & references ---------------- */

#[test]
fn rows22_23_containers_and_references() {
    let _g = lock();
    let p = pair();
    unsafe {
        for empty_kind in 0..2 {
            let (ci, ri) = if empty_kind == 0 {
                ((p.c.cJSON_CreateArray)(), (p.r.cJSON_CreateArray)())
            } else {
                ((p.c.cJSON_CreateObject)(), (p.r.cJSON_CreateObject)())
            };
            assert_snap_eq(ci, ri, "empty container");
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // references over a populated child chain
        let src = cstr("[1,2,3]");
        let ca = (p.c.cJSON_Parse)(sp(&src));
        let ra = (p.r.cJSON_Parse)(sp(&src));
        let cref = (p.c.cJSON_CreateArrayReference)((*ca).child);
        let rref = (p.r.cJSON_CreateArrayReference)((*ra).child);
        assert_snap_eq(cref, rref, "array reference");
        assert_eq!(print_all(&p.c, cref), print_all(&p.r, rref));
        (p.c.cJSON_Delete)(cref);
        (p.r.cJSON_Delete)(rref);

        let osrc = cstr("{\"a\":1,\"b\":2}");
        let co = (p.c.cJSON_Parse)(sp(&osrc));
        let ro = (p.r.cJSON_Parse)(sp(&osrc));
        let coref = (p.c.cJSON_CreateObjectReference)((*co).child);
        let roref = (p.r.cJSON_CreateObjectReference)((*ro).child);
        assert_snap_eq(coref, roref, "object reference");
        assert_eq!(print_all(&p.c, coref), print_all(&p.r, roref));
        (p.c.cJSON_Delete)(coref);
        (p.r.cJSON_Delete)(roref);

        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

/* ---------------- rows 24-27: typed array constructors ---------------- */

#[test]
fn rows24_27_typed_array_constructors() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x4444);
    unsafe {
        // ints
        for _ in 0..120 {
            let n = rng.below(65);
            let mut nums: Vec<c_int> = (0..n).map(|_| rng.i32()).collect();
            if n > 0 {
                nums[0] = i32::MIN;
                nums[n - 1] = i32::MAX;
            }
            let ci = (p.c.cJSON_CreateIntArray)(nums.as_ptr(), n as c_int);
            let ri = (p.r.cJSON_CreateIntArray)(nums.as_ptr(), n as c_int);
            assert_snap_eq(ci, ri, &format!("IntArray n={n}"));
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // floats
        for _ in 0..120 {
            let n = rng.below(65);
            let nums: Vec<c_float> = (0..n)
                .map(|_| match rng.below(6) {
                    0 => 0.0f32,
                    1 => f32::NAN,
                    2 => f32::INFINITY,
                    3 => f32::MAX,
                    4 => f32::MIN_POSITIVE,
                    _ => f32::from_bits(rng.next_u64() as u32),
                })
                .collect();
            let ci = (p.c.cJSON_CreateFloatArray)(nums.as_ptr(), n as c_int);
            let ri = (p.r.cJSON_CreateFloatArray)(nums.as_ptr(), n as c_int);
            assert_snap_eq(ci, ri, &format!("FloatArray n={n}"));
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // doubles
        for _ in 0..120 {
            let n = rng.below(65);
            let nums: Vec<c_double> = (0..n).map(|_| rng.nice_f64()).collect();
            let ci = (p.c.cJSON_CreateDoubleArray)(nums.as_ptr(), n as c_int);
            let ri = (p.r.cJSON_CreateDoubleArray)(nums.as_ptr(), n as c_int);
            assert_snap_eq(ci, ri, &format!("DoubleArray n={n}"));
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // strings
        for _ in 0..120 {
            let n = rng.below(33);
            let bufs: Vec<Vec<u8>> = (0..n).map(|_| cbytes(&rng.cstring(20))).collect();
            let ptrs: Vec<*const c_char> = bufs.iter().map(|b| sp(b)).collect();
            let ci = (p.c.cJSON_CreateStringArray)(ptrs.as_ptr(), n as c_int);
            let ri = (p.r.cJSON_CreateStringArray)(ptrs.as_ptr(), n as c_int);
            assert_snap_eq(ci, ri, &format!("StringArray n={n}"));
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* ---------------- rows 28-38: add* ---------------- */

#[test]
fn rows28_32_add_item_variants() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x5555);
    unsafe {
        // row 28: incremental array build (0->1, 1->2, n->n+1)
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        for i in 0..40 {
            let cn = (p.c.cJSON_CreateNumber)(i as f64);
            let rn = (p.r.cJSON_CreateNumber)(i as f64);
            let cok = (p.c.cJSON_AddItemToArray)(ca, cn);
            let rok = (p.r.cJSON_AddItemToArray)(ra, rn);
            assert_eq!(cok, rok, "AddItemToArray i={i}");
            assert_snap_eq(ca, ra, &format!("array after {i}"));
            assert_eq!((p.c.cJSON_GetArraySize)(ca), (p.r.cJSON_GetArraySize)(ra));
        }
        assert_eq!(print_all(&p.c, ca), print_all(&p.r, ra));
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 29/30: AddItemToObject (strdup key) vs AddItemToObjectCS (const key)
        for cs_mode in 0..2 {
            let co = (p.c.cJSON_CreateObject)();
            let ro = (p.r.cJSON_CreateObject)();
            let mut keys = Vec::new();
            for i in 0..25 {
                let k = cbytes(&{
                    let mut v = format!("k{i}").into_bytes();
                    v.extend_from_slice(&rng.cstring(6));
                    v
                });
                keys.push(k);
            }
            for k in &keys {
                let cn = (p.c.cJSON_CreateNumber)(rng.nice_f64());
                let rn = (p.r.cJSON_CreateNumber)(0.0);
                (*rn).valuedouble = (*cn).valuedouble;
                (*rn).valueint = (*cn).valueint;
                let (cok, rok) = if cs_mode == 0 {
                    (
                        (p.c.cJSON_AddItemToObject)(co, sp(k), cn),
                        (p.r.cJSON_AddItemToObject)(ro, sp(k), rn),
                    )
                } else {
                    (
                        (p.c.cJSON_AddItemToObjectCS)(co, sp(k), cn),
                        (p.r.cJSON_AddItemToObjectCS)(ro, sp(k), rn),
                    )
                };
                assert_eq!(cok, rok);
                assert_snap_eq(co, ro, "object build");
            }
            assert_eq!(print_all(&p.c, co), print_all(&p.r, ro));
            (p.c.cJSON_Delete)(co);
            (p.r.cJSON_Delete)(ro);
        }

        // rows 31/32: AddItemReference*
        let cs = cstr("ref-target");
        let cbase = (p.c.cJSON_CreateString)(sp(&cs));
        let rbase = (p.r.cJSON_CreateString)(sp(&cs));
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToArray)(ca, cbase),
            (p.r.cJSON_AddItemReferenceToArray)(ra, rbase)
        );
        assert_snap_eq(ca, ra, "AddItemReferenceToArray");
        assert_eq!(print_all(&p.c, ca), print_all(&p.r, ra));
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        let k = cstr("refkey");
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToObject)(co, sp(&k), cbase),
            (p.r.cJSON_AddItemReferenceToObject)(ro, sp(&k), rbase)
        );
        assert_snap_eq(co, ro, "AddItemReferenceToObject");
        assert_eq!(print_all(&p.c, co), print_all(&p.r, ro));
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
        (p.c.cJSON_Delete)(cbase);
        (p.r.cJSON_Delete)(rbase);
    }
}

#[test]
fn rows33_38_add_convenience_wrappers() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x6666);
    unsafe {
        for _ in 0..60 {
            let co = (p.c.cJSON_CreateObject)();
            let ro = (p.r.cJSON_CreateObject)();
            let mut names: Vec<Vec<u8>> = Vec::new();
            for i in 0..9 {
                names.push(cbytes(format!("n{i}").as_bytes()));
            }
            // 33
            for (i, f) in [0usize, 1, 2].iter().enumerate() {
                let n = &names[i];
                let (cp, rp) = match f {
                    0 => (
                        (p.c.cJSON_AddNullToObject)(co, sp(n)),
                        (p.r.cJSON_AddNullToObject)(ro, sp(n)),
                    ),
                    1 => (
                        (p.c.cJSON_AddTrueToObject)(co, sp(n)),
                        (p.r.cJSON_AddTrueToObject)(ro, sp(n)),
                    ),
                    _ => (
                        (p.c.cJSON_AddFalseToObject)(co, sp(n)),
                        (p.r.cJSON_AddFalseToObject)(ro, sp(n)),
                    ),
                };
                assert_eq!(cp.is_null(), rp.is_null());
                assert_snap_eq(cp, rp, "add scalar wrapper");
            }
            // 34
            let b = [0i32, 1, 2, -1, 0x100][rng.below(5)];
            let cp = (p.c.cJSON_AddBoolToObject)(co, sp(&names[3]), b);
            let rp = (p.r.cJSON_AddBoolToObject)(ro, sp(&names[3]), b);
            assert_snap_eq(cp, rp, "AddBoolToObject");
            // 35
            let d = rng.nice_f64();
            let cp = (p.c.cJSON_AddNumberToObject)(co, sp(&names[4]), d);
            let rp = (p.r.cJSON_AddNumberToObject)(ro, sp(&names[4]), d);
            assert_snap_eq(cp, rp, "AddNumberToObject");
            // 36
            let s = cbytes(&rng.cstring(20));
            let cp = (p.c.cJSON_AddStringToObject)(co, sp(&names[5]), sp(&s));
            let rp = (p.r.cJSON_AddStringToObject)(ro, sp(&names[5]), sp(&s));
            assert_snap_eq(cp, rp, "AddStringToObject");
            // 37
            let raw = cstr("[1,2,3]");
            let cp = (p.c.cJSON_AddRawToObject)(co, sp(&names[6]), sp(&raw));
            let rp = (p.r.cJSON_AddRawToObject)(ro, sp(&names[6]), sp(&raw));
            assert_snap_eq(cp, rp, "AddRawToObject");
            // 38
            let cinner = (p.c.cJSON_AddObjectToObject)(co, sp(&names[7]));
            let rinner = (p.r.cJSON_AddObjectToObject)(ro, sp(&names[7]));
            let ik = cstr("deep");
            (p.c.cJSON_AddNumberToObject)(cinner, sp(&ik), 7.5);
            (p.r.cJSON_AddNumberToObject)(rinner, sp(&ik), 7.5);
            let carr = (p.c.cJSON_AddArrayToObject)(co, sp(&names[8]));
            let rarr = (p.r.cJSON_AddArrayToObject)(ro, sp(&names[8]));
            (p.c.cJSON_AddItemToArray)(carr, (p.c.cJSON_CreateNumber)(1.0));
            (p.r.cJSON_AddItemToArray)(rarr, (p.r.cJSON_CreateNumber)(1.0));

            assert_snap_eq(co, ro, "convenience-built object");
            assert_eq!(print_all(&p.c, co), print_all(&p.r, ro));
            (p.c.cJSON_Delete)(co);
            (p.r.cJSON_Delete)(ro);
        }
    }
}

/* ---------------- rows 39-43, 46: accessors & predicates ---------------- */

#[test]
fn rows39_43_accessors() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x7777);
    unsafe {
        for _ in 0..200 {
            let doc = gen_json(&mut rng, 0);
            let cs = cstr(&doc);
            let ci = (p.c.cJSON_Parse)(sp(&cs));
            let ri = (p.r.cJSON_Parse)(sp(&cs));
            assert_eq!(ci.is_null(), ri.is_null(), "parse {doc}");
            if ci.is_null() {
                continue;
            }
            let csz = (p.c.cJSON_GetArraySize)(ci);
            let rsz = (p.r.cJSON_GetArraySize)(ri);
            assert_eq!(csz, rsz, "GetArraySize {doc}");
            for idx in -2..(csz + 3) {
                let cit = (p.c.cJSON_GetArrayItem)(ci, idx);
                let rit = (p.r.cJSON_GetArrayItem)(ri, idx);
                assert_eq!(cit.is_null(), rit.is_null(), "GetArrayItem {idx} {doc}");
                if !cit.is_null() {
                    assert_snap_eq(cit, rit, &format!("item {idx}"));
                }
            }
            // object key lookups, case sensitive and not
            let mut cur = (*ci).child;
            while !cur.is_null() {
                if let Some(key) = read_cstr((*cur).string) {
                    for variant in 0..3 {
                        let k = match variant {
                            0 => key.clone(),
                            1 => key.iter().map(|c| c.to_ascii_uppercase()).collect(),
                            _ => key.iter().map(|c| c.to_ascii_lowercase()).collect(),
                        };
                        let kb = cbytes(&k);
                        let a = (p.c.cJSON_GetObjectItem)(ci, sp(&kb));
                        let b = (p.r.cJSON_GetObjectItem)(ri, sp(&kb));
                        assert_eq!(a.is_null(), b.is_null(), "GetObjectItem {k:?} in {doc}");
                        if !a.is_null() {
                            assert_snap_eq(a, b, "GetObjectItem");
                        }
                        let a = (p.c.cJSON_GetObjectItemCaseSensitive)(ci, sp(&kb));
                        let b = (p.r.cJSON_GetObjectItemCaseSensitive)(ri, sp(&kb));
                        assert_eq!(a.is_null(), b.is_null(), "GetObjectItemCS {k:?}");
                        if !a.is_null() {
                            assert_snap_eq(a, b, "GetObjectItemCS");
                        }
                        assert_eq!(
                            (p.c.cJSON_HasObjectItem)(ci, sp(&kb)),
                            (p.r.cJSON_HasObjectItem)(ri, sp(&kb)),
                            "HasObjectItem {k:?}"
                        );
                    }
                }
                cur = (*cur).next;
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // duplicate keys: first match wins
        let dup = cstr("{\"a\":1,\"A\":2,\"a\":3}");
        let ci = (p.c.cJSON_Parse)(sp(&dup));
        let ri = (p.r.cJSON_Parse)(sp(&dup));
        for k in ["a", "A"] {
            let kb = cstr(k);
            assert_snap_eq(
                (p.c.cJSON_GetObjectItem)(ci, sp(&kb)),
                (p.r.cJSON_GetObjectItem)(ri, sp(&kb)),
                "dup insensitive",
            );
            assert_snap_eq(
                (p.c.cJSON_GetObjectItemCaseSensitive)(ci, sp(&kb)),
                (p.r.cJSON_GetObjectItemCaseSensitive)(ri, sp(&kb)),
                "dup sensitive",
            );
        }
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
        // non-ASCII bytes through tolower()
        let hi = cbytes(&[0xc3, 0xa9, b'K']);
        let json = {
            let mut v = b"{\"".to_vec();
            v.extend_from_slice(&[0xc3, 0xa9, b'K']);
            v.extend_from_slice(b"\":1}");
            cbytes(&v)
        };
        let ci = (p.c.cJSON_Parse)(sp(&json));
        let ri = (p.r.cJSON_Parse)(sp(&json));
        assert_eq!(
            (p.c.cJSON_HasObjectItem)(ci, sp(&hi)),
            (p.r.cJSON_HasObjectItem)(ri, sp(&hi))
        );
        let lo = cbytes(&[0xc3, 0xa9, b'k']);
        assert_eq!(
            (p.c.cJSON_HasObjectItem)(ci, sp(&lo)),
            (p.r.cJSON_HasObjectItem)(ri, sp(&lo))
        );
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
    }
}

/// Build one item of every type (incl. reference / const-string flavours) and
/// run every `cJSON_Is*` predicate against it — CONFIGS row 46.
#[test]
fn row46_predicate_matrix() {
    let _g = lock();
    let p = pair();
    unsafe {
        let s = cstr("s");
        let ctors: Vec<(&str, Box<dyn Fn(&Api) -> *mut cJSON>)> = vec![
            ("null", Box::new(|a: &Api| (a.cJSON_CreateNull)())),
            ("true", Box::new(|a: &Api| (a.cJSON_CreateTrue)())),
            ("false", Box::new(|a: &Api| (a.cJSON_CreateFalse)())),
            ("bool0", Box::new(|a: &Api| (a.cJSON_CreateBool)(0))),
            ("bool1", Box::new(|a: &Api| (a.cJSON_CreateBool)(1))),
            ("number", Box::new(|a: &Api| (a.cJSON_CreateNumber)(4.25))),
            ("array", Box::new(|a: &Api| (a.cJSON_CreateArray)())),
            ("object", Box::new(|a: &Api| (a.cJSON_CreateObject)())),
        ];
        let preds: [(&str, fn(&Api) -> unsafe extern "C" fn(*const cJSON) -> c_int); 10] = [
            ("IsInvalid", |a| a.cJSON_IsInvalid),
            ("IsFalse", |a| a.cJSON_IsFalse),
            ("IsTrue", |a| a.cJSON_IsTrue),
            ("IsBool", |a| a.cJSON_IsBool),
            ("IsNull", |a| a.cJSON_IsNull),
            ("IsNumber", |a| a.cJSON_IsNumber),
            ("IsString", |a| a.cJSON_IsString),
            ("IsArray", |a| a.cJSON_IsArray),
            ("IsObject", |a| a.cJSON_IsObject),
            ("IsRaw", |a| a.cJSON_IsRaw),
        ];
        for (name, f) in &ctors {
            let ci = f(&p.c);
            let ri = f(&p.r);
            for (pname, pf) in preds {
                assert_eq!(
                    pf(&p.c)(ci),
                    pf(&p.r)(ri),
                    "{pname} on {name}"
                );
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // string / raw / references / const-key flavours
        for kind in 0..4 {
            let (ci, ri) = match kind {
                0 => (
                    (p.c.cJSON_CreateString)(sp(&s)),
                    (p.r.cJSON_CreateString)(sp(&s)),
                ),
                1 => ((p.c.cJSON_CreateRaw)(sp(&s)), (p.r.cJSON_CreateRaw)(sp(&s))),
                2 => (
                    (p.c.cJSON_CreateStringReference)(sp(&s)),
                    (p.r.cJSON_CreateStringReference)(sp(&s)),
                ),
                _ => {
                    let ca = (p.c.cJSON_CreateArray)();
                    let ra = (p.r.cJSON_CreateArray)();
                    (
                        (p.c.cJSON_CreateArrayReference)(ca),
                        (p.r.cJSON_CreateArrayReference)(ra),
                    )
                }
            };
            for (pname, pf) in preds {
                assert_eq!(pf(&p.c)(ci), pf(&p.r)(ri), "{pname} on kind {kind}");
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* ---------------- row 96: header setter macros ---------------- */

#[test]
fn row96_header_setter_macros() {
    let _g = lock();
    let p = pair();
    unsafe {
        // cJSON_SetIntValue(object, number): sets valuedouble and valueint
        for v in [0i32, 1, -1, i32::MAX, i32::MIN, 12345] {
            let ci = (p.c.cJSON_CreateNumber)(0.0);
            let ri = (p.r.cJSON_CreateNumber)(0.0);
            // replicate the macro: valueint = valuedouble = v
            (*ci).valuedouble = v as f64;
            (*ci).valueint = v;
            (*ri).valuedouble = v as f64;
            (*ri).valueint = v;
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri), "SetIntValue {v}");
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // cJSON_SetNumberValue -> cJSON_SetNumberHelper (covered in row 47)
        // cJSON_SetBoolValue: flip type bits on a boolean item
        for start_true in [false, true] {
            for target in [0i32, 1, 5] {
                let (ci, ri) = if start_true {
                    ((p.c.cJSON_CreateTrue)(), (p.r.cJSON_CreateTrue)())
                } else {
                    ((p.c.cJSON_CreateFalse)(), (p.r.cJSON_CreateFalse)())
                };
                for it in [ci, ri] {
                    if (*it).type_ & (cJSON_False | cJSON_True) != 0 {
                        (*it).type_ = ((*it).type_ & !(cJSON_False | cJSON_True))
                            | if target != 0 { cJSON_True } else { cJSON_False };
                    }
                }
                assert_snap_eq(ci, ri, "SetBoolValue");
                assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}
