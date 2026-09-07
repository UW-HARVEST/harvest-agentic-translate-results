use libloading::Library;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{mem, ptr, slice};

type ArrGrow = unsafe extern "C" fn(*mut c_void, usize, usize, usize) -> *mut c_void;
type ArrFree = unsafe extern "C" fn(*mut c_void);
type RandSeed = unsafe extern "C" fn(usize);
type HashBytes = unsafe extern "C" fn(*mut c_void, usize, usize) -> usize;
type HashString = unsafe extern "C" fn(*mut c_char, usize) -> usize;
type HmFree = unsafe extern "C" fn(*mut c_void, usize);
type HmGetTs =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, *mut isize, c_int) -> *mut c_void;
type HmGet = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type HmDefault = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type HmPut = unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, c_int) -> *mut c_void;
type ShMode = unsafe extern "C" fn(usize, c_int) -> *mut c_void;
type HmDel =
    unsafe extern "C" fn(*mut c_void, usize, *mut c_void, usize, usize, c_int) -> *mut c_void;
type StrAlloc = unsafe extern "C" fn(*mut Arena, *mut c_char) -> *mut c_char;
type StrReset = unsafe extern "C" fn(*mut Arena);
type StrKey = unsafe extern "C" fn(c_int) -> *mut c_char;
type IntPut = unsafe extern "C" fn(c_int);

struct Api {
    _lib: Library,
    arrgrow: ArrGrow,
    arrfree: ArrFree,
    rand_seed: RandSeed,
    hash_bytes: HashBytes,
    hash_string: HashString,
    hmfree: HmFree,
    hmget_ts: HmGetTs,
    hmget: HmGet,
    hmdefault: HmDefault,
    hmput: HmPut,
    shmode: ShMode,
    hmdel: HmDel,
    stralloc: StrAlloc,
    strreset: StrReset,
    strkey: StrKey,
    intput: IntPut,
}

impl Api {
    unsafe fn load(path: &Path) -> Self {
        let lib = unsafe { Library::new(path) }.unwrap_or_else(|error| {
            panic!("failed to load {}: {error}", path.display());
        });
        macro_rules! symbol {
            ($name:literal, $ty:ty) => {{
                let value = unsafe { lib.get::<$ty>(concat!($name, "\0").as_bytes()) }
                    .unwrap_or_else(|error| panic!("missing {}: {error}", $name));
                *value
            }};
        }
        let api = Self {
            arrgrow: symbol!("stbds_arrgrowf", ArrGrow),
            arrfree: symbol!("stbds_arrfreef", ArrFree),
            rand_seed: symbol!("stbds_rand_seed", RandSeed),
            hash_bytes: symbol!("stbds_hash_bytes", HashBytes),
            hash_string: symbol!("stbds_hash_string", HashString),
            hmfree: symbol!("stbds_hmfree_func", HmFree),
            hmget_ts: symbol!("stbds_hmget_key_ts", HmGetTs),
            hmget: symbol!("stbds_hmget_key", HmGet),
            hmdefault: symbol!("stbds_hmput_default", HmDefault),
            hmput: symbol!("stbds_hmput_key", HmPut),
            shmode: symbol!("stbds_shmode_func", ShMode),
            hmdel: symbol!("stbds_hmdel_key", HmDel),
            stralloc: symbol!("stbds_stralloc", StrAlloc),
            strreset: symbol!("stbds_strreset", StrReset),
            strkey: symbol!("strkey", StrKey),
            intput: symbol!("intput", IntPut),
            _lib: lib,
        };
        api
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Header {
    length: usize,
    capacity: usize,
    hash_table: *mut c_void,
    temp: isize,
}

#[repr(C)]
#[derive(Debug)]
struct Arena {
    storage: *mut c_void,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl Arena {
    fn zeroed() -> Self {
        Self {
            storage: ptr::null_mut(),
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }
}

fn paths() -> (PathBuf, PathBuf) {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    (
        crate_dir.join("../c_src/build/libharvest-work-QTbeWq.so"),
        crate_dir.join("target/release/libintput_lib.so"),
    )
}

unsafe fn header(array: *mut c_void) -> *mut Header {
    unsafe { array.cast::<Header>().sub(1) }
}

unsafe fn raw_from_hash(hash: *mut c_void, elem_size: usize) -> *mut c_void {
    unsafe { hash.cast::<u8>().sub(elem_size).cast() }
}

unsafe fn map_header(hash: *mut c_void, elem_size: usize) -> Header {
    unsafe { *header(raw_from_hash(hash, elem_size)) }
}

unsafe fn map_bytes(hash: *mut c_void, elem_size: usize) -> Vec<Vec<u8>> {
    let count = unsafe { map_header(hash, elem_size).length - 1 };
    (0..count)
        .map(|index| unsafe {
            slice::from_raw_parts(hash.cast::<u8>().add(index * elem_size), elem_size).to_vec()
        })
        .collect()
}

unsafe fn set_payload(hash: *mut c_void, elem_size: usize, index: isize, value: u64) {
    let destination = unsafe {
        hash.cast::<u8>()
            .add(index as usize * elem_size + elem_size - mem::size_of::<u64>())
    };
    unsafe { ptr::copy_nonoverlapping(value.to_ne_bytes().as_ptr(), destination, 8) };
}

fn rng_next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

unsafe fn compare_arrays(c: &Api, r: &Api) {
    let elem_size = 3usize;
    for requested in 0..12 {
        let ca = unsafe { (c.arrgrow)(ptr::null_mut(), elem_size, 0, requested) };
        let ra = unsafe { (r.arrgrow)(ptr::null_mut(), elem_size, 0, requested) };
        if requested == 0 {
            assert!(ca.is_null() && ra.is_null());
            continue;
        }
        let ch = unsafe { *header(ca) };
        let rh = unsafe { *header(ra) };
        assert_eq!(
            (ch.length, ch.capacity, ch.temp),
            (rh.length, rh.capacity, rh.temp)
        );
        assert!(ch.hash_table.is_null() && rh.hash_table.is_null());
        unsafe {
            (c.arrfree)(ca);
            (r.arrfree)(ra);
        }
    }

    let mut ca = unsafe { (c.arrgrow)(ptr::null_mut(), elem_size, 0, 4) };
    let mut ra = unsafe { (r.arrgrow)(ptr::null_mut(), elem_size, 0, 4) };
    unsafe {
        (*header(ca)).length = 3;
        (*header(ra)).length = 3;
        ptr::copy_nonoverlapping(b"abcdefghi".as_ptr(), ca.cast(), 9);
        ptr::copy_nonoverlapping(b"abcdefghi".as_ptr(), ra.cast(), 9);
    }
    let old_c = ca;
    let old_r = ra;
    ca = unsafe { (c.arrgrow)(ca, elem_size, 0, 4) };
    ra = unsafe { (r.arrgrow)(ra, elem_size, 0, 4) };
    assert_eq!(ca, old_c);
    assert_eq!(ra, old_r);

    ca = unsafe { (c.arrgrow)(ca, elem_size, 2, 0) };
    ra = unsafe { (r.arrgrow)(ra, elem_size, 2, 0) };
    assert_eq!(unsafe { (*header(ca)).capacity }, unsafe {
        (*header(ra)).capacity
    });
    assert_eq!(
        unsafe { slice::from_raw_parts(ca.cast::<u8>(), 9) },
        unsafe { slice::from_raw_parts(ra.cast::<u8>(), 9) }
    );

    ca = unsafe { (c.arrgrow)(ca, elem_size, 0, 41) };
    ra = unsafe { (r.arrgrow)(ra, elem_size, 0, 41) };
    assert_eq!(unsafe { (*header(ca)).capacity }, 41);
    assert_eq!(unsafe { (*header(ra)).capacity }, 41);
    unsafe {
        (c.arrfree)(ca);
        (r.arrfree)(ra);
    }
}

unsafe fn compare_hashes(c: &Api, r: &Api) {
    let seeds = [0usize, 1, 0x3141_5926, usize::MAX, 0xfeed_beef_dead_cafe];
    for seed in seeds {
        let empty = CString::new("").unwrap();
        assert_eq!(
            unsafe { (c.hash_string)(empty.as_ptr().cast_mut(), seed) },
            unsafe { (r.hash_string)(empty.as_ptr().cast_mut(), seed) }
        );
    }

    let mut state = 0x8f31_a2d4_5519_772bu64;
    for len in 0..96usize {
        for _ in 0..32 {
            let mut bytes = vec![0u8; len];
            for byte in &mut bytes {
                *byte = rng_next(&mut state) as u8;
            }
            let seed = rng_next(&mut state) as usize;
            let c_hash = unsafe {
                (c.hash_bytes)(
                    if bytes.is_empty() {
                        ptr::null_mut()
                    } else {
                        bytes.as_mut_ptr().cast()
                    },
                    bytes.len(),
                    seed,
                )
            };
            let r_hash = unsafe {
                (r.hash_bytes)(
                    if bytes.is_empty() {
                        ptr::null_mut()
                    } else {
                        bytes.as_mut_ptr().cast()
                    },
                    bytes.len(),
                    seed,
                )
            };
            assert_eq!(c_hash, r_hash, "byte hash mismatch at length {len}");

            let mut nonzero = bytes
                .iter()
                .map(|byte| if *byte == 0 { 1 } else { *byte })
                .collect::<Vec<_>>();
            nonzero.push(0);
            assert_eq!(
                unsafe { (c.hash_string)(nonzero.as_mut_ptr().cast(), seed) },
                unsafe { (r.hash_string)(nonzero.as_mut_ptr().cast(), seed) },
                "string hash mismatch at length {len}"
            );
        }
    }

    let mut large = vec![0u8; 4096];
    for byte in &mut large {
        *byte = rng_next(&mut state) as u8;
    }
    assert_eq!(
        unsafe { (c.hash_bytes)(large.as_mut_ptr().cast(), large.len(), usize::MAX) },
        unsafe { (r.hash_bytes)(large.as_mut_ptr().cast(), large.len(), usize::MAX) }
    );
}

unsafe fn compare_binary_maps(c: &Api, r: &Api) {
    for key_size in [1usize, 2, 4, 8, 13] {
        let elem_size = key_size + 8;
        let mut cm = ptr::null_mut();
        let mut rm = ptr::null_mut();
        unsafe {
            (c.rand_seed)(0x1234_5000 + key_size);
            (r.rand_seed)(0x1234_5000 + key_size);
        }
        let mut state = 0x90d7_315a_f00d_1234u64 ^ key_size as u64;
        let mut keys = Vec::new();
        for item in 0..96u64 {
            let mut key = vec![0u8; key_size];
            for byte in &mut key {
                *byte = rng_next(&mut state) as u8;
            }
            key[0] ^= item as u8;
            cm = unsafe { (c.hmput)(cm, elem_size, key.as_mut_ptr().cast(), key_size, 0) };
            rm = unsafe { (r.hmput)(rm, elem_size, key.as_mut_ptr().cast(), key_size, 0) };
            let ci = unsafe { map_header(cm, elem_size).temp };
            let ri = unsafe { map_header(rm, elem_size).temp };
            assert_eq!(ci, ri);
            let value = rng_next(&mut state);
            unsafe {
                set_payload(cm, elem_size, ci, value);
                set_payload(rm, elem_size, ri, value);
            }
            keys.push(key);
            assert_eq!(unsafe { map_header(cm, elem_size).capacity }, unsafe {
                map_header(rm, elem_size).capacity
            });
            assert_eq!(unsafe { map_bytes(cm, elem_size) }, unsafe {
                map_bytes(rm, elem_size)
            });
        }

        for (index, key) in keys.iter_mut().enumerate() {
            let mut ct = 123isize;
            let mut rt = 456isize;
            let cp = unsafe {
                (c.hmget_ts)(cm, elem_size, key.as_mut_ptr().cast(), key_size, &mut ct, 0)
            };
            let rp = unsafe {
                (r.hmget_ts)(rm, elem_size, key.as_mut_ptr().cast(), key_size, &mut rt, 0)
            };
            assert_eq!(ct, rt, "lookup index mismatch for item {index}");
            assert_eq!(cp, cm);
            assert_eq!(rp, rm);
        }

        let mut existing = keys[17].clone();
        cm = unsafe { (c.hmput)(cm, elem_size, existing.as_mut_ptr().cast(), key_size, 0) };
        rm = unsafe { (r.hmput)(rm, elem_size, existing.as_mut_ptr().cast(), key_size, 0) };
        assert_eq!(unsafe { map_header(cm, elem_size).temp }, unsafe {
            map_header(rm, elem_size).temp
        });

        let mut missing = vec![0x5au8; key_size];
        let mut ct = 0;
        let mut rt = 0;
        unsafe {
            (c.hmget_ts)(
                cm,
                elem_size,
                missing.as_mut_ptr().cast(),
                key_size,
                &mut ct,
                0,
            );
            (r.hmget_ts)(
                rm,
                elem_size,
                missing.as_mut_ptr().cast(),
                key_size,
                &mut rt,
                0,
            );
        }
        assert_eq!(ct, rt);

        for key in keys.iter_mut().step_by(3) {
            cm = unsafe { (c.hmdel)(cm, elem_size, key.as_mut_ptr().cast(), key_size, 0, 0) };
            rm = unsafe { (r.hmdel)(rm, elem_size, key.as_mut_ptr().cast(), key_size, 0, 0) };
            assert_eq!(unsafe { map_header(cm, elem_size).temp }, unsafe {
                map_header(rm, elem_size).temp
            });
            assert_eq!(unsafe { map_bytes(cm, elem_size) }, unsafe {
                map_bytes(rm, elem_size)
            });
        }
        for key in keys.iter_mut().skip(1) {
            cm = unsafe { (c.hmdel)(cm, elem_size, key.as_mut_ptr().cast(), key_size, 0, 0) };
            rm = unsafe { (r.hmdel)(rm, elem_size, key.as_mut_ptr().cast(), key_size, 0, 0) };
            assert_eq!(unsafe { map_bytes(cm, elem_size) }, unsafe {
                map_bytes(rm, elem_size)
            });
        }
        unsafe {
            (c.hmfree)(raw_from_hash(cm, elem_size), elem_size);
            (r.hmfree)(raw_from_hash(rm, elem_size), elem_size);
        }
    }
}

unsafe fn compare_defaults_and_errors(c: &Api, r: &Api) {
    let elem_size = 16usize;
    let mut key = 77u64;
    let mut ct = 99isize;
    let mut rt = 99isize;
    let cm = unsafe {
        (c.hmget_ts)(
            ptr::null_mut(),
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut ct,
            0,
        )
    };
    let rm = unsafe {
        (r.hmget_ts)(
            ptr::null_mut(),
            elem_size,
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut rt,
            0,
        )
    };
    assert_eq!((ct, rt), (-1, -1));
    assert_eq!(unsafe { map_bytes(cm, elem_size) }, unsafe {
        map_bytes(rm, elem_size)
    });

    let cm2 = unsafe { (c.hmget)(cm, elem_size, ptr::addr_of_mut!(key).cast(), 8, 0) };
    let rm2 = unsafe { (r.hmget)(rm, elem_size, ptr::addr_of_mut!(key).cast(), 8, 0) };
    assert_eq!(unsafe { map_header(cm2, elem_size).temp }, -1);
    assert_eq!(unsafe { map_header(rm2, elem_size).temp }, -1);
    let c_same = unsafe { (c.hmdefault)(cm2, elem_size) };
    let r_same = unsafe { (r.hmdefault)(rm2, elem_size) };
    assert_eq!(c_same, cm2);
    assert_eq!(r_same, rm2);

    assert!(
        unsafe {
            (c.hmdel)(
                ptr::null_mut(),
                elem_size,
                ptr::addr_of_mut!(key).cast(),
                8,
                0,
                0,
            )
        }
        .is_null()
    );
    assert!(
        unsafe {
            (r.hmdel)(
                ptr::null_mut(),
                elem_size,
                ptr::addr_of_mut!(key).cast(),
                8,
                0,
                0,
            )
        }
        .is_null()
    );
    unsafe {
        (c.hmfree)(raw_from_hash(cm2, elem_size), elem_size);
        (r.hmfree)(raw_from_hash(rm2, elem_size), elem_size);
        (c.hmfree)(ptr::null_mut(), elem_size);
        (r.hmfree)(ptr::null_mut(), elem_size);
    }

    let cd = unsafe { (c.hmdefault)(ptr::null_mut(), elem_size) };
    let rd = unsafe { (r.hmdefault)(ptr::null_mut(), elem_size) };
    assert_eq!(unsafe { map_bytes(cd, elem_size) }, unsafe {
        map_bytes(rd, elem_size)
    });
    unsafe {
        (c.hmfree)(raw_from_hash(cd, elem_size), elem_size);
        (r.hmfree)(raw_from_hash(rd, elem_size), elem_size);
    }
}

unsafe fn logical_string_map(hash: *mut c_void, elem_size: usize) -> Vec<(Vec<u8>, u64)> {
    let count = unsafe { map_header(hash, elem_size).length - 1 };
    let mut result = Vec::new();
    for index in 0..count {
        let entry = unsafe { hash.cast::<u8>().add(index * elem_size) };
        let key = unsafe { entry.cast::<*mut c_char>().read_unaligned() };
        let value = unsafe { entry.add(8).cast::<u64>().read_unaligned() };
        result.push((unsafe { CStr::from_ptr(key).to_bytes().to_vec() }, value));
    }
    result.sort();
    result
}

unsafe fn compare_string_maps(c: &Api, r: &Api) {
    const ELEM: usize = 16;
    for storage_mode in [1, 2, 3] {
        unsafe {
            (c.rand_seed)(0x8877_6600 + storage_mode as usize);
            (r.rand_seed)(0x8877_6600 + storage_mode as usize);
        }
        let mut cm = unsafe { (c.shmode)(ELEM, storage_mode) };
        let mut rm = unsafe { (r.shmode)(ELEM, storage_mode) };
        let keys = (0..80)
            .map(|index| CString::new(format!("key_{index:03}_{}", index * 17)).unwrap())
            .collect::<Vec<_>>();
        for (index, key) in keys.iter().enumerate() {
            cm = unsafe { (c.hmput)(cm, ELEM, key.as_ptr().cast_mut().cast(), 8, 1) };
            rm = unsafe { (r.hmput)(rm, ELEM, key.as_ptr().cast_mut().cast(), 8, 1) };
            let ci = unsafe { map_header(cm, ELEM).temp };
            let ri = unsafe { map_header(rm, ELEM).temp };
            assert_eq!(ci, ri);
            unsafe {
                set_payload(cm, ELEM, ci, index as u64 * 19);
                set_payload(rm, ELEM, ri, index as u64 * 19);
            }
        }
        assert_eq!(unsafe { logical_string_map(cm, ELEM) }, unsafe {
            logical_string_map(rm, ELEM)
        });

        for key in keys.iter().step_by(7) {
            let mut ct = 0isize;
            let mut rt = 0isize;
            unsafe {
                (c.hmget_ts)(cm, ELEM, key.as_ptr().cast_mut().cast(), 8, &mut ct, 1);
                (r.hmget_ts)(rm, ELEM, key.as_ptr().cast_mut().cast(), 8, &mut rt, 1);
            }
            assert_eq!(ct, rt);
        }
        let absent = CString::new("definitely-absent").unwrap();
        let before_c = unsafe { logical_string_map(cm, ELEM) };
        let before_r = unsafe { logical_string_map(rm, ELEM) };
        cm = unsafe { (c.hmdel)(cm, ELEM, absent.as_ptr().cast_mut().cast(), 8, 0, 1) };
        rm = unsafe { (r.hmdel)(rm, ELEM, absent.as_ptr().cast_mut().cast(), 8, 0, 1) };
        assert_eq!(unsafe { map_header(cm, ELEM).temp }, 0);
        assert_eq!(unsafe { map_header(rm, ELEM).temp }, 0);
        assert_eq!(unsafe { logical_string_map(cm, ELEM) }, before_c);
        assert_eq!(unsafe { logical_string_map(rm, ELEM) }, before_r);

        let update = &keys[23];
        cm = unsafe { (c.hmput)(cm, ELEM, update.as_ptr().cast_mut().cast(), 8, 1) };
        rm = unsafe { (r.hmput)(rm, ELEM, update.as_ptr().cast_mut().cast(), 8, 1) };
        let ci = unsafe { map_header(cm, ELEM).temp };
        let ri = unsafe { map_header(rm, ELEM).temp };
        unsafe {
            set_payload(cm, ELEM, ci, 0xdead_beef);
            set_payload(rm, ELEM, ri, 0xdead_beef);
        }
        for key in keys.iter().step_by(4) {
            cm = unsafe { (c.hmdel)(cm, ELEM, key.as_ptr().cast_mut().cast(), 8, 0, 1) };
            rm = unsafe { (r.hmdel)(rm, ELEM, key.as_ptr().cast_mut().cast(), 8, 0, 1) };
        }
        assert_eq!(unsafe { logical_string_map(cm, ELEM) }, unsafe {
            logical_string_map(rm, ELEM)
        });
        unsafe {
            (c.hmfree)(raw_from_hash(cm, ELEM), ELEM);
            (r.hmfree)(raw_from_hash(rm, ELEM), ELEM);
        }
    }

    let key = CString::new("implicit-default").unwrap();
    let cm = unsafe { (c.hmput)(ptr::null_mut(), ELEM, key.as_ptr().cast_mut().cast(), 8, 42) };
    let rm = unsafe { (r.hmput)(ptr::null_mut(), ELEM, key.as_ptr().cast_mut().cast(), 8, 42) };
    assert_eq!(unsafe { logical_string_map(cm, ELEM) }, unsafe {
        logical_string_map(rm, ELEM)
    });
    unsafe {
        (c.hmfree)(raw_from_hash(cm, ELEM), ELEM);
        (r.hmfree)(raw_from_hash(rm, ELEM), ELEM);
    }

    let mut binary_key = 0x1234_5678_9abc_def0u64;
    let mut cm = ptr::null_mut();
    let mut rm = ptr::null_mut();
    cm = unsafe { (c.hmput)(cm, ELEM, ptr::addr_of_mut!(binary_key).cast(), 8, -17) };
    rm = unsafe { (r.hmput)(rm, ELEM, ptr::addr_of_mut!(binary_key).cast(), 8, -17) };
    assert_eq!(unsafe { map_bytes(cm, ELEM) }, unsafe {
        map_bytes(rm, ELEM)
    });
    unsafe {
        (c.hmfree)(raw_from_hash(cm, ELEM), ELEM);
        (r.hmfree)(raw_from_hash(rm, ELEM), ELEM);
    }

    let mut odd_key = 0x7654_3210u64;
    let mut cm = unsafe { (c.shmode)(ELEM, 0x163) };
    let mut rm = unsafe { (r.shmode)(ELEM, 0x163) };
    cm = unsafe { (c.hmput)(cm, ELEM, ptr::addr_of_mut!(odd_key).cast(), 8, 0) };
    rm = unsafe { (r.hmput)(rm, ELEM, ptr::addr_of_mut!(odd_key).cast(), 8, 0) };
    assert_eq!(unsafe { map_bytes(cm, ELEM) }, unsafe {
        map_bytes(rm, ELEM)
    });
    unsafe {
        (c.hmfree)(raw_from_hash(cm, ELEM), ELEM);
        (r.hmfree)(raw_from_hash(rm, ELEM), ELEM);
    }
}

unsafe fn compare_arenas(c: &Api, r: &Api) {
    let mut ca = Arena::zeroed();
    let mut ra = Arena::zeroed();
    unsafe {
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
    assert_eq!((ca.remaining, ca.block, ca.mode), (0, 0, 0));
    assert_eq!((ra.remaining, ra.block, ra.mode), (0, 0, 0));

    for text in ["", "a", "short", "another short value"] {
        let value = CString::new(text).unwrap();
        let cp = unsafe { (c.stralloc)(&mut ca, value.as_ptr().cast_mut()) };
        let rp = unsafe { (r.stralloc)(&mut ra, value.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(cp).to_bytes() }, unsafe {
            CStr::from_ptr(rp).to_bytes()
        });
        assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
    }
    unsafe {
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }

    for _ in 0..24 {
        let block_size = 512usize << (ca.block >> 1);
        let block_size = block_size.min(1 << 20);
        let value = CString::new(vec![b'x'; block_size - 1]).unwrap();
        let cp = unsafe { (c.stralloc)(&mut ca, value.as_ptr().cast_mut()) };
        let rp = unsafe { (r.stralloc)(&mut ra, value.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(cp).to_bytes().len() }, unsafe {
            CStr::from_ptr(rp).to_bytes().len()
        });
        assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
    }
    unsafe {
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }

    for size in [600usize, 20, 2200, (1 << 20) + 31] {
        let value = CString::new(vec![b'z'; size]).unwrap();
        let cp = unsafe { (c.stralloc)(&mut ca, value.as_ptr().cast_mut()) };
        let rp = unsafe { (r.stralloc)(&mut ra, value.as_ptr().cast_mut()) };
        assert_eq!(unsafe { CStr::from_ptr(cp).to_bytes() }, unsafe {
            CStr::from_ptr(rp).to_bytes()
        });
        assert_eq!((ca.remaining, ca.block), (ra.remaining, ra.block));
    }
    unsafe {
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
    }
    assert!(ca.storage.is_null() && ra.storage.is_null());
    assert_eq!((ca.remaining, ca.block, ca.mode), (0, 0, 0));
    assert_eq!((ra.remaining, ra.block, ra.mode), (0, 0, 0));
}

unsafe fn compare_wrappers(c: &Api, r: &Api) {
    for number in [c_int::MIN, -1000, -1, 0, 1, 9, 11, 42, c_int::MAX] {
        let cp = unsafe { (c.strkey)(number) };
        let rp = unsafe { (r.strkey)(number) };
        assert_eq!(unsafe { CStr::from_ptr(cp).to_bytes() }, unsafe {
            CStr::from_ptr(rp).to_bytes()
        });
    }
    for number in [-1000, -1, 0, 1, 8, 10, 12, 1000] {
        unsafe {
            (c.intput)(number);
            (r.intput)(number);
        }
    }
}

fn child_status(path: &Path, number: c_int) -> std::process::ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("intput_child")
        .arg("--nocapture")
        .env("DIFF_CHILD_LIB", path)
        .env("DIFF_CHILD_NUM", number.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
}

#[test]
fn intput_child() {
    let Ok(path) = std::env::var("DIFF_CHILD_LIB") else {
        return;
    };
    let number = std::env::var("DIFF_CHILD_NUM")
        .unwrap()
        .parse::<c_int>()
        .unwrap();
    let api = unsafe { Api::load(Path::new(&path)) };
    unsafe { (api.intput)(number) };
}

#[test]
fn differential_surface() {
    let (c_path, rust_path) = paths();
    assert!(c_path.is_file(), "missing {}", c_path.display());
    assert!(rust_path.is_file(), "missing {}", rust_path.display());
    let c = unsafe { Api::load(&c_path) };
    let r = unsafe { Api::load(&rust_path) };

    unsafe {
        compare_arrays(&c, &r);
        compare_hashes(&c, &r);
        compare_defaults_and_errors(&c, &r);
        compare_binary_maps(&c, &r);
        compare_string_maps(&c, &r);
        compare_arenas(&c, &r);
        compare_wrappers(&c, &r);
    }

    for number in [9, 11] {
        let c_status = child_status(&c_path, number);
        let r_status = child_status(&rust_path, number);
        assert_eq!(
            c_status.signal(),
            r_status.signal(),
            "intput({number}) termination differs: C={c_status:?}, Rust={r_status:?}"
        );
        assert!(!c_status.success(), "C intput({number}) should assert");
        assert!(!r_status.success(), "Rust intput({number}) should assert");
    }
}
