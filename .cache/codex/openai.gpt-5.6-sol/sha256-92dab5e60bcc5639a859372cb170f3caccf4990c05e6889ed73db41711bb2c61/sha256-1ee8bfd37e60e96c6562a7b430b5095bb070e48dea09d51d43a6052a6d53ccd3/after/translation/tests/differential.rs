use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::ptr;

const JSON_REJECT_DUPLICATES: usize = 0x1;
const JSON_DISABLE_EOF_CHECK: usize = 0x2;
const JSON_DECODE_ANY: usize = 0x4;
const JSON_DECODE_INT_AS_REAL: usize = 0x8;
const JSON_ALLOW_NUL: usize = 0x10;
const JSON_COMPACT: usize = 0x20;
const JSON_ENSURE_ASCII: usize = 0x40;
const JSON_SORT_KEYS: usize = 0x80;
const JSON_PRESERVE_ORDER: usize = 0x100;
const JSON_ENCODE_ANY: usize = 0x200;
const JSON_ESCAPE_SLASH: usize = 0x400;
const JSON_EMBED: usize = 0x10000;
const JSON_STRICT: usize = 0x2;
const JSON_VALIDATE_ONLY: usize = 0x1;

#[repr(C)]
#[derive(Clone, Copy)]
struct JsonT {
    type_: c_int,
    refcount: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct JsonError {
    line: c_int,
    column: c_int,
    position: c_int,
    source: [c_char; 80],
    text: [c_char; 160],
}

impl JsonError {
    fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }

    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.line.to_ne_bytes());
        out.extend_from_slice(&self.column.to_ne_bytes());
        out.extend_from_slice(&self.position.to_ne_bytes());
        out.extend(self.source.iter().map(|v| *v as u8));
        out.extend(self.text.iter().map(|v| *v as u8));
        out
    }

    fn code(&self) -> u8 {
        self.text[159] as u8
    }
}

#[repr(C)]
struct StrBuffer {
    value: *mut c_char,
    length: usize,
    size: usize,
}

#[repr(C)]
struct List {
    prev: *mut List,
    next: *mut List,
}

#[repr(C)]
struct Bucket {
    first: *mut List,
    last: *mut List,
}

#[repr(C)]
struct Hashtable {
    size: usize,
    buckets: *mut Bucket,
    order: usize,
    list: List,
    ordered_list: List,
}

struct Api {
    lib: Library,
}

impl Api {
    unsafe fn open(path: &Path) -> Self {
        Self {
            lib: unsafe { Library::new(path) }.unwrap(),
        }
    }

    unsafe fn sym<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.lib.get::<T>(name) }.unwrap()
    }

    unsafe fn load_bytes(&self, bytes: &[u8], flags: usize) -> (*mut JsonT, JsonError) {
        let f: unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut JsonT =
            unsafe { self.sym(b"json_loadb\0") };
        let mut error = JsonError::zeroed();
        let value = unsafe { f(bytes.as_ptr().cast(), bytes.len(), flags, &mut error) };
        (value, error)
    }

    unsafe fn dump(&self, value: *const JsonT, flags: usize) -> Option<Vec<u8>> {
        let dumps: unsafe extern "C" fn(*const JsonT, usize) -> *mut c_char =
            unsafe { self.sym(b"json_dumps\0") };
        let free: unsafe extern "C" fn(*mut c_void) = unsafe { self.sym(b"jsonp_free\0") };
        let raw = unsafe { dumps(value, flags) };
        if raw.is_null() {
            return None;
        }
        let result = unsafe { CStr::from_ptr(raw) }.to_bytes().to_vec();
        unsafe { free(raw.cast()) };
        Some(result)
    }

    unsafe fn delete(&self, value: *mut JsonT) {
        if !value.is_null() {
            let f: unsafe extern "C" fn(*mut JsonT) = unsafe { self.sym(b"json_delete\0") };
            unsafe { f(value) };
        }
    }
}

fn paths() -> (PathBuf, PathBuf) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        root.join("../c_src/build/libjansson.so"),
        root.join("target/release/libjansson.so"),
    )
}

unsafe fn apis() -> (Api, Api) {
    let (c, r) = paths();
    assert!(c.is_file(), "missing C shared library: {}", c.display());
    assert!(r.is_file(), "missing Rust shared library: {}", r.display());
    (unsafe { Api::open(&c) }, unsafe { Api::open(&r) })
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn random_json(state: &mut u64, depth: usize) -> String {
    if depth == 0 {
        return match lcg(state) % 6 {
            0 => "null".into(),
            1 => (lcg(state) as i64).to_string(),
            2 => format!("{}.{}", lcg(state) % 100_000, lcg(state) % 10_000),
            3 => (lcg(state) & 1 == 0).to_string(),
            4 => format!("\"s{}\\/é😀\"", lcg(state) % 10_000),
            _ => "\"\\u0001\\n\\t\"".into(),
        };
    }
    match lcg(state) % 3 {
        0 => {
            let count = (lcg(state) % 5) as usize;
            let values = (0..count)
                .map(|_| random_json(state, depth - 1))
                .collect::<Vec<_>>();
            format!("[{}]", values.join(","))
        }
        1 => {
            let count = (lcg(state) % 5) as usize;
            let values = (0..count)
                .map(|i| format!("\"k{i}\":{}", random_json(state, depth - 1)))
                .collect::<Vec<_>>();
            format!("{{{}}}", values.join(","))
        }
        _ => random_json(state, 0),
    }
}

#[test]
fn every_c_export_resolves_from_both_shared_objects() {
    unsafe {
        let (c, r) = apis();
        let symbols = include_str!("../SYMBOLS.md");
        let mut count = 0;
        for line in symbols.lines() {
            let Some(rest) = line.strip_prefix("| ") else {
                continue;
            };
            if !(rest.starts_with("B | ") || rest.starts_with("D | ") || rest.starts_with("T | ")) {
                continue;
            }
            let name = rest.split('`').nth(1).unwrap();
            let mut nul = name.as_bytes().to_vec();
            nul.push(0);
            let _: *mut c_void = c.sym(&nul);
            let _: *mut c_void = r.sym(&nul);
            count += 1;
        }
        assert_eq!(count, 130);
    }
}

unsafe fn compare_utf_and_numeric(c: &Api, r: &Api) {
    let cv: unsafe extern "C" fn() -> *const c_char = unsafe { c.sym(b"jansson_version_str\0") };
    let rv: unsafe extern "C" fn() -> *const c_char = unsafe { r.sym(b"jansson_version_str\0") };
    assert_eq!(
        unsafe { CStr::from_ptr(cv()) }.to_bytes(),
        unsafe { CStr::from_ptr(rv()) }.to_bytes()
    );
    let cc: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int =
        unsafe { c.sym(b"jansson_version_cmp\0") };
    let rc: unsafe extern "C" fn(c_int, c_int, c_int) -> c_int =
        unsafe { r.sym(b"jansson_version_cmp\0") };
    for v in [(2, 15, 0), (1, 99, 99), (3, 0, 0), (2, 14, 9), (2, 15, 1)] {
        assert_eq!(unsafe { cc(v.0, v.1, v.2) }, unsafe { rc(v.0, v.1, v.2) });
    }

    let ce: unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int =
        unsafe { c.sym(b"utf8_encode\0") };
    let re: unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int =
        unsafe { r.sym(b"utf8_encode\0") };
    for cp in [
        -1, 0, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xffff, 0x10000, 0x10ffff, 0x110000,
    ] {
        let (mut cb, mut rb) = ([0i8; 8], [0i8; 8]);
        let (mut cs, mut rs) = (99usize, 99usize);
        let cr = unsafe { ce(cp, cb.as_mut_ptr(), &mut cs) };
        let rr = unsafe { re(cp, rb.as_mut_ptr(), &mut rs) };
        assert_eq!((cr, cs, cb), (rr, rs, rb));
    }

    let cfirst: unsafe extern "C" fn(c_char) -> usize = unsafe { c.sym(b"utf8_check_first\0") };
    let rfirst: unsafe extern "C" fn(c_char) -> usize = unsafe { r.sym(b"utf8_check_first\0") };
    for byte in 0u16..=255 {
        assert_eq!(unsafe { cfirst(byte as u8 as c_char) }, unsafe {
            rfirst(byte as u8 as c_char)
        });
    }

    let cfull: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize =
        unsafe { c.sym(b"utf8_check_full\0") };
    let rfull: unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize =
        unsafe { r.sym(b"utf8_check_full\0") };
    let cases: &[&[u8]] = &[
        b"\xc2\xa2",
        b"\xe2\x82\xac",
        b"\xf0\x9f\x98\x80",
        b"\xc0\x80",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"\xe2\x28\xa1",
    ];
    for bytes in cases {
        for size in 0..=5 {
            let (mut co, mut ro) = (-7, -7);
            assert_eq!(
                unsafe { cfull(bytes.as_ptr().cast(), size, &mut co) },
                unsafe { rfull(bytes.as_ptr().cast(), size, &mut ro) }
            );
            assert_eq!(co, ro);
        }
    }

    let cstr: unsafe extern "C" fn(*const c_char, usize) -> c_int =
        unsafe { c.sym(b"utf8_check_string\0") };
    let rstr: unsafe extern "C" fn(*const c_char, usize) -> c_int =
        unsafe { r.sym(b"utf8_check_string\0") };
    for bytes in [
        b"".as_slice(),
        b"ascii",
        "é€😀".as_bytes(),
        b"\x80",
        b"\xe2\x82",
        b"\xed\xa0\x80",
    ] {
        assert_eq!(
            unsafe { cstr(bytes.as_ptr().cast(), bytes.len()) },
            unsafe { rstr(bytes.as_ptr().cast(), bytes.len()) }
        );
    }

    let cdt: unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int =
        unsafe { c.sym(b"jsonp_dtostr\0") };
    let rdt: unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int =
        unsafe { r.sym(b"jsonp_dtostr\0") };
    let mut seed = 0x51ed_cafe_d00d_f00d;
    for precision in 0..=31 {
        for value in [
            -0.0,
            0.0,
            1.0,
            -1.25,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            1e-300,
            1e300,
            f64::from_bits(lcg(&mut seed)),
        ] {
            if !value.is_finite() {
                continue;
            }
            for size in [0, 4, 8, 25, 64] {
                let (mut cb, mut rb) = ([0i8; 64], [0i8; 64]);
                let cr = unsafe { cdt(cb.as_mut_ptr(), size, value, precision) };
                let rr = unsafe { rdt(rb.as_mut_ptr(), size, value, precision) };
                assert_eq!(cr, rr);
                assert_eq!(cb, rb);
            }
        }
    }

    let cdtoa: unsafe extern "C" fn(
        c_double,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
        *mut c_char,
        usize,
    ) -> *mut c_char = unsafe { c.sym(b"dtoa_r\0") };
    let rdtoa: unsafe extern "C" fn(
        c_double,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
        *mut c_char,
        usize,
    ) -> *mut c_char = unsafe { r.sym(b"dtoa_r\0") };
    for mode in 0..=9 {
        for digits in [-5, 0, 1, 6, 17, 30] {
            for value in [-0.0, 0.0, -1.5, 1e-300, 1e300, f64::INFINITY, f64::NAN] {
                let (mut cb, mut rb) = ([0i8; 512], [0i8; 512]);
                let (mut cdp, mut rdp, mut csgn, mut rsgn) = (0, 0, 0, 0);
                let (mut cend, mut rend) = (ptr::null_mut(), ptr::null_mut());
                let cp = unsafe {
                    cdtoa(
                        value,
                        mode,
                        digits,
                        &mut cdp,
                        &mut csgn,
                        &mut cend,
                        cb.as_mut_ptr(),
                        cb.len(),
                    )
                };
                let rp = unsafe {
                    rdtoa(
                        value,
                        mode,
                        digits,
                        &mut rdp,
                        &mut rsgn,
                        &mut rend,
                        rb.as_mut_ptr(),
                        rb.len(),
                    )
                };
                assert_eq!(cp.is_null(), rp.is_null());
                assert_eq!((cdp, csgn), (rdp, rsgn));
                if !cp.is_null() {
                    assert_eq!(
                        unsafe { CStr::from_ptr(cp) }.to_bytes(),
                        unsafe { CStr::from_ptr(rp) }.to_bytes()
                    );
                    assert_eq!(unsafe { cend.offset_from(cp) }, unsafe {
                        rend.offset_from(rp)
                    });
                }
            }
        }
    }
}

unsafe fn compare_buffers_and_hashtables(c: &Api, r: &Api) {
    type Init = unsafe extern "C" fn(*mut StrBuffer) -> c_int;
    type Append = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
    type AppendByte = unsafe extern "C" fn(*mut StrBuffer, c_char) -> c_int;
    type Pop = unsafe extern "C" fn(*mut StrBuffer) -> c_char;
    type Clear = unsafe extern "C" fn(*mut StrBuffer);
    type Close = unsafe extern "C" fn(*mut StrBuffer);
    let ci: Init = unsafe { c.sym(b"strbuffer_init\0") };
    let ri: Init = unsafe { r.sym(b"strbuffer_init\0") };
    let ca: Append = unsafe { c.sym(b"strbuffer_append_bytes\0") };
    let ra: Append = unsafe { r.sym(b"strbuffer_append_bytes\0") };
    let cab: AppendByte = unsafe { c.sym(b"strbuffer_append_byte\0") };
    let rab: AppendByte = unsafe { r.sym(b"strbuffer_append_byte\0") };
    let cp: Pop = unsafe { c.sym(b"strbuffer_pop\0") };
    let rp: Pop = unsafe { r.sym(b"strbuffer_pop\0") };
    let cclear: Clear = unsafe { c.sym(b"strbuffer_clear\0") };
    let rclear: Clear = unsafe { r.sym(b"strbuffer_clear\0") };
    let cclose: Close = unsafe { c.sym(b"strbuffer_close\0") };
    let rclose: Close = unsafe { r.sym(b"strbuffer_close\0") };
    let (mut cs, mut rs): (StrBuffer, StrBuffer) = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { ci(&mut cs) }, unsafe { ri(&mut rs) });
    for bytes in [b"".as_slice(), b"a", &[b'x'; 63], &[b'y'; 300]] {
        assert_eq!(
            unsafe { ca(&mut cs, bytes.as_ptr().cast(), bytes.len()) },
            unsafe { ra(&mut rs, bytes.as_ptr().cast(), bytes.len()) }
        );
        assert_eq!(cs.length, rs.length);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(cs.value.cast::<u8>(), cs.length + 1) },
            unsafe { std::slice::from_raw_parts(rs.value.cast::<u8>(), rs.length + 1) }
        );
    }
    assert_eq!(unsafe { cab(&mut cs, b'!' as c_char) }, unsafe {
        rab(&mut rs, b'!' as c_char)
    });
    assert_eq!(unsafe { cp(&mut cs) }, unsafe { rp(&mut rs) });
    unsafe { cclear(&mut cs) };
    unsafe { rclear(&mut rs) };
    assert_eq!(
        (cs.length, unsafe { *cs.value }),
        (rs.length, unsafe { *rs.value })
    );
    unsafe { cclose(&mut cs) };
    unsafe { rclose(&mut rs) };

    type HInit = unsafe extern "C" fn(*mut Hashtable) -> c_int;
    type HClose = unsafe extern "C" fn(*mut Hashtable);
    type HSet = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize, *mut JsonT) -> c_int;
    type HGet = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut c_void;
    type HDel = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> c_int;
    type HIter = unsafe extern "C" fn(*mut Hashtable) -> *mut c_void;
    type HNext = unsafe extern "C" fn(*mut Hashtable, *mut c_void) -> *mut c_void;
    let hi_c: HInit = unsafe { c.sym(b"hashtable_init\0") };
    let hi_r: HInit = unsafe { r.sym(b"hashtable_init\0") };
    let hc_c: HClose = unsafe { c.sym(b"hashtable_close\0") };
    let hc_r: HClose = unsafe { r.sym(b"hashtable_close\0") };
    let hs_c: HSet = unsafe { c.sym(b"hashtable_set\0") };
    let hs_r: HSet = unsafe { r.sym(b"hashtable_set\0") };
    let hg_c: HGet = unsafe { c.sym(b"hashtable_get\0") };
    let hg_r: HGet = unsafe { r.sym(b"hashtable_get\0") };
    let hd_c: HDel = unsafe { c.sym(b"hashtable_del\0") };
    let hd_r: HDel = unsafe { r.sym(b"hashtable_del\0") };
    let hit_c: HIter = unsafe { c.sym(b"hashtable_iter\0") };
    let hit_r: HIter = unsafe { r.sym(b"hashtable_iter\0") };
    let hn_c: HNext = unsafe { c.sym(b"hashtable_iter_next\0") };
    let hn_r: HNext = unsafe { r.sym(b"hashtable_iter_next\0") };
    let jint_c: unsafe extern "C" fn(i64) -> *mut JsonT = unsafe { c.sym(b"json_integer\0") };
    let jint_r: unsafe extern "C" fn(i64) -> *mut JsonT = unsafe { r.sym(b"json_integer\0") };
    let (mut cht, mut rht): (Hashtable, Hashtable) = unsafe { std::mem::zeroed() };
    assert_eq!(unsafe { hi_c(&mut cht) }, unsafe { hi_r(&mut rht) });
    for i in 0..80 {
        let key = format!("k{i:03}");
        assert_eq!(
            unsafe { hs_c(&mut cht, key.as_ptr().cast(), key.len(), jint_c(i),) },
            unsafe { hs_r(&mut rht, key.as_ptr().cast(), key.len(), jint_r(i),) }
        );
    }
    assert_eq!(cht.size, rht.size);
    for key in ["k000", "k040", "k079", "missing"] {
        assert_eq!(
            unsafe { hg_c(&mut cht, key.as_ptr().cast(), key.len()) }.is_null(),
            unsafe { hg_r(&mut rht, key.as_ptr().cast(), key.len()) }.is_null()
        );
    }
    let (mut ci, mut ri) = (unsafe { hit_c(&mut cht) }, unsafe { hit_r(&mut rht) });
    let (mut cn, mut rn) = (0, 0);
    while !ci.is_null() {
        cn += 1;
        ci = unsafe { hn_c(&mut cht, ci) };
    }
    while !ri.is_null() {
        rn += 1;
        ri = unsafe { hn_r(&mut rht, ri) };
    }
    assert_eq!(cn, rn);
    for key in ["k000", "k040", "missing"] {
        assert_eq!(
            unsafe { hd_c(&mut cht, key.as_ptr().cast(), key.len()) },
            unsafe { hd_r(&mut rht, key.as_ptr().cast(), key.len()) }
        );
    }
    unsafe { hc_c(&mut cht) };
    unsafe { hc_r(&mut rht) };
}

unsafe fn compare_random_json(c: &Api, r: &Api) {
    let flags = [
        0,
        1,
        2,
        31,
        JSON_COMPACT,
        JSON_ENSURE_ASCII,
        JSON_SORT_KEYS,
        JSON_PRESERVE_ORDER,
        JSON_ESCAPE_SLASH,
        JSON_EMBED,
        JSON_COMPACT | JSON_ENSURE_ASCII | JSON_SORT_KEYS | JSON_ESCAPE_SLASH,
        (6 << 11) | JSON_SORT_KEYS,
        (31 << 11) | JSON_COMPACT | JSON_ENSURE_ASCII,
    ];
    let mut seed = 0x5eed_1234_9876_abcd;
    for _ in 0..300 {
        let text = random_json(&mut seed, 4);
        let top = if text.starts_with('{') || text.starts_with('[') {
            text
        } else {
            format!("[{text}]")
        };
        let (cv, ce) = unsafe { c.load_bytes(top.as_bytes(), 0) };
        let (rv, re) = unsafe { r.load_bytes(top.as_bytes(), 0) };
        assert_eq!(cv.is_null(), rv.is_null(), "{top}");
        assert_eq!(ce.bytes(), re.bytes(), "{top}");
        assert!(!cv.is_null(), "{top}");
        for flag in flags {
            assert_eq!(
                unsafe { c.dump(cv, flag) },
                unsafe { r.dump(rv, flag) },
                "input={top} flags={flag:#x}"
            );
        }

        let copy_c: unsafe extern "C" fn(*mut JsonT) -> *mut JsonT =
            unsafe { c.sym(b"json_copy\0") };
        let copy_r: unsafe extern "C" fn(*mut JsonT) -> *mut JsonT =
            unsafe { r.sym(b"json_copy\0") };
        let deep_c: unsafe extern "C" fn(*const JsonT) -> *mut JsonT =
            unsafe { c.sym(b"json_deep_copy\0") };
        let deep_r: unsafe extern "C" fn(*const JsonT) -> *mut JsonT =
            unsafe { r.sym(b"json_deep_copy\0") };
        let eq_c: unsafe extern "C" fn(*const JsonT, *const JsonT) -> c_int =
            unsafe { c.sym(b"json_equal\0") };
        let eq_r: unsafe extern "C" fn(*const JsonT, *const JsonT) -> c_int =
            unsafe { r.sym(b"json_equal\0") };
        let ccopy = unsafe { copy_c(cv) };
        let rcopy = unsafe { copy_r(rv) };
        let cdeep = unsafe { deep_c(cv) };
        let rdeep = unsafe { deep_r(rv) };
        assert_eq!(unsafe { c.dump(ccopy, JSON_SORT_KEYS) }, unsafe {
            r.dump(rcopy, JSON_SORT_KEYS)
        });
        assert_eq!(unsafe { c.dump(cdeep, JSON_SORT_KEYS) }, unsafe {
            r.dump(rdeep, JSON_SORT_KEYS)
        });
        assert_eq!(unsafe { eq_c(cv, cdeep) }, unsafe { eq_r(rv, rdeep) });
        unsafe { c.delete(ccopy) };
        unsafe { r.delete(rcopy) };
        unsafe { c.delete(cdeep) };
        unsafe { r.delete(rdeep) };
        unsafe { c.delete(cv) };
        unsafe { r.delete(rv) };
    }

    let scalar_cases = [
        ("\"x\"", 0),
        ("123", 0),
        ("123", JSON_DECODE_INT_AS_REAL),
        ("true", 0),
        ("null", 0),
        ("\"a\\u0000b\"", JSON_ALLOW_NUL),
        ("[1] trailing", JSON_DISABLE_EOF_CHECK),
    ];
    for (text, extra) in scalar_cases {
        let flags = JSON_DECODE_ANY | extra;
        let (cv, ce) = unsafe { c.load_bytes(text.as_bytes(), flags) };
        let (rv, re) = unsafe { r.load_bytes(text.as_bytes(), flags) };
        assert_eq!(cv.is_null(), rv.is_null());
        assert_eq!(ce.bytes(), re.bytes());
        assert_eq!(
            unsafe { c.dump(cv, JSON_ENCODE_ANY | JSON_ENSURE_ASCII) },
            unsafe { r.dump(rv, JSON_ENCODE_ANY | JSON_ENSURE_ASCII) }
        );
        unsafe { c.delete(cv) };
        unsafe { r.delete(rv) };
    }
}

#[repr(C)]
struct CallbackInput {
    bytes: Vec<u8>,
    pos: usize,
    chunk: usize,
}

unsafe extern "C" fn load_callback(buffer: *mut c_void, len: usize, data: *mut c_void) -> usize {
    let state = unsafe { &mut *data.cast::<CallbackInput>() };
    if state.pos == state.bytes.len() {
        return 0;
    }
    let count = state.chunk.min(len).min(state.bytes.len() - state.pos);
    unsafe {
        ptr::copy_nonoverlapping(
            state.bytes.as_ptr().add(state.pos),
            buffer.cast::<u8>(),
            count,
        )
    };
    state.pos += count;
    count
}

unsafe extern "C" fn dump_callback(buffer: *const c_char, len: usize, data: *mut c_void) -> c_int {
    let out = unsafe { &mut *data.cast::<Vec<u8>>() };
    out.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer.cast(), len) });
    0
}

unsafe extern "C" fn rejecting_dump_callback(
    _buffer: *const c_char,
    _len: usize,
    _data: *mut c_void,
) -> c_int {
    -1
}

unsafe fn compare_callbacks_and_files(c: &Api, r: &Api) {
    type LoadCb = unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
        *mut c_void,
        usize,
        *mut JsonError,
    ) -> *mut JsonT;
    type DumpCb = unsafe extern "C" fn(
        *const JsonT,
        Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
        *mut c_void,
        usize,
    ) -> c_int;
    let lc: LoadCb = unsafe { c.sym(b"json_load_callback\0") };
    let lr: LoadCb = unsafe { r.sym(b"json_load_callback\0") };
    let dc: DumpCb = unsafe { c.sym(b"json_dump_callback\0") };
    let dr: DumpCb = unsafe { r.sym(b"json_dump_callback\0") };
    let document = r#"{"z":[1,2,3],"s":"é😀/"}"#.as_bytes().to_vec();
    for chunk in [1, 7, 1024] {
        let mut ci = CallbackInput {
            bytes: document.clone(),
            pos: 0,
            chunk,
        };
        let mut ri = CallbackInput {
            bytes: document.clone(),
            pos: 0,
            chunk,
        };
        let (mut ce, mut re) = (JsonError::zeroed(), JsonError::zeroed());
        let cv = unsafe {
            lc(
                Some(load_callback),
                (&mut ci as *mut CallbackInput).cast(),
                0,
                &mut ce,
            )
        };
        let rv = unsafe {
            lr(
                Some(load_callback),
                (&mut ri as *mut CallbackInput).cast(),
                0,
                &mut re,
            )
        };
        assert_eq!(ce.bytes(), re.bytes());
        let (mut co, mut ro) = (Vec::new(), Vec::new());
        assert_eq!(
            unsafe {
                dc(
                    cv,
                    Some(dump_callback),
                    (&mut co as *mut Vec<u8>).cast(),
                    JSON_SORT_KEYS | JSON_COMPACT,
                )
            },
            unsafe {
                dr(
                    rv,
                    Some(dump_callback),
                    (&mut ro as *mut Vec<u8>).cast(),
                    JSON_SORT_KEYS | JSON_COMPACT,
                )
            }
        );
        assert_eq!(co, ro);
        assert_eq!(
            unsafe { dc(cv, Some(rejecting_dump_callback), ptr::null_mut(), 0) },
            unsafe { dr(rv, Some(rejecting_dump_callback), ptr::null_mut(), 0) }
        );
        unsafe { c.delete(cv) };
        unsafe { r.delete(rv) };
    }

    let base = std::env::temp_dir().join(format!("jansson-diff-{}", std::process::id()));
    fs::create_dir_all(&base).unwrap();
    let input_c = base.join("input-c.json");
    let input_r = base.join("input-r.json");
    fs::write(&input_c, &document).unwrap();
    fs::write(&input_r, &document).unwrap();
    type LoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut JsonT;
    let lfc: LoadFile = unsafe { c.sym(b"json_load_file\0") };
    let lfr: LoadFile = unsafe { r.sym(b"json_load_file\0") };
    let cpath = CString::new(input_c.to_str().unwrap()).unwrap();
    let rpath = CString::new(input_r.to_str().unwrap()).unwrap();
    let (mut ce, mut re) = (JsonError::zeroed(), JsonError::zeroed());
    let cv = unsafe { lfc(cpath.as_ptr(), 0, &mut ce) };
    let rv = unsafe { lfr(rpath.as_ptr(), 0, &mut re) };
    assert_eq!(ce.code(), re.code());
    type DumpFile = unsafe extern "C" fn(*const JsonT, *const c_char, usize) -> c_int;
    let dfc: DumpFile = unsafe { c.sym(b"json_dump_file\0") };
    let dfr: DumpFile = unsafe { r.sym(b"json_dump_file\0") };
    let output_c = base.join("output-c.json");
    let output_r = base.join("output-r.json");
    let cop = CString::new(output_c.to_str().unwrap()).unwrap();
    let rop = CString::new(output_r.to_str().unwrap()).unwrap();
    assert_eq!(
        unsafe { dfc(cv, cop.as_ptr(), JSON_SORT_KEYS | 2) },
        unsafe { dfr(rv, rop.as_ptr(), JSON_SORT_KEYS | 2) }
    );
    assert_eq!(fs::read(output_c).unwrap(), fs::read(output_r).unwrap());

    type LoadFd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut JsonT;
    let fd_c = File::open(&input_c).unwrap();
    let fd_r = File::open(&input_r).unwrap();
    let loadfd_c: LoadFd = unsafe { c.sym(b"json_loadfd\0") };
    let loadfd_r: LoadFd = unsafe { r.sym(b"json_loadfd\0") };
    let cv2 = unsafe { loadfd_c(fd_c.as_raw_fd(), 0, &mut ce) };
    let rv2 = unsafe { loadfd_r(fd_r.as_raw_fd(), 0, &mut re) };
    assert_eq!(unsafe { c.dump(cv2, JSON_SORT_KEYS) }, unsafe {
        r.dump(rv2, JSON_SORT_KEYS)
    });
    unsafe { c.delete(cv2) };
    unsafe { r.delete(rv2) };
    unsafe { c.delete(cv) };
    unsafe { r.delete(rv) };
    fs::remove_dir_all(base).unwrap();
}

unsafe fn compare_variadics(c: &Api, r: &Api) {
    type Pack = unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT;
    type PackEx = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, ...) -> *mut JsonT;
    type UnpackEx =
        unsafe extern "C" fn(*mut JsonT, *mut JsonError, usize, *const c_char, ...) -> c_int;
    type Sprintf = unsafe extern "C" fn(*const c_char, ...) -> *mut JsonT;
    let pc: Pack = unsafe { c.sym(b"json_pack\0") };
    let pr: Pack = unsafe { r.sym(b"json_pack\0") };
    let pxc: PackEx = unsafe { c.sym(b"json_pack_ex\0") };
    let pxr: PackEx = unsafe { r.sym(b"json_pack_ex\0") };
    let uc: UnpackEx = unsafe { c.sym(b"json_unpack_ex\0") };
    let ur: UnpackEx = unsafe { r.sym(b"json_unpack_ex\0") };
    let sc: Sprintf = unsafe { c.sym(b"json_sprintf\0") };
    let sr: Sprintf = unsafe { r.sym(b"json_sprintf\0") };

    let fmt = CString::new("{s:i,s:s,s:[b,f,n]}").unwrap();
    let k1 = CString::new("i").unwrap();
    let k2 = CString::new("s").unwrap();
    let k3 = CString::new("a").unwrap();
    let text = CString::new("héllo").unwrap();
    let cv = unsafe {
        pc(
            fmt.as_ptr(),
            k1.as_ptr(),
            42i32,
            k2.as_ptr(),
            text.as_ptr(),
            k3.as_ptr(),
            1i32,
            1.25f64,
        )
    };
    let rv = unsafe {
        pr(
            fmt.as_ptr(),
            k1.as_ptr(),
            42i32,
            k2.as_ptr(),
            text.as_ptr(),
            k3.as_ptr(),
            1i32,
            1.25f64,
        )
    };
    assert_eq!(unsafe { c.dump(cv, JSON_SORT_KEYS) }, unsafe {
        r.dump(rv, JSON_SORT_KEYS)
    });

    let ufmt = CString::new("{s:i,s:s%,s:[b,F,n]!}!").unwrap();
    let (mut ci, mut ri) = (0i32, 0i32);
    let (mut cs, mut rs) = (ptr::null::<c_char>(), ptr::null::<c_char>());
    let (mut csl, mut rsl) = (0usize, 0usize);
    let (mut cb, mut rb) = (0i32, 0i32);
    let (mut cf, mut rf) = (0f64, 0f64);
    let (mut ce, mut re) = (JsonError::zeroed(), JsonError::zeroed());
    let cr = unsafe {
        uc(
            cv,
            &mut ce,
            0,
            ufmt.as_ptr(),
            k1.as_ptr(),
            &mut ci,
            k2.as_ptr(),
            &mut cs,
            &mut csl,
            k3.as_ptr(),
            &mut cb,
            &mut cf,
        )
    };
    let rr = unsafe {
        ur(
            rv,
            &mut re,
            0,
            ufmt.as_ptr(),
            k1.as_ptr(),
            &mut ri,
            k2.as_ptr(),
            &mut rs,
            &mut rsl,
            k3.as_ptr(),
            &mut rb,
            &mut rf,
        )
    };
    assert_eq!((cr, ci, csl, cb, cf), (rr, ri, rsl, rb, rf));
    assert_eq!(ce.bytes(), re.bytes());
    assert_eq!(
        unsafe { CStr::from_ptr(cs) }.to_bytes(),
        unsafe { CStr::from_ptr(rs) }.to_bytes()
    );

    let validate = CString::new("{s:i,s:s,s:[b,F,n]}").unwrap();
    assert_eq!(
        unsafe {
            uc(
                cv,
                &mut ce,
                JSON_VALIDATE_ONLY | JSON_STRICT,
                validate.as_ptr(),
                k1.as_ptr(),
                k2.as_ptr(),
                k3.as_ptr(),
            )
        },
        unsafe {
            ur(
                rv,
                &mut re,
                JSON_VALIDATE_ONLY | JSON_STRICT,
                validate.as_ptr(),
                k1.as_ptr(),
                k2.as_ptr(),
                k3.as_ptr(),
            )
        }
    );

    let bad_fmt = CString::new("{").unwrap();
    assert_eq!(
        unsafe { pxc(&mut ce, 0, bad_fmt.as_ptr()) }.is_null(),
        unsafe { pxr(&mut re, 0, bad_fmt.as_ptr()) }.is_null()
    );
    assert_eq!(ce.bytes(), re.bytes());

    let sfmt = CString::new("%s:%d:%.2f").unwrap();
    let word = CString::new("é").unwrap();
    let csv = unsafe { sc(sfmt.as_ptr(), word.as_ptr(), 7i32, 1.25f64) };
    let rsv = unsafe { sr(sfmt.as_ptr(), word.as_ptr(), 7i32, 1.25f64) };
    assert_eq!(
        unsafe { c.dump(csv, JSON_ENCODE_ANY | JSON_ENSURE_ASCII) },
        unsafe { r.dump(rsv, JSON_ENCODE_ANY | JSON_ENSURE_ASCII) }
    );
    unsafe { c.delete(csv) };
    unsafe { r.delete(rsv) };
    unsafe { c.delete(cv) };
    unsafe { r.delete(rv) };
}

unsafe fn compare_errors(c: &Api, r: &Api) {
    let invalid_inputs: &[(&[u8], usize)] = &[
        (b"", 0),
        (b"null", 0),
        (b"{", 0),
        (b"[1,", 0),
        (b"{\"a\":1,\"a\":2}", JSON_REJECT_DUPLICATES),
        (b"{\"a\\u0000b\":1}", 0),
        (b"[\"a\\u0000b\"]", 0),
        (b"[\"\\q\"]", 0),
        (b"[\"\\ud800\"]", 0),
        (b"[\"\\udc00\"]", 0),
        (b"[\"\\ud800\\u0041\"]", 0),
        (b"[9223372036854775808]", 0),
        (b"[-9223372036854775809]", 0),
        (b"[1e10000]", 0),
        (b"[01]", 0),
        (b"[1.] ", 0),
        (b"[1e+]", 0),
        (b"[true] garbage", 0),
        (b"[\x80]", 0),
        (b"[\xe2\x82]", 0),
    ];
    for (bytes, flags) in invalid_inputs {
        let (cv, ce) = unsafe { c.load_bytes(bytes, *flags) };
        let (rv, re) = unsafe { r.load_bytes(bytes, *flags) };
        assert_eq!(cv.is_null(), rv.is_null(), "{bytes:?}");
        assert_eq!(ce.bytes(), re.bytes(), "{bytes:?}");
        unsafe { c.delete(cv) };
        unsafe { r.delete(rv) };
    }

    type LoadB = unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut JsonT;
    let lbc: LoadB = unsafe { c.sym(b"json_loadb\0") };
    let lbr: LoadB = unsafe { r.sym(b"json_loadb\0") };
    let (mut ce, mut re) = (JsonError::zeroed(), JsonError::zeroed());
    assert_eq!(
        unsafe { lbc(ptr::null(), 0, 0, &mut ce) }.is_null(),
        unsafe { lbr(ptr::null(), 0, 0, &mut re) }.is_null()
    );
    assert_eq!(ce.bytes(), re.bytes());

    let fake = JsonT {
        type_: 12345,
        refcount: usize::MAX,
    };
    let copy_c: unsafe extern "C" fn(*mut JsonT) -> *mut JsonT = unsafe { c.sym(b"json_copy\0") };
    let copy_r: unsafe extern "C" fn(*mut JsonT) -> *mut JsonT = unsafe { r.sym(b"json_copy\0") };
    assert_eq!(
        unsafe { copy_c((&fake as *const JsonT).cast_mut()) }.is_null(),
        unsafe { copy_r((&fake as *const JsonT).cast_mut()) }.is_null()
    );
    assert_eq!(unsafe { c.dump(&fake, JSON_ENCODE_ANY) }, unsafe {
        r.dump(&fake, JSON_ENCODE_ANY)
    });

    let real_c: unsafe extern "C" fn(c_double) -> *mut JsonT = unsafe { c.sym(b"json_real\0") };
    let real_r: unsafe extern "C" fn(c_double) -> *mut JsonT = unsafe { r.sym(b"json_real\0") };
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            unsafe { real_c(value) }.is_null(),
            unsafe { real_r(value) }.is_null()
        );
    }

    let null_checks: &[(&[u8], unsafe fn(&Api) -> bool)] = &[
        (b"json_object_get\0", |api| unsafe {
            let f: unsafe extern "C" fn(*const JsonT, *const c_char) -> *mut JsonT =
                api.sym(b"json_object_get\0");
            f(ptr::null(), ptr::null()).is_null()
        }),
        (b"json_array_get\0", |api| unsafe {
            let f: unsafe extern "C" fn(*const JsonT, usize) -> *mut JsonT =
                api.sym(b"json_array_get\0");
            f(ptr::null(), usize::MAX).is_null()
        }),
        (b"json_string\0", |api| unsafe {
            let f: unsafe extern "C" fn(*const c_char) -> *mut JsonT = api.sym(b"json_string\0");
            f(ptr::null()).is_null()
        }),
    ];
    for (_name, check) in null_checks {
        assert_eq!(unsafe { check(c) }, unsafe { check(r) });
    }
}

#[test]
fn differential_surface_and_error_paths() {
    unsafe {
        let (c, r) = apis();
        compare_utf_and_numeric(&c, &r);
        compare_buffers_and_hashtables(&c, &r);
        compare_random_json(&c, &r);
        compare_callbacks_and_files(&c, &r);
        compare_variadics(&c, &r);
        compare_errors(&c, &r);
    }
}
