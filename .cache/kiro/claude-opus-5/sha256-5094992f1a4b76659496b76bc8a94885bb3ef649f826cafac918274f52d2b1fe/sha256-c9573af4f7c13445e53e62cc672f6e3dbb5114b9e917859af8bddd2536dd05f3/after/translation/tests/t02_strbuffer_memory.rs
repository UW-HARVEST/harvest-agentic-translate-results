//! Phase B/C — strbuffer.c and memory.c.
//! CONFIGS rows 6-8, 13-15 · ERRORS rows 114-122.
mod common;
use common::*;
use std::ffi::{c_char, c_void};

unsafe fn sb_bytes(api: &Api, sb: &StrBuffer) -> Vec<u8> {
    unsafe {
        let p = (api.strbuffer_value)(sb);
        if p.is_null() {
            return b"<NULL>".to_vec();
        }
        std::slice::from_raw_parts(p as *const u8, sb.length).to_vec()
    }
}

/* -------- CONFIGS 6: init + append_bytes growth past MIN_SIZE 16 -------- */

#[test]
fn strbuffer_append_bytes_growth() {
    unsafe {
        let mut rng = Rng::new(0x5B01);
        for trial in 0..400 {
            let mut csb = StrBuffer::default();
            let mut rsb = StrBuffer::default();
            assert_eq!((c().strbuffer_init)(&mut csb), 0);
            assert_eq!((r().strbuffer_init)(&mut rsb), 0);
            // The C's documented invariant: fresh buffer is 16 bytes, empty.
            assert_eq!((csb.size, csb.length), (rsb.size, rsb.length), "fresh sizes");

            let nappend = 1 + rng.below(8);
            let mut sizes: Vec<usize> = Vec::new();
            for _ in 0..nappend {
                sizes.push(match rng.below(7) {
                    0 => 0,
                    1 => 1,
                    2 => 15,
                    3 => 16,
                    4 => 17,
                    5 => 1000,
                    _ => rng.below(64),
                });
            }
            for &n in &sizes {
                let data = rng.bytes(n);
                let p = if data.is_empty() {
                    std::ptr::NonNull::<i8>::dangling().as_ptr()
                } else {
                    data.as_ptr() as *const c_char
                };
                let cv = (c().strbuffer_append_bytes)(&mut csb, p, n);
                let rv = (r().strbuffer_append_bytes)(&mut rsb, p, n);
                assert_eq!(cv, rv, "trial {trial} append {n} ret");
                assert_eq!(csb.length, rsb.length, "trial {trial} append {n} length");
                assert_eq!(csb.size, rsb.size, "trial {trial} append {n} size");
                assert_eq!(
                    sb_bytes(c(), &csb),
                    sb_bytes(r(), &rsb),
                    "trial {trial} append {n} contents"
                );
                // NUL terminator must be present in both
                if !csb.value.is_null() {
                    assert_eq!(*csb.value.add(csb.length), 0);
                    assert_eq!(*rsb.value.add(rsb.length), 0);
                }
            }
            (c().strbuffer_close)(&mut csb);
            (r().strbuffer_close)(&mut rsb);
            assert_eq!((csb.size, csb.length), (rsb.size, rsb.length), "post-close");
            assert!(csb.value.is_null() && rsb.value.is_null());
        }
    }
}

/* -------- CONFIGS 7: append_byte / pop (incl. ERRORS 122 pop-on-empty) -------- */

#[test]
fn strbuffer_append_byte_and_pop() {
    unsafe {
        let mut rng = Rng::new(0x5B02);
        for _ in 0..300 {
            let mut csb = StrBuffer::default();
            let mut rsb = StrBuffer::default();
            assert_eq!((c().strbuffer_init)(&mut csb), 0);
            assert_eq!((r().strbuffer_init)(&mut rsb), 0);
            let n = rng.below(40);
            for _ in 0..n {
                let b = (rng.next_u64() & 0xFF) as u8 as c_char;
                assert_eq!(
                    (c().strbuffer_append_byte)(&mut csb, b),
                    (r().strbuffer_append_byte)(&mut rsb, b)
                );
            }
            assert_eq!(sb_bytes(c(), &csb), sb_bytes(r(), &rsb));
            // pop more than we pushed => ERRORS 122
            for i in 0..(n + 5) {
                let cv = (c().strbuffer_pop)(&mut csb);
                let rv = (r().strbuffer_pop)(&mut rsb);
                assert_eq!(cv, rv, "pop #{i}");
                assert_eq!(csb.length, rsb.length, "pop #{i} length");
            }
            (c().strbuffer_close)(&mut csb);
            (r().strbuffer_close)(&mut rsb);
        }
    }
}

/* -------- CONFIGS 8: clear / steal_value -------- */

#[test]
fn strbuffer_clear_and_steal_value() {
    unsafe {
        let mut rng = Rng::new(0x5B03);
        for _ in 0..200 {
            let mut csb = StrBuffer::default();
            let mut rsb = StrBuffer::default();
            assert_eq!((c().strbuffer_init)(&mut csb), 0);
            assert_eq!((r().strbuffer_init)(&mut rsb), 0);
            let n = rng.below(50);
            let data = rng.bytes(n);
            let p = if n == 0 {
                std::ptr::NonNull::<i8>::dangling().as_ptr()
            } else {
                data.as_ptr() as *const c_char
            };
            (c().strbuffer_append_bytes)(&mut csb, p, n);
            (r().strbuffer_append_bytes)(&mut rsb, p, n);

            (c().strbuffer_clear)(&mut csb);
            (r().strbuffer_clear)(&mut rsb);
            assert_eq!(csb.length, rsb.length, "after clear");
            assert_eq!(csb.size, rsb.size, "after clear size");
            assert_eq!(sb_bytes(c(), &csb), sb_bytes(r(), &rsb));

            // reuse after clear
            (c().strbuffer_append_bytes)(&mut csb, p, n);
            (r().strbuffer_append_bytes)(&mut rsb, p, n);
            assert_eq!(sb_bytes(c(), &csb), sb_bytes(r(), &rsb));

            let cv = (c().strbuffer_steal_value)(&mut csb);
            let rv = (r().strbuffer_steal_value)(&mut rsb);
            assert_eq!(cv.is_null(), rv.is_null());
            assert!(csb.value.is_null() && rsb.value.is_null());
            if !cv.is_null() {
                let cb = std::ffi::CStr::from_ptr(cv).to_bytes().to_vec();
                let rb = std::ffi::CStr::from_ptr(rv).to_bytes().to_vec();
                assert_eq!(cb, rb, "stolen value");
                (c().jsonp_free)(cv as *mut c_void);
                (r().jsonp_free)(rv as *mut c_void);
            }
            (c().strbuffer_close)(&mut csb);
            (r().strbuffer_close)(&mut rsb);
        }
    }
}

/* -------- ERRORS 119/120/121: overflow guards in append_bytes -------- */

#[test]
fn strbuffer_append_bytes_overflow_guards() {
    unsafe {
        // ERRORS 120: size > SIZE_MAX - 1  (checked before any read)
        let mut csb = StrBuffer::default();
        let mut rsb = StrBuffer::default();
        assert_eq!((c().strbuffer_init)(&mut csb), 0);
        assert_eq!((r().strbuffer_init)(&mut rsb), 0);
        let dummy = std::ptr::NonNull::<i8>::dangling().as_ptr();
        assert_eq!(
            (c().strbuffer_append_bytes)(&mut csb, dummy, usize::MAX),
            (r().strbuffer_append_bytes)(&mut rsb, dummy, usize::MAX),
            "ERRORS 120: size == SIZE_MAX"
        );
        assert_eq!(
            (c().strbuffer_append_bytes)(&mut csb, dummy, usize::MAX),
            -1
        );

        // ERRORS 121: length > SIZE_MAX - 1 - size, with length >= 1
        let one = [b'x' as c_char];
        (c().strbuffer_append_byte)(&mut csb, one[0]);
        (r().strbuffer_append_byte)(&mut rsb, one[0]);
        assert_eq!(
            (c().strbuffer_append_bytes)(&mut csb, dummy, usize::MAX - 1),
            (r().strbuffer_append_bytes)(&mut rsb, dummy, usize::MAX - 1),
            "ERRORS 121: size == SIZE_MAX-1 with length 1"
        );
        assert_eq!(
            (c().strbuffer_append_bytes)(&mut csb, dummy, usize::MAX - 1),
            -1
        );
        (c().strbuffer_close)(&mut csb);
        (r().strbuffer_close)(&mut rsb);

        // ERRORS 119: strbuff->size > SIZE_MAX / 2.  Reached only with a
        // caller-forged strbuffer_t (the struct is caller-allocated in the C
        // API, so this is a legitimate input across the FFI boundary).
        let mut backing = [0i8; 32];
        let mut cforge = StrBuffer {
            value: backing.as_mut_ptr(),
            length: 0,
            size: usize::MAX,
        };
        let mut rforge = cforge;
        assert_eq!(
            (c().strbuffer_append_bytes)(&mut cforge, dummy, usize::MAX),
            (r().strbuffer_append_bytes)(&mut rforge, dummy, usize::MAX),
            "ERRORS 119: size > SIZE_MAX/2"
        );
        assert_eq!(
            (c().strbuffer_append_bytes)(&mut cforge, dummy, usize::MAX),
            -1
        );
    }
}

/* -------- CONFIGS 13 / ERRORS 114-116 -------- */

#[test]
fn jsonp_malloc_free_realloc_strndup() {
    unsafe {
        // ERRORS 114: jsonp_malloc(0) => NULL (does not call malloc)
        assert!((c().jsonp_malloc)(0).is_null());
        assert!((r().jsonp_malloc)(0).is_null());
        // ERRORS 115: jsonp_free(NULL) is a no-op
        (c().jsonp_free)(std::ptr::null_mut());
        (r().jsonp_free)(std::ptr::null_mut());

        for &n in &[1usize, 7, 16, 1024, 65536] {
            let cp = (c().jsonp_malloc)(n);
            let rp = (r().jsonp_malloc)(n);
            assert!(!cp.is_null() && !rp.is_null(), "jsonp_malloc({n})");
            // grow then shrink
            let cp2 = (c().jsonp_realloc)(cp, n, n * 2);
            let rp2 = (r().jsonp_realloc)(rp, n, n * 2);
            assert!(!cp2.is_null() && !rp2.is_null());
            let cp3 = (c().jsonp_realloc)(cp2, n * 2, 8);
            let rp3 = (r().jsonp_realloc)(rp2, n * 2, 8);
            assert!(!cp3.is_null() && !rp3.is_null());
            (c().jsonp_free)(cp3);
            (r().jsonp_free)(rp3);
        }

        let mut rng = Rng::new(0x5B04);
        for _ in 0..500 {
            let n = rng.below(64);
            let mut data = rng.bytes(n);
            data.push(0);
            let cp = (c().jsonp_strndup)(data.as_ptr() as *const c_char, n);
            let rp = (r().jsonp_strndup)(data.as_ptr() as *const c_char, n);
            assert!(!cp.is_null() && !rp.is_null());
            let cb = std::slice::from_raw_parts(cp as *const u8, n + 1).to_vec();
            let rb = std::slice::from_raw_parts(rp as *const u8, n + 1).to_vec();
            assert_eq!(cb, rb, "jsonp_strndup({n})");
            assert_eq!(cb[n], 0, "jsonp_strndup NUL-terminates");
            (c().jsonp_free)(cp as *mut c_void);
            (r().jsonp_free)(rp as *mut c_void);
        }
    }
}

/* -------- CONFIGS 14/15 + ERRORS 116-118: allocator hooks -------- */

static mut C_MALLOC_CALLS: usize = 0;
static mut C_FREE_CALLS: usize = 0;

unsafe extern "C" fn my_malloc(n: usize) -> *mut c_void {
    unsafe {
        C_MALLOC_CALLS += 1;
        libc_malloc(n)
    }
}
unsafe extern "C" fn my_free(p: *mut c_void) {
    unsafe {
        C_FREE_CALLS += 1;
        libc_free(p)
    }
}
unsafe extern "C" fn my_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    unsafe { libc_realloc(p, n) }
}

unsafe extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
}

#[test]
fn alloc_funcs_get_set_roundtrip() {
    unsafe {
        for api in both() {
            // Capture the defaults so we can restore them.
            let mut m0: Option<MallocFn> = None;
            let mut re0: Option<ReallocFn> = None;
            let mut f0: Option<FreeFn> = None;
            (api.json_get_alloc_funcs2)(&mut m0, &mut re0, &mut f0);
            assert!(m0.is_some(), "{}: default malloc_fn", api.tag);
            assert!(re0.is_some(), "{}: default realloc_fn", api.tag);
            assert!(f0.is_some(), "{}: default free_fn", api.tag);

            // ERRORS 117/118: NULL out-pointers must be skipped, not crash.
            (api.json_get_alloc_funcs)(std::ptr::null_mut(), std::ptr::null_mut());
            (api.json_get_alloc_funcs2)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );

            // json_set_alloc_funcs must set do_realloc = NULL.
            (api.json_set_alloc_funcs)(Some(my_malloc), Some(my_free));
            let mut m1: Option<MallocFn> = None;
            let mut re1: Option<ReallocFn> = Some(my_realloc);
            let mut f1: Option<FreeFn> = None;
            (api.json_get_alloc_funcs2)(&mut m1, &mut re1, &mut f1);
            assert!(
                re1.is_none(),
                "{}: json_set_alloc_funcs must clear realloc",
                api.tag
            );
            assert_eq!(
                m1.map(|f| f as usize),
                Some(my_malloc as usize),
                "{}: malloc_fn readback",
                api.tag
            );
            assert_eq!(
                f1.map(|f| f as usize),
                Some(my_free as usize),
                "{}: free_fn readback",
                api.tag
            );

            // 2-arg get must agree with the 3-arg one.
            let mut m1b: Option<MallocFn> = None;
            let mut f1b: Option<FreeFn> = None;
            (api.json_get_alloc_funcs)(&mut m1b, &mut f1b);
            assert_eq!(m1b.map(|f| f as usize), m1.map(|f| f as usize));
            assert_eq!(f1b.map(|f| f as usize), f1.map(|f| f as usize));

            // ERRORS 116: realloc-emulation path (do_realloc == NULL).
            let p = (api.jsonp_malloc)(16);
            assert!(!p.is_null());
            let p2 = (api.jsonp_realloc)(p, 16, 64);
            assert!(!p2.is_null(), "{}: emulated realloc grow", api.tag);
            let p3 = (api.jsonp_realloc)(p2, 64, 0);
            assert!(p3.is_null(), "{}: emulated realloc to 0 => NULL", api.tag);
            // realloc(NULL, 0) in emulation => NULL, no free
            assert!((api.jsonp_realloc)(std::ptr::null_mut(), 0, 0).is_null());

            // json_set_alloc_funcs2 sets all three.
            (api.json_set_alloc_funcs2)(Some(my_malloc), Some(my_realloc), Some(my_free));
            let mut m2: Option<MallocFn> = None;
            let mut re2: Option<ReallocFn> = None;
            let mut f2: Option<FreeFn> = None;
            (api.json_get_alloc_funcs2)(&mut m2, &mut re2, &mut f2);
            assert_eq!(re2.map(|f| f as usize), Some(my_realloc as usize));

            // A real allocation must still work through the hooks.
            let s = cs("through-hooks");
            let js = (api.json_string)(s.as_ptr());
            assert!(!js.is_null());
            let d = dumps(api, js, JSON_ENCODE_ANY);
            assert_eq!(d.as_deref(), Some(&b"\"through-hooks\""[..]));
            decref(api, js);

            // restore
            (api.json_set_alloc_funcs2)(m0, re0, f0);
            let mut m3: Option<MallocFn> = None;
            let mut re3: Option<ReallocFn> = None;
            let mut f3: Option<FreeFn> = None;
            (api.json_get_alloc_funcs2)(&mut m3, &mut re3, &mut f3);
            assert_eq!(m3.map(|f| f as usize), m0.map(|f| f as usize));
            assert_eq!(re3.map(|f| f as usize), re0.map(|f| f as usize));
            assert_eq!(f3.map(|f| f as usize), f0.map(|f| f as usize));
        }
        assert!(C_MALLOC_CALLS > 0 && C_FREE_CALLS > 0, "hooks were exercised");
    }
}
