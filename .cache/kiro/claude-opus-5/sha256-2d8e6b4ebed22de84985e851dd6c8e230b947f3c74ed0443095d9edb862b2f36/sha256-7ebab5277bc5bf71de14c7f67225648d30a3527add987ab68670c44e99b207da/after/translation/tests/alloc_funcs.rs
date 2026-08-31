//! Allocator-hook tests. These mutate process-global state inside each
//! library, so they live in their own test binary (own process) and are
//! serialised through a mutex.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::c_void;

static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

type FnGetAlloc = unsafe extern "C" fn(*mut Option<FnMalloc>, *mut Option<FnFree>);
type FnSetAlloc = unsafe extern "C" fn(Option<FnMalloc>, Option<FnFree>);
type FnGetAlloc2 = unsafe extern "C" fn(
    *mut Option<FnMalloc>,
    *mut Option<FnRealloc>,
    *mut Option<FnFree>,
);
type FnSetAlloc2 =
    unsafe extern "C" fn(Option<FnMalloc>, Option<FnRealloc>, Option<FnFree>);

#[test]
fn alloc_funcs_getters_match() {
    let _g = LOCK.lock().unwrap();
    let (c, r) = libs();
    for l in [c, r] {
        let g: Symbol<FnGetAlloc> = l.sym("json_get_alloc_funcs");
        let g2: Symbol<FnGetAlloc2> = l.sym("json_get_alloc_funcs2");
        unsafe {
            let mut m: Option<FnMalloc> = None;
            let mut f: Option<FnFree> = None;
            g(&mut m, &mut f);
            assert!(m.is_some(), "{}: malloc fn", l.name);
            assert!(f.is_some(), "{}: free fn", l.name);
            // NULL out-params must be tolerated
            g(std::ptr::null_mut(), std::ptr::null_mut());

            let mut m2: Option<FnMalloc> = None;
            let mut re2: Option<FnRealloc> = None;
            let mut f2: Option<FnFree> = None;
            g2(&mut m2, &mut re2, &mut f2);
            assert!(m2.is_some(), "{}: malloc2 fn", l.name);
            assert!(re2.is_some(), "{}: realloc2 fn", l.name);
            assert!(f2.is_some(), "{}: free2 fn", l.name);
            g2(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            // the defaults must be libc malloc/realloc/free
            assert_eq!(
                m.map(|x| x as *const () as usize),
                Some(libc_malloc as *const () as usize),
                "{}: default malloc is libc malloc",
                l.name
            );
            assert_eq!(
                f.map(|x| x as *const () as usize),
                Some(libc_free as *const () as usize),
                "{}: default free is libc free",
                l.name
            );
            assert_eq!(
                re2.map(|x| x as *const () as usize),
                Some(libc_realloc as *const () as usize),
                "{}: default realloc is libc realloc",
                l.name
            );

            let p = (m2.unwrap())(16);
            assert!(!p.is_null());
            (f2.unwrap())(p);
        }
    }
}

static CUSTOM_HITS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn custom_malloc(n: usize) -> *mut c_void {
    CUSTOM_HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    libc_malloc(n)
}
unsafe extern "C" fn custom_free(p: *mut c_void) {
    libc_free(p)
}
unsafe extern "C" fn custom_malloc2(n: usize) -> *mut c_void {
    CUSTOM_HITS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    libc_malloc(n)
}
unsafe extern "C" fn custom_realloc2(p: *mut c_void, n: usize) -> *mut c_void {
    libc_realloc(p, n)
}
unsafe extern "C" fn custom_free2(p: *mut c_void) {
    libc_free(p)
}

extern "C" {
    #[link_name = "malloc"]
    fn libc_malloc(n: usize) -> *mut c_void;
    #[link_name = "realloc"]
    fn libc_realloc(p: *mut c_void, n: usize) -> *mut c_void;
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

#[test]
fn set_alloc_funcs_round_trip() {
    let _g = LOCK.lock().unwrap();
    let (c, r) = libs();
    for l in [c, r] {
        let set: Symbol<FnSetAlloc> = l.sym("json_set_alloc_funcs");
        let get: Symbol<FnGetAlloc> = l.sym("json_get_alloc_funcs");
        let set2: Symbol<FnSetAlloc2> = l.sym("json_set_alloc_funcs2");
        let get2: Symbol<FnGetAlloc2> = l.sym("json_get_alloc_funcs2");
        unsafe {
            let mut om: Option<FnMalloc> = None;
            let mut ore: Option<FnRealloc> = None;
            let mut of: Option<FnFree> = None;
            get2(&mut om, &mut ore, &mut of);

            set(Some(custom_malloc), Some(custom_free));
            let mut m: Option<FnMalloc> = None;
            let mut f: Option<FnFree> = None;
            let mut re: Option<FnRealloc> = Some(libc_realloc);
            get(&mut m, &mut f);
            assert_eq!(
                m.map(|x| x as *const () as usize),
                Some(custom_malloc as *const () as usize),
                "{}: set/get malloc",
                l.name
            );
            assert_eq!(
                f.map(|x| x as *const () as usize),
                Some(custom_free as *const () as usize),
                "{}: set/get free",
                l.name
            );
            // json_set_alloc_funcs clears do_realloc
            get2(std::ptr::null_mut(), &mut re, std::ptr::null_mut());
            assert!(
                re.is_none(),
                "{}: json_set_alloc_funcs must NULL do_realloc",
                l.name
            );

            let before = CUSTOM_HITS.load(std::sync::atomic::Ordering::SeqCst);
            let jm: Symbol<FnMalloc> = l.sym("jsonp_malloc");
            let jf: Symbol<FnFree> = l.sym("jsonp_free");
            let jre: Symbol<unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void> =
                l.sym("jsonp_realloc");
            let p = jm(8);
            assert!(
                CUSTOM_HITS.load(std::sync::atomic::Ordering::SeqCst) > before,
                "{}: custom malloc used",
                l.name
            );
            // realloc emulation path (do_realloc == NULL)
            let p = p as *mut u8;
            for i in 0..8 {
                *p.add(i) = 0xA0 | i as u8;
            }
            let p2 = jre(p as *mut c_void, 8, 16) as *mut u8;
            assert!(!p2.is_null());
            for i in 0..8 {
                assert_eq!(*p2.add(i), 0xA0 | i as u8, "{}: emulated realloc", l.name);
            }
            // newSize == 0 through the emulation path frees and returns NULL
            assert!(jre(p2 as *mut c_void, 16, 0).is_null());
            assert!(jre(std::ptr::null_mut(), 0, 0).is_null());
            jf(std::ptr::null_mut());

            set2(
                Some(custom_malloc2),
                Some(custom_realloc2),
                Some(custom_free2),
            );
            let mut m2: Option<FnMalloc> = None;
            let mut re2: Option<FnRealloc> = None;
            let mut f2: Option<FnFree> = None;
            get2(&mut m2, &mut re2, &mut f2);
            assert_eq!(
                m2.map(|x| x as *const () as usize),
                Some(custom_malloc2 as *const () as usize)
            );
            assert_eq!(
                re2.map(|x| x as *const () as usize),
                Some(custom_realloc2 as *const () as usize)
            );
            assert_eq!(
                f2.map(|x| x as *const () as usize),
                Some(custom_free2 as *const () as usize)
            );

            let p = jm(8);
            let p = jre(p, 8, 32);
            jf(p);

            // restore
            set2(om, ore, of);
        }
    }
}

