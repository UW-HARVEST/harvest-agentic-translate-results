use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::fs;
use std::mem::{MaybeUninit, size_of};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::ptr;

const DECODE_ANY: usize = 0x4;
const REJECT_DUPLICATES: usize = 0x1;
const DISABLE_EOF_CHECK: usize = 0x2;
const DECODE_INT_AS_REAL: usize = 0x8;
const ALLOW_NUL: usize = 0x10;
const COMPACT: usize = 0x20;
const ENSURE_ASCII: usize = 0x40;
const SORT_KEYS: usize = 0x80;
const ENCODE_ANY: usize = 0x200;
const ESCAPE_SLASH: usize = 0x400;
const EMBED: usize = 0x10000;

#[repr(C)]
struct Json {
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
    fn blank() -> Self {
        Self {
            line: 0,
            column: 0,
            position: 0,
            source: [0; 80],
            text: [0; 160],
        }
    }

    fn code(&self) -> u8 {
        self.text[159] as u8
    }

    fn comparable(&self) -> (c_int, c_int, c_int, Vec<u8>, Vec<u8>, u8) {
        let source = self
            .source
            .iter()
            .map(|value| *value as u8)
            .take_while(|value| *value != 0)
            .collect();
        let text = self
            .text
            .iter()
            .map(|value| *value as u8)
            .take(159)
            .take_while(|value| *value != 0)
            .collect();
        (
            self.line,
            self.column,
            self.position,
            source,
            text,
            self.code(),
        )
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
struct Hashtable {
    size: usize,
    buckets: *mut c_void,
    order: usize,
    list: List,
    ordered_list: List,
}

struct Api {
    lib: Library,
}

impl Api {
    unsafe fn open(path: PathBuf) -> Self {
        Self {
            lib: unsafe { Library::new(path).unwrap() },
        }
    }

    unsafe fn sym<T: Copy>(&self, name: &[u8]) -> T {
        *unsafe { self.lib.get::<T>(name).unwrap() }
    }

    unsafe fn load(&self, bytes: &[u8], flags: usize) -> (*mut Json, JsonError) {
        type F = unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut Json;
        let mut error = JsonError::blank();
        let value = unsafe {
            self.sym::<F>(b"json_loadb\0")(bytes.as_ptr().cast(), bytes.len(), flags, &mut error)
        };
        (value, error)
    }

    unsafe fn dump(&self, value: *const Json, flags: usize) -> Vec<u8> {
        type F = unsafe extern "C" fn(*const Json, *mut c_char, usize, usize) -> usize;
        let dump = unsafe { self.sym::<F>(b"json_dumpb\0") };
        let length = unsafe { dump(value, ptr::null_mut(), 0, flags) };
        let mut bytes = vec![0u8; length];
        let actual = unsafe { dump(value, bytes.as_mut_ptr().cast(), bytes.len(), flags) };
        assert_eq!(actual, length);
        bytes
    }

    unsafe fn delete(&self, value: *mut Json) {
        type F = unsafe extern "C" fn(*mut Json);
        if !value.is_null() {
            unsafe { self.sym::<F>(b"json_delete\0")(value) };
        }
    }
}

fn apis() -> (Api, Api) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    unsafe {
        (
            Api::open(root.join("../c_src/build/libjansson.so")),
            Api::open(root.join("target/release/libjansson.so")),
        )
    }
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

unsafe fn compare_document(
    c: &Api,
    rust: &Api,
    input: &[u8],
    load_flags: usize,
    dump_flags: usize,
) {
    let (cv, ce) = unsafe { c.load(input, load_flags) };
    let (rv, re) = unsafe { rust.load(input, load_flags) };
    assert_eq!(cv.is_null(), rv.is_null(), "input={input:?}");
    assert_eq!(ce.comparable(), re.comparable(), "input={input:?}");
    if !cv.is_null() {
        assert_eq!(
            unsafe { c.dump(cv, dump_flags) },
            unsafe { rust.dump(rv, dump_flags) },
            "input={input:?}, flags={dump_flags:#x}"
        );
        unsafe {
            c.delete(cv);
            rust.delete(rv);
        }
    }
}

#[test]
fn dynamic_symbol_surface_matches() {
    let (c, rust) = apis();
    let symbols = include_str!("../rust_exports.txt");
    assert_eq!(symbols.lines().count(), 130);
    for name in symbols.lines() {
        let mut nul = name.as_bytes().to_vec();
        nul.push(0);
        unsafe {
            c.lib
                .get::<*mut c_void>(&nul)
                .unwrap_or_else(|_| panic!("C missing {name}"));
            rust.lib
                .get::<*mut c_void>(&nul)
                .unwrap_or_else(|_| panic!("Rust missing {name}"));
        }
    }
}

#[test]
fn randomized_parse_dump_flag_matrix_is_byte_identical() {
    let (c, rust) = apis();
    let mut seed = 0x4a41_4e53_534f_4e15;
    let dump_flags = [
        0,
        COMPACT,
        1,
        4,
        31,
        ENSURE_ASCII,
        ESCAPE_SLASH,
        SORT_KEYS,
        SORT_KEYS | COMPACT | ENSURE_ASCII | ESCAPE_SLASH,
        1 << 11,
        6 << 11,
        17 << 11,
        31 << 11,
        EMBED,
    ];
    for index in 0..160 {
        let a = lcg(&mut seed) as i64;
        let b = f64::from_bits((lcg(&mut seed) >> 12) | 0x3ff0_0000_0000_0000);
        let count = (lcg(&mut seed) % 9) as usize;
        let values = (0..count)
            .map(|_| (lcg(&mut seed) as i32).to_string())
            .collect::<Vec<_>>()
            .join(",");
        let input = format!(
            "{{\"z{index}\":{a},\"slash\":\"a/b\",\"utf\":\"é𝄞\",\"real\":{b:.17},\"items\":[{values}],\"bool\":true,\"nil\":null}}"
        );
        for flags in dump_flags {
            unsafe { compare_document(&c, &rust, input.as_bytes(), 0, flags) };
        }
    }
}

#[test]
fn parser_configuration_matrix_is_identical() {
    let (c, rust) = apis();
    let cases: &[(&[u8], usize)] = &[
        (b"0", DECODE_ANY),
        (b"-9223372036854775808", DECODE_ANY),
        (b"9223372036854775807", DECODE_ANY),
        (b"123", DECODE_ANY | DECODE_INT_AS_REAL),
        (b"\"a\\u0000b\"", DECODE_ANY | ALLOW_NUL),
        (b"{\"a\":1,\"a\":2}", 0),
        (b"{\"a\":1,\"a\":2}", REJECT_DUPLICATES),
        (b"[] trailing", DISABLE_EOF_CHECK),
        (b" [ true, false, null, -0.0, 1e-300 ] ", 0),
    ];
    for (input, flags) in cases {
        unsafe { compare_document(&c, &rust, input, *flags, ENCODE_ANY | SORT_KEYS) };
    }

    for depth in [1, 2, 32, 2048, 2049] {
        let input = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        unsafe { compare_document(&c, &rust, input.as_bytes(), 0, 0) };
    }
}

#[test]
fn constructors_getters_setters_and_containers_match() {
    type New = unsafe extern "C" fn() -> *mut Json;
    type Int = unsafe extern "C" fn(i64) -> *mut Json;
    type Real = unsafe extern "C" fn(c_double) -> *mut Json;
    type StrN = unsafe extern "C" fn(*const c_char, usize) -> *mut Json;
    type SetObj = unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int;
    type Append = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type Insert = unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int;
    type Remove = unsafe extern "C" fn(*mut Json, usize) -> c_int;
    type Size = unsafe extern "C" fn(*const Json) -> usize;

    unsafe fn exercise(api: &Api) -> Vec<u8> {
        let object = unsafe { api.sym::<New>(b"json_object\0")() };
        let array = unsafe { api.sym::<New>(b"json_array\0")() };
        let mut seed = 0x5eed_cafe_f00d_u64;
        for index in 0..80 {
            let key = format!("k{index:03}");
            let value = unsafe { api.sym::<Int>(b"json_integer\0")(lcg(&mut seed) as i64) };
            assert_eq!(
                unsafe {
                    api.sym::<SetObj>(b"json_object_setn_new\0")(
                        object,
                        key.as_ptr().cast(),
                        key.len(),
                        value,
                    )
                },
                0
            );
            let item = if index % 3 == 0 {
                unsafe { api.sym::<Real>(b"json_real\0")(index as f64 + 0.25) }
            } else {
                let bytes = format!("v{index}é");
                unsafe { api.sym::<StrN>(b"json_stringn\0")(bytes.as_ptr().cast(), bytes.len()) }
            };
            assert_eq!(
                unsafe { api.sym::<Append>(b"json_array_append_new\0")(array, item) },
                0
            );
        }
        assert_eq!(
            unsafe { api.sym::<Size>(b"json_object_size\0")(object) },
            80
        );
        assert_eq!(unsafe { api.sym::<Size>(b"json_array_size\0")(array) }, 80);
        assert_eq!(
            unsafe {
                api.sym::<Insert>(b"json_array_insert_new\0")(
                    array,
                    0,
                    api.sym::<Int>(b"json_integer\0")(-7),
                )
            },
            0
        );
        assert_eq!(
            unsafe { api.sym::<Remove>(b"json_array_remove\0")(array, 40) },
            0
        );
        assert_eq!(
            unsafe {
                api.sym::<SetObj>(b"json_object_setn_new_nocheck\0")(
                    object,
                    b"array".as_ptr().cast(),
                    5,
                    array,
                )
            },
            0
        );
        let result = unsafe { api.dump(object, SORT_KEYS | COMPACT | ENSURE_ASCII) };
        unsafe { api.delete(object) };
        result
    }

    let (c, rust) = apis();
    assert_eq!(unsafe { exercise(&c) }, unsafe { exercise(&rust) });
}

#[test]
fn low_level_utf_buffer_numeric_and_hashtable_match() {
    type UtfEncode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
    type UtfFirst = unsafe extern "C" fn(c_char) -> usize;
    type UtfFull = unsafe extern "C" fn(*const c_char, usize, *mut i32) -> usize;
    type Dtostr = unsafe extern "C" fn(*mut c_char, usize, c_double, c_int) -> c_int;
    type DtoaR = unsafe extern "C" fn(
        c_double,
        c_int,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut *mut c_char,
        *mut c_char,
        usize,
    ) -> *mut c_char;
    type SbInit = unsafe extern "C" fn(*mut StrBuffer) -> c_int;
    type SbAppend = unsafe extern "C" fn(*mut StrBuffer, *const c_char, usize) -> c_int;
    type SbPop = unsafe extern "C" fn(*mut StrBuffer) -> c_char;
    type SbVoid = unsafe extern "C" fn(*mut StrBuffer);
    type HInit = unsafe extern "C" fn(*mut Hashtable) -> c_int;
    type HSet = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize, *mut Json) -> c_int;
    type HGet = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> *mut Json;
    type HDel = unsafe extern "C" fn(*mut Hashtable, *const c_char, usize) -> c_int;
    type HVoid = unsafe extern "C" fn(*mut Hashtable);
    type Null = unsafe extern "C" fn() -> *mut Json;

    let (c, rust) = apis();
    for codepoint in [
        -1, 0, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xffff, 0x10000, 0x10ffff, 0x110000,
    ] {
        let mut cb = [0i8; 8];
        let mut rb = [0i8; 8];
        let mut cs = 99;
        let mut rs = 99;
        let cr =
            unsafe { c.sym::<UtfEncode>(b"utf8_encode\0")(codepoint, cb.as_mut_ptr(), &mut cs) };
        let rr =
            unsafe { rust.sym::<UtfEncode>(b"utf8_encode\0")(codepoint, rb.as_mut_ptr(), &mut rs) };
        assert_eq!((cr, cs, cb), (rr, rs, rb));
    }
    for byte in 0u8..=255 {
        assert_eq!(
            unsafe { c.sym::<UtfFirst>(b"utf8_check_first\0")(byte as c_char) },
            unsafe { rust.sym::<UtfFirst>(b"utf8_check_first\0")(byte as c_char) }
        );
    }
    for bytes in [
        b"\xc2\x80".as_slice(),
        b"\xe2\x82\xac",
        b"\xf4\x8f\xbf\xbf",
        b"\xed\xa0\x80",
    ] {
        let mut cc = -1;
        let mut rc = -1;
        assert_eq!(
            unsafe {
                c.sym::<UtfFull>(b"utf8_check_full\0")(bytes.as_ptr().cast(), bytes.len(), &mut cc)
            },
            unsafe {
                rust.sym::<UtfFull>(b"utf8_check_full\0")(
                    bytes.as_ptr().cast(),
                    bytes.len(),
                    &mut rc,
                )
            }
        );
        assert_eq!(cc, rc);
    }

    let mut seed = 0x1234_5678_9abc_def0;
    for precision in [0, 1, 6, 17, 31] {
        for _ in 0..100 {
            let value = f64::from_bits(lcg(&mut seed));
            if !value.is_finite() {
                continue;
            }
            let mut cb = [0i8; 128];
            let mut rb = [0i8; 128];
            let cr = unsafe {
                c.sym::<Dtostr>(b"jsonp_dtostr\0")(cb.as_mut_ptr(), cb.len(), value, precision)
            };
            let rr = unsafe {
                rust.sym::<Dtostr>(b"jsonp_dtostr\0")(rb.as_mut_ptr(), rb.len(), value, precision)
            };
            assert_eq!(cr, rr);
            assert_eq!(cb, rb);
            for mode in 0..=9 {
                let (mut cdec, mut rdec, mut csign, mut rsign) = (0, 0, 0, 0);
                let (mut cend, mut rend) = (ptr::null_mut(), ptr::null_mut());
                let mut cbuf = [0i8; 128];
                let mut rbuf = [0i8; 128];
                let cp = unsafe {
                    c.sym::<DtoaR>(b"dtoa_r\0")(
                        value,
                        mode,
                        precision,
                        &mut cdec,
                        &mut csign,
                        &mut cend,
                        cbuf.as_mut_ptr(),
                        cbuf.len(),
                    )
                };
                let rp = unsafe {
                    rust.sym::<DtoaR>(b"dtoa_r\0")(
                        value,
                        mode,
                        precision,
                        &mut rdec,
                        &mut rsign,
                        &mut rend,
                        rbuf.as_mut_ptr(),
                        rbuf.len(),
                    )
                };
                assert_eq!(cp.is_null(), rp.is_null());
                assert_eq!((cdec, csign), (rdec, rsign));
                assert_eq!(cbuf, rbuf);
            }
        }
    }

    unsafe fn buffer_result(api: &Api) -> Vec<u8> {
        let mut buffer = MaybeUninit::<StrBuffer>::uninit();
        assert_eq!(
            unsafe { api.sym::<SbInit>(b"strbuffer_init\0")(buffer.as_mut_ptr()) },
            0
        );
        let mut buffer = unsafe { buffer.assume_init() };
        for part in [b"".as_slice(), b"a", b"0123456789abcdef", b"tail"] {
            assert_eq!(
                unsafe {
                    api.sym::<SbAppend>(b"strbuffer_append_bytes\0")(
                        &mut buffer,
                        part.as_ptr().cast(),
                        part.len(),
                    )
                },
                0
            );
        }
        let popped = unsafe { api.sym::<SbPop>(b"strbuffer_pop\0")(&mut buffer) };
        let bytes = unsafe { std::slice::from_raw_parts(buffer.value.cast::<u8>(), buffer.length) }
            .to_vec();
        unsafe { api.sym::<SbVoid>(b"strbuffer_close\0")(&mut buffer) };
        [bytes, vec![popped as u8]].concat()
    }
    assert_eq!(unsafe { buffer_result(&c) }, unsafe {
        buffer_result(&rust)
    });

    unsafe fn hash_result(api: &Api) -> (Vec<bool>, Vec<c_int>) {
        let mut table = MaybeUninit::<Hashtable>::uninit();
        assert_eq!(
            unsafe { api.sym::<HInit>(b"hashtable_init\0")(table.as_mut_ptr()) },
            0
        );
        let table = unsafe { &mut *table.as_mut_ptr() };
        for index in 0..40i64 {
            let key = format!("key-{index:03}");
            let value = unsafe { api.sym::<Null>(b"json_null\0")() };
            assert_eq!(
                unsafe {
                    api.sym::<HSet>(b"hashtable_set\0")(
                        table,
                        key.as_ptr().cast(),
                        key.len(),
                        value,
                    )
                },
                0
            );
        }
        let mut result = Vec::new();
        let mut deletions = Vec::new();
        for index in 0..40i64 {
            let key = format!("key-{index:03}");
            let value = unsafe {
                api.sym::<HGet>(b"hashtable_get\0")(table, key.as_ptr().cast(), key.len())
            };
            result.push(!value.is_null());
            if index % 3 == 0 {
                deletions.push(unsafe {
                    api.sym::<HDel>(b"hashtable_del\0")(table, key.as_ptr().cast(), key.len())
                });
            }
        }
        unsafe { api.sym::<HVoid>(b"hashtable_close\0")(table) };
        (result, deletions)
    }
    let c_hash = unsafe { hash_result(&c) };
    let rust_hash = unsafe { hash_result(&rust) };
    assert_eq!(c_hash, rust_hash);
}

unsafe extern "C" fn collect_dump(data: *const c_char, length: usize, state: *mut c_void) -> c_int {
    let output = unsafe { &mut *state.cast::<Vec<u8>>() };
    output.extend_from_slice(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), length) });
    0
}

struct LoadState {
    bytes: Vec<u8>,
    position: usize,
    chunk: usize,
}

unsafe extern "C" fn provide_load(buffer: *mut c_void, length: usize, state: *mut c_void) -> usize {
    let state = unsafe { &mut *state.cast::<LoadState>() };
    let count = state
        .chunk
        .min(length)
        .min(state.bytes.len().saturating_sub(state.position));
    unsafe {
        ptr::copy_nonoverlapping(
            state.bytes.as_ptr().add(state.position),
            buffer.cast(),
            count,
        )
    };
    state.position += count;
    count
}

#[test]
fn callback_file_fd_and_variadic_entry_points_match() {
    type LoadCb = unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
        *mut c_void,
        usize,
        *mut JsonError,
    ) -> *mut Json;
    type DumpCb = unsafe extern "C" fn(
        *const Json,
        Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
        *mut c_void,
        usize,
    ) -> c_int;
    type LoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json;
    type DumpFile = unsafe extern "C" fn(*const Json, *const c_char, usize) -> c_int;
    type LoadFd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut Json;
    type DumpFd = unsafe extern "C" fn(*const Json, c_int, usize) -> c_int;
    type PackI = unsafe extern "C" fn(*const c_char, c_int) -> *mut Json;
    type PackS = unsafe extern "C" fn(*const c_char, *const c_char) -> *mut Json;
    type SprintfI = unsafe extern "C" fn(*const c_char, c_int) -> *mut Json;
    type UnpackI = unsafe extern "C" fn(*mut Json, *const c_char, *mut c_int) -> c_int;

    let (c, rust) = apis();
    let document = r#"{"callback":[1,2,3],"utf":"é"}"#.as_bytes();
    for chunk in [1, 2, 7, 1024] {
        let mut cs = LoadState {
            bytes: document.to_vec(),
            position: 0,
            chunk,
        };
        let mut rs = LoadState {
            bytes: document.to_vec(),
            position: 0,
            chunk,
        };
        let mut ce = JsonError::blank();
        let mut re = JsonError::blank();
        let cv = unsafe {
            c.sym::<LoadCb>(b"json_load_callback\0")(
                Some(provide_load),
                (&mut cs as *mut LoadState).cast(),
                0,
                &mut ce,
            )
        };
        let rv = unsafe {
            rust.sym::<LoadCb>(b"json_load_callback\0")(
                Some(provide_load),
                (&mut rs as *mut LoadState).cast(),
                0,
                &mut re,
            )
        };
        assert_eq!(ce.comparable(), re.comparable());
        let mut co = Vec::new();
        let mut ro = Vec::new();
        assert_eq!(
            unsafe {
                c.sym::<DumpCb>(b"json_dump_callback\0")(
                    cv,
                    Some(collect_dump),
                    (&mut co as *mut Vec<u8>).cast(),
                    SORT_KEYS,
                )
            },
            0
        );
        assert_eq!(
            unsafe {
                rust.sym::<DumpCb>(b"json_dump_callback\0")(
                    rv,
                    Some(collect_dump),
                    (&mut ro as *mut Vec<u8>).cast(),
                    SORT_KEYS,
                )
            },
            0
        );
        assert_eq!(co, ro);
        unsafe {
            c.delete(cv);
            rust.delete(rv)
        };
    }

    let temp = std::env::temp_dir().join(format!("jansson-diff-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let input = temp.join("input.json");
    fs::write(&input, document).unwrap();
    let input_c = CString::new(input.to_str().unwrap()).unwrap();
    let mut ce = JsonError::blank();
    let mut re = JsonError::blank();
    let cv = unsafe { c.sym::<LoadFile>(b"json_load_file\0")(input_c.as_ptr(), 0, &mut ce) };
    let rv = unsafe { rust.sym::<LoadFile>(b"json_load_file\0")(input_c.as_ptr(), 0, &mut re) };
    assert_eq!(unsafe { c.dump(cv, SORT_KEYS) }, unsafe {
        rust.dump(rv, SORT_KEYS)
    });
    let cout = CString::new(temp.join("c.json").to_str().unwrap()).unwrap();
    let rout = CString::new(temp.join("r.json").to_str().unwrap()).unwrap();
    assert_eq!(
        unsafe { c.sym::<DumpFile>(b"json_dump_file\0")(cv, cout.as_ptr(), COMPACT) },
        0
    );
    assert_eq!(
        unsafe { rust.sym::<DumpFile>(b"json_dump_file\0")(rv, rout.as_ptr(), COMPACT) },
        0
    );
    assert_eq!(
        fs::read(cout.to_str().unwrap()).unwrap(),
        fs::read(rout.to_str().unwrap()).unwrap()
    );

    let cf = fs::File::open(&input).unwrap();
    let rf = fs::File::open(&input).unwrap();
    let cfdv = unsafe { c.sym::<LoadFd>(b"json_loadfd\0")(cf.as_raw_fd(), 0, &mut ce) };
    let rfdv = unsafe { rust.sym::<LoadFd>(b"json_loadfd\0")(rf.as_raw_fd(), 0, &mut re) };
    let cfdout = fs::File::create(temp.join("c-fd.json")).unwrap();
    let rfdout = fs::File::create(temp.join("r-fd.json")).unwrap();
    assert_eq!(
        unsafe { c.sym::<DumpFd>(b"json_dumpfd\0")(cfdv, cfdout.as_raw_fd(), COMPACT) },
        0
    );
    assert_eq!(
        unsafe { rust.sym::<DumpFd>(b"json_dumpfd\0")(rfdv, rfdout.as_raw_fd(), COMPACT) },
        0
    );
    drop((cfdout, rfdout));
    assert_eq!(
        fs::read(temp.join("c-fd.json")).unwrap(),
        fs::read(temp.join("r-fd.json")).unwrap()
    );

    let fmt_i = c"i";
    let cpacked = unsafe { c.sym::<PackI>(b"json_pack\0")(fmt_i.as_ptr(), -12345) };
    let rpacked = unsafe { rust.sym::<PackI>(b"json_pack\0")(fmt_i.as_ptr(), -12345) };
    assert_eq!(unsafe { c.dump(cpacked, ENCODE_ANY) }, unsafe {
        rust.dump(rpacked, ENCODE_ANY)
    });
    let text = c"formatted";
    let cstr = unsafe { c.sym::<PackS>(b"json_pack\0")(c"s".as_ptr(), text.as_ptr()) };
    let rstr = unsafe { rust.sym::<PackS>(b"json_pack\0")(c"s".as_ptr(), text.as_ptr()) };
    assert_eq!(unsafe { c.dump(cstr, ENCODE_ANY) }, unsafe {
        rust.dump(rstr, ENCODE_ANY)
    });
    let csprintf = unsafe { c.sym::<SprintfI>(b"json_sprintf\0")(c"value=%d".as_ptr(), 42) };
    let rsprintf = unsafe { rust.sym::<SprintfI>(b"json_sprintf\0")(c"value=%d".as_ptr(), 42) };
    assert_eq!(unsafe { c.dump(csprintf, ENCODE_ANY) }, unsafe {
        rust.dump(rsprintf, ENCODE_ANY)
    });
    let (mut ci, mut ri) = (0, 0);
    assert_eq!(
        unsafe { c.sym::<UnpackI>(b"json_unpack\0")(cpacked, fmt_i.as_ptr(), &mut ci) },
        0
    );
    assert_eq!(
        unsafe { rust.sym::<UnpackI>(b"json_unpack\0")(rpacked, fmt_i.as_ptr(), &mut ri) },
        0
    );
    assert_eq!(ci, ri);

    unsafe {
        for value in [cv, cfdv, cpacked, cstr, csprintf] {
            c.delete(value);
        }
        for value in [rv, rfdv, rpacked, rstr, rsprintf] {
            rust.delete(value);
        }
    }
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn extended_public_api_state_transitions_match() {
    type Version = unsafe extern "C" fn() -> *const c_char;
    type VersionCmp = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
    type New = unsafe extern "C" fn() -> *mut Json;
    type Int = unsafe extern "C" fn(i64) -> *mut Json;
    type Real = unsafe extern "C" fn(f64) -> *mut Json;
    type StringN = unsafe extern "C" fn(*const c_char, usize) -> *mut Json;
    type SetObject = unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int;
    type Update = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type Iter = unsafe extern "C" fn(*mut Json) -> *mut c_void;
    type IterNext = unsafe extern "C" fn(*mut Json, *mut c_void) -> *mut c_void;
    type IterKey = unsafe extern "C" fn(*mut c_void) -> *const c_char;
    type IterKeyLen = unsafe extern "C" fn(*mut c_void) -> usize;
    type IterValue = unsafe extern "C" fn(*mut c_void) -> *mut Json;
    type IterSet = unsafe extern "C" fn(*mut Json, *mut c_void, *mut Json) -> c_int;
    type Append = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type ArraySet = unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int;
    type ArrayExtend = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type ArrayClear = unsafe extern "C" fn(*mut Json) -> c_int;
    type StringSet = unsafe extern "C" fn(*mut Json, *const c_char, usize) -> c_int;
    type IntSet = unsafe extern "C" fn(*mut Json, i64) -> c_int;
    type IntGet = unsafe extern "C" fn(*const Json) -> i64;
    type RealSet = unsafe extern "C" fn(*mut Json, f64) -> c_int;
    type RealGet = unsafe extern "C" fn(*const Json) -> f64;
    type Copy = unsafe extern "C" fn(*mut Json) -> *mut Json;
    type DeepCopy = unsafe extern "C" fn(*const Json) -> *mut Json;
    type Dumps = unsafe extern "C" fn(*const Json, usize) -> *mut c_char;
    type Free = unsafe extern "C" fn(*mut c_void);

    unsafe fn exercise(api: &Api) -> (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<i64>, Vec<u64>) {
        let version = unsafe { CStr::from_ptr(api.sym::<Version>(b"jansson_version_str\0")()) }
            .to_bytes()
            .to_vec();
        assert_eq!(version, b"2.15.0");
        for (input, expected_sign) in [
            ((1, 99, 99), 1),
            ((2, 14, 99), 1),
            ((2, 15, 0), 0),
            ((2, 15, 1), -1),
            ((3, 0, 0), -1),
        ] {
            let actual = unsafe {
                api.sym::<VersionCmp>(b"jansson_version_cmp\0")(input.0, input.1, input.2)
            };
            assert_eq!(actual.signum(), expected_sign);
        }

        let destination = unsafe { api.sym::<New>(b"json_object\0")() };
        let source = unsafe { api.sym::<New>(b"json_object\0")() };
        for (key, value) in [(c"a", 1), (c"shared", 2)] {
            assert_eq!(
                unsafe {
                    api.sym::<SetObject>(b"json_object_set_new\0")(
                        destination,
                        key.as_ptr(),
                        api.sym::<Int>(b"json_integer\0")(value),
                    )
                },
                0
            );
        }
        for (key, value) in [(c"shared", 20), (c"b", 30)] {
            assert_eq!(
                unsafe {
                    api.sym::<SetObject>(b"json_object_set_new\0")(
                        source,
                        key.as_ptr(),
                        api.sym::<Int>(b"json_integer\0")(value),
                    )
                },
                0
            );
        }

        let mut snapshots = Vec::new();
        let copy = unsafe { api.sym::<Copy>(b"json_copy\0")(destination) };
        assert_eq!(
            unsafe { api.sym::<Update>(b"json_object_update\0")(copy, source) },
            0
        );
        snapshots.push(unsafe { api.dump(copy, SORT_KEYS | COMPACT) });
        unsafe { api.delete(copy) };

        let copy = unsafe { api.sym::<Copy>(b"json_copy\0")(destination) };
        assert_eq!(
            unsafe { api.sym::<Update>(b"json_object_update_existing\0")(copy, source) },
            0
        );
        snapshots.push(unsafe { api.dump(copy, SORT_KEYS | COMPACT) });
        unsafe { api.delete(copy) };

        let copy = unsafe { api.sym::<Copy>(b"json_copy\0")(destination) };
        assert_eq!(
            unsafe { api.sym::<Update>(b"json_object_update_missing\0")(copy, source) },
            0
        );
        snapshots.push(unsafe { api.dump(copy, SORT_KEYS | COMPACT) });
        unsafe { api.delete(copy) };

        let nested_left = unsafe { api.load(br#"{"n":{"x":1},"keep":0}"#, 0).0 };
        let nested_right = unsafe { api.load(br#"{"n":{"y":2},"keep":{"z":3}}"#, 0).0 };
        assert_eq!(
            unsafe {
                api.sym::<Update>(b"json_object_update_recursive\0")(nested_left, nested_right)
            },
            0
        );
        snapshots.push(unsafe { api.dump(nested_left, SORT_KEYS | COMPACT) });

        let mut keys = Vec::new();
        let mut iter = unsafe { api.sym::<Iter>(b"json_object_iter\0")(destination) };
        while !iter.is_null() {
            let key = unsafe { api.sym::<IterKey>(b"json_object_iter_key\0")(iter) };
            let length = unsafe { api.sym::<IterKeyLen>(b"json_object_iter_key_len\0")(iter) };
            keys.push(unsafe { std::slice::from_raw_parts(key.cast::<u8>(), length) }.to_vec());
            assert!(!unsafe { api.sym::<IterValue>(b"json_object_iter_value\0")(iter) }.is_null());
            iter = unsafe { api.sym::<IterNext>(b"json_object_iter_next\0")(destination, iter) };
        }
        let first = unsafe { api.sym::<Iter>(b"json_object_iter\0")(destination) };
        assert_eq!(
            unsafe {
                api.sym::<IterSet>(b"json_object_iter_set_new\0")(
                    destination,
                    first,
                    api.sym::<Int>(b"json_integer\0")(999),
                )
            },
            0
        );
        snapshots.push(unsafe { api.dump(destination, SORT_KEYS | COMPACT) });

        let array = unsafe { api.sym::<New>(b"json_array\0")() };
        let other = unsafe { api.sym::<New>(b"json_array\0")() };
        for value in 0..5 {
            assert_eq!(
                unsafe {
                    api.sym::<Append>(b"json_array_append_new\0")(
                        array,
                        api.sym::<Int>(b"json_integer\0")(value),
                    )
                },
                0
            );
        }
        for value in 5..9 {
            assert_eq!(
                unsafe {
                    api.sym::<Append>(b"json_array_append_new\0")(
                        other,
                        api.sym::<Int>(b"json_integer\0")(value),
                    )
                },
                0
            );
        }
        assert_eq!(
            unsafe {
                api.sym::<ArraySet>(b"json_array_set_new\0")(
                    array,
                    2,
                    api.sym::<Int>(b"json_integer\0")(222),
                )
            },
            0
        );
        assert_eq!(
            unsafe { api.sym::<ArrayExtend>(b"json_array_extend\0")(array, other) },
            0
        );
        snapshots.push(unsafe { api.dump(array, COMPACT) });
        assert_eq!(
            unsafe { api.sym::<ArrayClear>(b"json_array_clear\0")(other) },
            0
        );

        let string =
            unsafe { api.sym::<StringN>(b"json_stringn\0")(b"initial".as_ptr().cast(), 7) };
        assert_eq!(
            unsafe {
                api.sym::<StringSet>(b"json_string_setn\0")(
                    string,
                    "replaced-é".as_ptr().cast(),
                    "replaced-é".len(),
                )
            },
            0
        );
        snapshots.push(unsafe { api.dump(string, ENCODE_ANY | ENSURE_ASCII) });

        let integer = unsafe { api.sym::<Int>(b"json_integer\0")(0) };
        let real = unsafe { api.sym::<Real>(b"json_real\0")(0.0) };
        let mut ints = Vec::new();
        let mut reals = Vec::new();
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            assert_eq!(
                unsafe { api.sym::<IntSet>(b"json_integer_set\0")(integer, value) },
                0
            );
            ints.push(unsafe { api.sym::<IntGet>(b"json_integer_value\0")(integer) });
        }
        for value in [-0.0, f64::MIN_POSITIVE, -1.25, f64::MAX] {
            assert_eq!(
                unsafe { api.sym::<RealSet>(b"json_real_set\0")(real, value) },
                0
            );
            reals.push(unsafe { api.sym::<RealGet>(b"json_real_value\0")(real) }.to_bits());
        }

        let deep = unsafe { api.sym::<DeepCopy>(b"json_deep_copy\0")(nested_left) };
        snapshots.push(unsafe { api.dump(deep, SORT_KEYS | COMPACT) });
        let allocated = unsafe { api.sym::<Dumps>(b"json_dumps\0")(deep, SORT_KEYS | COMPACT) };
        snapshots.push(unsafe { CStr::from_ptr(allocated) }.to_bytes().to_vec());
        unsafe { api.sym::<Free>(b"jsonp_free\0")(allocated.cast()) };

        unsafe {
            for value in [
                destination,
                source,
                nested_left,
                nested_right,
                array,
                other,
                string,
                integer,
                real,
                deep,
            ] {
                api.delete(value);
            }
        }
        (snapshots, keys, ints, reals)
    }

    let (c, rust) = apis();
    assert_eq!(unsafe { exercise(&c) }, unsafe { exercise(&rust) });
}

type MallocCallback = Option<unsafe extern "C" fn(usize) -> *mut c_void>;
type ReallocCallback = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
type FreeCallback = Option<unsafe extern "C" fn(*mut c_void)>;

unsafe extern "C" fn fail_malloc(_: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn fail_realloc(_: *mut c_void, _: usize) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn ignore_free(_: *mut c_void) {}

#[test]
fn allocator_memory_and_error_helpers_match() {
    type GetAlloc =
        unsafe extern "C" fn(*mut MallocCallback, *mut ReallocCallback, *mut FreeCallback);
    type SetAlloc = unsafe extern "C" fn(MallocCallback, ReallocCallback, FreeCallback);
    type Malloc = unsafe extern "C" fn(usize) -> *mut c_void;
    type Realloc = unsafe extern "C" fn(*mut c_void, usize, usize) -> *mut c_void;
    type Free = unsafe extern "C" fn(*mut c_void);
    type Strndup = unsafe extern "C" fn(*const c_char, usize) -> *mut c_char;
    type Int = unsafe extern "C" fn(i64) -> *mut Json;
    type ErrorInit = unsafe extern "C" fn(*mut JsonError, *const c_char);
    type ErrorSource = unsafe extern "C" fn(*mut JsonError, *const c_char);
    type ErrorSet = unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char);

    unsafe fn exercise(api: &Api) -> (Vec<u8>, Vec<u8>, JsonError) {
        assert!(unsafe { api.sym::<Malloc>(b"jsonp_malloc\0")(0) }.is_null());
        let allocation = unsafe { api.sym::<Malloc>(b"jsonp_malloc\0")(8) };
        assert!(!allocation.is_null());
        let allocation = unsafe { api.sym::<Realloc>(b"jsonp_realloc\0")(allocation, 8, 64) };
        assert!(!allocation.is_null());
        unsafe { api.sym::<Free>(b"jsonp_free\0")(allocation) };
        let duplicate =
            unsafe { api.sym::<Strndup>(b"jsonp_strndup\0")(b"a\0b".as_ptr().cast(), 3) };
        let duplicated = unsafe { std::slice::from_raw_parts(duplicate.cast::<u8>(), 4) }.to_vec();
        unsafe { api.sym::<Free>(b"jsonp_free\0")(duplicate.cast()) };

        let mut old_malloc = None;
        let mut old_realloc = None;
        let mut old_free = None;
        unsafe {
            api.sym::<GetAlloc>(b"json_get_alloc_funcs2\0")(
                &mut old_malloc,
                &mut old_realloc,
                &mut old_free,
            )
        };
        assert!(old_malloc.is_some());
        assert!(old_free.is_some());
        unsafe {
            api.sym::<SetAlloc>(b"json_set_alloc_funcs2\0")(
                Some(fail_malloc),
                Some(fail_realloc),
                Some(ignore_free),
            )
        };
        assert!(unsafe { api.sym::<Int>(b"json_integer\0")(1) }.is_null());
        unsafe {
            api.sym::<SetAlloc>(b"json_set_alloc_funcs2\0")(old_malloc, old_realloc, old_free)
        };

        let mut error = JsonError::blank();
        let long_source = CString::new("x".repeat(120)).unwrap();
        unsafe {
            api.sym::<ErrorInit>(b"jsonp_error_init\0")(&mut error, c"initial".as_ptr());
            api.sym::<ErrorSource>(b"jsonp_error_set_source\0")(&mut error, long_source.as_ptr());
            api.sym::<ErrorSet>(b"jsonp_error_set\0")(&mut error, 4, 5, 6, 9, c"first".as_ptr());
            api.sym::<ErrorSet>(b"jsonp_error_set\0")(&mut error, 7, 8, 9, 10, c"second".as_ptr());
        }
        (
            duplicated,
            error
                .source
                .iter()
                .map(|value| *value as u8)
                .take_while(|value| *value != 0)
                .collect(),
            error,
        )
    }

    let (c, rust) = apis();
    let (cd, cs, ce) = unsafe { exercise(&c) };
    let (rd, rs, re) = unsafe { exercise(&rust) };
    assert_eq!((cd, cs, ce.comparable()), (rd, rs, re.comparable()));
}

#[test]
fn error_surface_returns_codes_and_sentinels_identically() {
    let (c, rust) = apis();
    let cases: &[(&[u8], usize)] = &[
        (b"", 0),
        (b"null", 0),
        (b"[", 0),
        (b"{]", 0),
        (b"{\"a\" 1}", 0),
        (b"{\"a\":1,\"a\":2}", REJECT_DUPLICATES),
        (b"\"\\q\"", DECODE_ANY),
        (b"\"\\uD800\"", DECODE_ANY),
        (b"\"\\uDC00\"", DECODE_ANY),
        (b"\"\\u0000\"", DECODE_ANY),
        (b"01", DECODE_ANY),
        (b"1.", DECODE_ANY),
        (b"1e", DECODE_ANY),
        (b"9223372036854775808", DECODE_ANY),
        (b"1e100000", DECODE_ANY),
        (b"[] x", 0),
        (b"{\"a\\u0000b\":1}", ALLOW_NUL),
        (b"\xff", DECODE_ANY),
        (b"\xc2", DECODE_ANY),
    ];
    for (input, flags) in cases {
        unsafe { compare_document(&c, &rust, input, *flags, ENCODE_ANY) };
    }

    type LoadB = unsafe extern "C" fn(*const c_char, usize, usize, *mut JsonError) -> *mut Json;
    type UnaryPtr = unsafe extern "C" fn(*const Json) -> *mut Json;
    type UnarySize = unsafe extern "C" fn(*const Json) -> usize;
    type UnaryI64 = unsafe extern "C" fn(*const Json) -> i64;
    type UnaryF64 = unsafe extern "C" fn(*const Json) -> f64;
    type Real = unsafe extern "C" fn(f64) -> *mut Json;
    type UtfEncode = unsafe extern "C" fn(i32, *mut c_char, *mut usize) -> c_int;
    type ErrorSet = unsafe extern "C" fn(*mut JsonError, c_int, c_int, usize, c_int, *const c_char);
    for api in [&c, &rust] {
        let mut error = JsonError::blank();
        assert!(
            unsafe { api.sym::<LoadB>(b"json_loadb\0")(ptr::null(), 1, 0, &mut error) }.is_null()
        );
        assert_eq!(error.code(), 4);
        assert!(unsafe { api.sym::<UnaryPtr>(b"json_copy\0")(ptr::null()) }.is_null());
        assert!(unsafe { api.sym::<UnaryPtr>(b"json_deep_copy\0")(ptr::null()) }.is_null());
        assert_eq!(
            unsafe { api.sym::<UnarySize>(b"json_object_size\0")(ptr::null()) },
            0
        );
        assert_eq!(
            unsafe { api.sym::<UnarySize>(b"json_array_size\0")(ptr::null()) },
            0
        );
        assert_eq!(
            unsafe { api.sym::<UnaryI64>(b"json_integer_value\0")(ptr::null()) },
            0
        );
        assert_eq!(
            unsafe { api.sym::<UnaryF64>(b"json_real_value\0")(ptr::null()) },
            0.0
        );
        assert!(unsafe { api.sym::<Real>(b"json_real\0")(f64::NAN) }.is_null());
        assert!(unsafe { api.sym::<Real>(b"json_real\0")(f64::INFINITY) }.is_null());
        let mut bytes = [0i8; 8];
        let mut length = 0;
        assert_eq!(
            unsafe { api.sym::<UtfEncode>(b"utf8_encode\0")(-1, bytes.as_mut_ptr(), &mut length) },
            -1
        );
        assert_eq!(
            unsafe {
                api.sym::<UtfEncode>(b"utf8_encode\0")(0x110000, bytes.as_mut_ptr(), &mut length)
            },
            -1
        );

        let mut out_of_range = JsonError::blank();
        unsafe {
            api.sym::<ErrorSet>(b"jsonp_error_set\0")(
                &mut out_of_range,
                1,
                2,
                3,
                255,
                c"bad".as_ptr(),
            )
        };
        assert_eq!(out_of_range.code(), 255);
        let fake = Json {
            type_: 99,
            refcount: 1,
        };
        assert!(unsafe { api.sym::<UnaryPtr>(b"json_copy\0")(&fake) }.is_null());
        unsafe {
            api.sym::<unsafe extern "C" fn(*mut Json)>(b"json_delete\0")(
                (&fake as *const Json).cast_mut(),
            )
        };
    }
}

#[test]
fn variadic_and_mutator_error_paths_match_exactly() {
    type Pack0 = unsafe extern "C" fn(*mut JsonError, usize, *const c_char) -> *mut Json;
    type PackS =
        unsafe extern "C" fn(*mut JsonError, usize, *const c_char, *const c_char) -> *mut Json;
    type PackF = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, f64) -> *mut Json;
    type PackI = unsafe extern "C" fn(*mut JsonError, usize, *const c_char, c_int) -> *mut Json;
    type Unpack0 = unsafe extern "C" fn(*mut Json, *mut JsonError, usize, *const c_char) -> c_int;
    type UnpackS = unsafe extern "C" fn(
        *mut Json,
        *mut JsonError,
        usize,
        *const c_char,
        *const c_char,
        *mut c_int,
    ) -> c_int;
    type UnpackOut =
        unsafe extern "C" fn(*mut Json, *mut JsonError, usize, *const c_char, *mut c_int) -> c_int;
    type UnpackStr = unsafe extern "C" fn(
        *mut Json,
        *mut JsonError,
        usize,
        *const c_char,
        *mut *const c_char,
    ) -> c_int;
    type New = unsafe extern "C" fn() -> *mut Json;
    type Int = unsafe extern "C" fn(i64) -> *mut Json;
    type StringN = unsafe extern "C" fn(*const c_char, usize) -> *mut Json;
    type SetObject = unsafe extern "C" fn(*mut Json, *const c_char, *mut Json) -> c_int;
    type SetObjectN = unsafe extern "C" fn(*mut Json, *const c_char, usize, *mut Json) -> c_int;
    type SetArray = unsafe extern "C" fn(*mut Json, usize, *mut Json) -> c_int;
    type Append = unsafe extern "C" fn(*mut Json, *mut Json) -> c_int;
    type Remove = unsafe extern "C" fn(*mut Json, usize) -> c_int;
    type StringSet = unsafe extern "C" fn(*mut Json, *const c_char, usize) -> c_int;
    type RealSet = unsafe extern "C" fn(*mut Json, f64) -> c_int;
    type Dump = unsafe extern "C" fn(
        *const Json,
        Option<unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int>,
        *mut c_void,
        usize,
    ) -> c_int;
    type LoadFd = unsafe extern "C" fn(c_int, usize, *mut JsonError) -> *mut Json;
    type LoadFile = unsafe extern "C" fn(*const c_char, usize, *mut JsonError) -> *mut Json;
    type LoadCallback = unsafe extern "C" fn(
        Option<unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> usize>,
        *mut c_void,
        usize,
        *mut JsonError,
    ) -> *mut Json;

    unsafe fn pack_errors(api: &Api) -> Vec<(bool, (c_int, c_int, c_int, Vec<u8>, Vec<u8>, u8))> {
        let mut results = Vec::new();
        let mut call0 = |format: *const c_char| {
            let mut error = JsonError::blank();
            let value = unsafe { api.sym::<Pack0>(b"json_pack_ex\0")(&mut error, 0, format) };
            results.push((value.is_null(), error.comparable()));
            unsafe { api.delete(value) };
        };
        call0(ptr::null());
        call0(c"".as_ptr());
        call0(c"x".as_ptr());
        call0(c"nn".as_ptr());

        let mut error = JsonError::blank();
        let value = unsafe {
            api.sym::<PackS>(b"json_pack_ex\0")(&mut error, 0, c"s".as_ptr(), ptr::null())
        };
        results.push((value.is_null(), error.comparable()));

        let invalid = [0xffu8, 0];
        let mut error = JsonError::blank();
        let value = unsafe {
            api.sym::<PackS>(b"json_pack_ex\0")(
                &mut error,
                0,
                c"s".as_ptr(),
                invalid.as_ptr().cast(),
            )
        };
        results.push((value.is_null(), error.comparable()));

        let mut error = JsonError::blank();
        let value =
            unsafe { api.sym::<PackF>(b"json_pack_ex\0")(&mut error, 0, c"f".as_ptr(), f64::NAN) };
        results.push((value.is_null(), error.comparable()));

        let mut error = JsonError::blank();
        let value =
            unsafe { api.sym::<PackI>(b"json_pack_ex\0")(&mut error, 0, c"[i".as_ptr(), 1) };
        results.push((value.is_null(), error.comparable()));
        results
    }

    unsafe fn unpack_errors(
        api: &Api,
    ) -> Vec<(c_int, (c_int, c_int, c_int, Vec<u8>, Vec<u8>, u8))> {
        let mut results = Vec::new();
        let mut error = JsonError::blank();
        let result = unsafe {
            api.sym::<Unpack0>(b"json_unpack_ex\0")(ptr::null_mut(), &mut error, 0, c"n".as_ptr())
        };
        results.push((result, error.comparable()));

        let null = unsafe { api.sym::<New>(b"json_object\0")() };
        let mut error = JsonError::blank();
        let result =
            unsafe { api.sym::<Unpack0>(b"json_unpack_ex\0")(null, &mut error, 0, ptr::null()) };
        results.push((result, error.comparable()));

        let integer = unsafe { api.sym::<Int>(b"json_integer\0")(7) };
        let mut output = 0;
        let mut error = JsonError::blank();
        let result = unsafe {
            api.sym::<UnpackOut>(b"json_unpack_ex\0")(
                integer,
                &mut error,
                0,
                c"s".as_ptr(),
                &mut output,
            )
        };
        results.push((result, error.comparable()));

        let object = unsafe { api.sym::<New>(b"json_object\0")() };
        let mut error = JsonError::blank();
        let result = unsafe {
            api.sym::<UnpackS>(b"json_unpack_ex\0")(
                object,
                &mut error,
                0,
                c"{s:i}".as_ptr(),
                c"missing".as_ptr(),
                &mut output,
            )
        };
        results.push((result, error.comparable()));

        let array = unsafe { api.sym::<New>(b"json_array\0")() };
        let mut error = JsonError::blank();
        let result = unsafe {
            api.sym::<UnpackOut>(b"json_unpack_ex\0")(
                array,
                &mut error,
                0,
                c"[i]".as_ptr(),
                &mut output,
            )
        };
        results.push((result, error.comparable()));

        let strict = unsafe { api.load(br#"{"a":1,"b":2}"#, 0).0 };
        let mut error = JsonError::blank();
        let result = unsafe {
            api.sym::<UnpackS>(b"json_unpack_ex\0")(
                strict,
                &mut error,
                0,
                c"{s:i!}".as_ptr(),
                c"a".as_ptr(),
                &mut output,
            )
        };
        results.push((result, error.comparable()));

        let string = unsafe { api.sym::<StringN>(b"json_stringn\0")(b"value".as_ptr().cast(), 5) };
        let mut error = JsonError::blank();
        let result = unsafe {
            api.sym::<UnpackStr>(b"json_unpack_ex\0")(
                string,
                &mut error,
                0,
                c"s".as_ptr(),
                ptr::null_mut(),
            )
        };
        results.push((result, error.comparable()));

        unsafe {
            for value in [null, integer, object, array, strict, string] {
                api.delete(value);
            }
        }
        results
    }

    unsafe fn mutator_errors(api: &Api) -> Vec<c_int> {
        let object = unsafe { api.sym::<New>(b"json_object\0")() };
        let array = unsafe { api.sym::<New>(b"json_array\0")() };
        let integer = unsafe { api.sym::<Int>(b"json_integer\0")(1) };
        let string = unsafe { api.sym::<StringN>(b"json_stringn\0")(b"ok".as_ptr().cast(), 2) };
        let invalid = [0xffu8];
        let mut results = vec![
            unsafe {
                api.sym::<SetObject>(b"json_object_set_new\0")(
                    object,
                    ptr::null(),
                    api.sym::<New>(b"json_null\0")(),
                )
            },
            unsafe {
                api.sym::<SetObjectN>(b"json_object_setn_new\0")(
                    object,
                    invalid.as_ptr().cast(),
                    invalid.len(),
                    api.sym::<New>(b"json_null\0")(),
                )
            },
            unsafe {
                api.sym::<SetArray>(b"json_array_set_new\0")(
                    array,
                    0,
                    api.sym::<New>(b"json_null\0")(),
                )
            },
            unsafe {
                api.sym::<Append>(b"json_array_append_new\0")(
                    integer,
                    api.sym::<New>(b"json_null\0")(),
                )
            },
            unsafe { api.sym::<Remove>(b"json_array_remove\0")(array, usize::MAX) },
            unsafe {
                api.sym::<StringSet>(b"json_string_setn\0")(
                    string,
                    invalid.as_ptr().cast(),
                    invalid.len(),
                )
            },
            unsafe { api.sym::<RealSet>(b"json_real_set\0")(integer, f64::NAN) },
        ];
        results.push(unsafe {
            api.sym::<Dump>(b"json_dump_callback\0")(
                integer,
                Some(collect_dump),
                ptr::null_mut(),
                0,
            )
        });
        let mut error = JsonError::blank();
        results.push(
            unsafe { api.sym::<LoadFd>(b"json_loadfd\0")(-1, 0, &mut error) }.is_null() as c_int,
        );
        results.push(
            unsafe {
                api.sym::<LoadFile>(b"json_load_file\0")(
                    c"/path/that/does/not/exist".as_ptr(),
                    0,
                    &mut error,
                )
            }
            .is_null() as c_int,
        );
        results.push(
            unsafe {
                api.sym::<LoadCallback>(b"json_load_callback\0")(
                    None,
                    ptr::null_mut(),
                    0,
                    &mut error,
                )
            }
            .is_null() as c_int,
        );
        unsafe {
            for value in [object, array, integer, string] {
                api.delete(value);
            }
        }
        results
    }

    let (c, rust) = apis();
    assert_eq!(unsafe { pack_errors(&c) }, unsafe { pack_errors(&rust) });
    assert_eq!(unsafe { unpack_errors(&c) }, unsafe {
        unpack_errors(&rust)
    });
    assert_eq!(unsafe { mutator_errors(&c) }, unsafe {
        mutator_errors(&rust)
    });
}

#[test]
fn abi_layout_matches_c_headers() {
    assert_eq!(size_of::<Json>(), 16);
    assert_eq!(size_of::<JsonError>(), 252);
    assert_eq!(size_of::<StrBuffer>(), 24);
    assert_eq!(size_of::<Hashtable>(), 56);
    assert_eq!(
        CStr::from_bytes_with_nul(b"2.15.0\0").unwrap().to_bytes(),
        b"2.15.0"
    );
}
