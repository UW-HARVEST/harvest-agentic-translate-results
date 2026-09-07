#![allow(unsafe_op_in_unsafe_fn)]

mod common;

use common::*;
use std::ffi::{CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
struct BinaryEntry {
    key: u64,
    spare: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

unsafe fn assert_same_fault<C, R>(label: &str, c_operation: C, rust_operation: R)
where
    C: FnOnce(),
    R: FnOnce(),
{
    let c_outcome = isolated(c_operation);
    let rust_outcome = isolated(rust_operation);
    assert_ne!(c_outcome.signal, 0, "{label}: C unexpectedly returned");
    assert_eq!(c_outcome, rust_outcome, "{label}");
}

unsafe fn make_binary_map(api: &Api, keys: &[u64]) -> *mut c_void {
    let mut map = ptr::null_mut();
    for &key in keys {
        let mut input = key;
        map = (api.hmput)(
            map,
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(input).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        let index = (*map_header(map, size_of::<BinaryEntry>())).temp as usize;
        *map.cast::<BinaryEntry>().add(index) = BinaryEntry { key, spare: key };
    }
    map
}

unsafe fn make_string_map(api: &Api, key: &CString) -> *mut c_void {
    let mut map = (api.shmode)(size_of::<StringEntry>(), SH_DEFAULT);
    map = (api.hmput)(
        map,
        size_of::<StringEntry>(),
        key.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );
    map
}

#[test]
fn null_oversized_and_enum_boundaries_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();
        let valid_string = CString::new("valid").unwrap();

        assert_same_fault(
            "arrfree(NULL)",
            || (c.arrfree)(ptr::null_mut()),
            || (rust.arrfree)(ptr::null_mut()),
        );
        assert_same_fault(
            "hash_string(NULL)",
            || {
                (c.hash_string)(ptr::null_mut(), 0);
            },
            || {
                (rust.hash_string)(ptr::null_mut(), 0);
            },
        );
        assert_same_fault(
            "hash_bytes(NULL, 1)",
            || {
                (c.hash_bytes)(ptr::null_mut(), 1, 0);
            },
            || {
                (rust.hash_bytes)(ptr::null_mut(), 1, 0);
            },
        );
        assert_same_fault(
            "hmget_key_ts temp=NULL",
            || {
                let mut key = 1_u64;
                (c.hmget_ts)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    ptr::null_mut(),
                    HM_BINARY,
                );
            },
            || {
                let mut key = 1_u64;
                (rust.hmget_ts)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    ptr::null_mut(),
                    HM_BINARY,
                );
            },
        );
        assert_same_fault(
            "binary hmget NULL key",
            || {
                let map = make_binary_map(&c, &[1]);
                (c.hmget)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
            },
            || {
                let map = make_binary_map(&rust, &[1]);
                (rust.hmget)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
            },
        );
        assert_same_fault(
            "string hmget NULL key",
            || {
                let map = make_string_map(&c, &valid_string);
                (c.hmget)(
                    map,
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
            },
            || {
                let map = make_string_map(&rust, &valid_string);
                (rust.hmget)(
                    map,
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
            },
        );
        assert_same_fault(
            "binary hmput NULL key",
            || {
                (c.hmput)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
            },
            || {
                (rust.hmput)(
                    ptr::null_mut(),
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    HM_BINARY,
                );
            },
        );
        assert_same_fault(
            "string hmput NULL key",
            || {
                (c.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
            },
            || {
                (rust.hmput)(
                    ptr::null_mut(),
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
            },
        );
        assert_same_fault(
            "binary hmdel NULL key",
            || {
                let map = make_binary_map(&c, &[1]);
                (c.hmdel)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    0,
                    HM_BINARY,
                );
            },
            || {
                let map = make_binary_map(&rust, &[1]);
                (rust.hmdel)(
                    map,
                    size_of::<BinaryEntry>(),
                    ptr::null_mut(),
                    size_of::<u64>(),
                    0,
                    HM_BINARY,
                );
            },
        );
        assert_same_fault(
            "string hmdel NULL key",
            || {
                let map = make_string_map(&c, &valid_string);
                (c.hmdel)(
                    map,
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                );
            },
            || {
                let map = make_string_map(&rust, &valid_string);
                (rust.hmdel)(
                    map,
                    size_of::<StringEntry>(),
                    ptr::null_mut(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                );
            },
        );
        assert_same_fault(
            "stralloc NULL arena",
            || {
                (c.stralloc)(ptr::null_mut(), valid_string.as_ptr().cast_mut());
            },
            || {
                (rust.stralloc)(ptr::null_mut(), valid_string.as_ptr().cast_mut());
            },
        );
        assert_same_fault(
            "stralloc NULL string",
            || {
                let mut arena = StringArena::default();
                (c.stralloc)(&mut arena, ptr::null_mut());
            },
            || {
                let mut arena = StringArena::default();
                (rust.stralloc)(&mut arena, ptr::null_mut());
            },
        );
        assert_same_fault(
            "strreset NULL arena",
            || (c.strreset)(ptr::null_mut()),
            || (rust.strreset)(ptr::null_mut()),
        );

        (c.hmfree)(ptr::null_mut(), size_of::<BinaryEntry>());
        (rust.hmfree)(ptr::null_mut(), size_of::<BinaryEntry>());
        let mut key = 1_u64;
        assert!(
            (c.hmdel)(
                ptr::null_mut(),
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                0,
                HM_BINARY,
            )
            .is_null()
        );
        assert!(
            (rust.hmdel)(
                ptr::null_mut(),
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                0,
                HM_BINARY,
            )
            .is_null()
        );

        let c_huge = (c.arrgrow)(ptr::null_mut(), 2, 0, usize::MAX);
        let r_huge = (rust.arrgrow)(ptr::null_mut(), 2, 0, usize::MAX);
        assert_eq!((*header(c_huge)).capacity, usize::MAX);
        assert_eq!((*header(r_huge)).capacity, usize::MAX);
        (c.arrfree)(c_huge);
        (rust.arrfree)(r_huge);

        assert_same_fault(
            "hash_bytes oversized length",
            || {
                let mut byte = 1_u8;
                (c.hash_bytes)(ptr::addr_of_mut!(byte).cast(), usize::MAX, 0);
            },
            || {
                let mut byte = 1_u8;
                (rust.hash_bytes)(ptr::addr_of_mut!(byte).cast(), usize::MAX, 0);
            },
        );
        assert_same_fault(
            "shmode oversized element size",
            || {
                (c.shmode)(usize::MAX, SH_DEFAULT);
            },
            || {
                (rust.shmode)(usize::MAX, SH_DEFAULT);
            },
        );

        for mode in [c_int::MIN, -1, 4, 255, 256, c_int::MAX] {
            (c.rand_seed)(0xabcdef);
            (rust.rand_seed)(0xabcdef);
            if mode < HM_STRING {
                let mut c_map = ptr::null_mut();
                let mut r_map = ptr::null_mut();
                let mut key = 0x1122_3344_5566_7788_u64;
                c_map = (c.hmput)(
                    c_map,
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    mode,
                );
                r_map = (rust.hmput)(
                    r_map,
                    size_of::<BinaryEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    size_of::<u64>(),
                    mode,
                );
                assert_eq!(
                    (*map_header(c_map, size_of::<BinaryEntry>())).temp,
                    (*map_header(r_map, size_of::<BinaryEntry>())).temp
                );
                free_map(&c, c_map, size_of::<BinaryEntry>());
                free_map(&rust, r_map, size_of::<BinaryEntry>());
            } else {
                let key = CString::new(format!("mode_{mode}")).unwrap();
                let c_map = make_string_map_with_mode(&c, &key, mode);
                let r_map = make_string_map_with_mode(&rust, &key, mode);
                assert_eq!(
                    (*map_header(c_map, size_of::<StringEntry>())).temp,
                    (*map_header(r_map, size_of::<StringEntry>())).temp
                );
                free_map(&c, c_map, size_of::<StringEntry>());
                free_map(&rust, r_map, size_of::<StringEntry>());
            }
        }
    }
}

unsafe fn make_string_map_with_mode(
    api: &Api,
    key: &CString,
    operation_mode: c_int,
) -> *mut c_void {
    (api.hmput)(
        ptr::null_mut(),
        size_of::<StringEntry>(),
        key.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        operation_mode,
    )
}

unsafe fn corrupt_growth_assert(api: &Api) {
    let map = (api.shmode)(size_of::<StringEntry>(), SH_DEFAULT);
    let hash_table = table(map, size_of::<StringEntry>());
    (*hash_table).slot_count = 1;
    (*hash_table).used_count_threshold = 0;
    let key = CString::new("trigger").unwrap();
    (api.hmput)(
        map,
        size_of::<StringEntry>(),
        key.as_ptr().cast_mut().cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );
}

unsafe fn corrupt_slot_bound_assert(api: &Api) {
    let map = make_binary_map(api, &[11]);
    let hash_table = table(map, size_of::<BinaryEntry>());
    let mut key = 11_u64;
    let mut hash = (api.hash_bytes)(
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        (*hash_table).seed,
    );
    if hash < 2 {
        hash += 2;
    }
    (*hash_table).slot_count = 4;
    let start = hash & 3;
    let bucket = (*hash_table).storage;
    for index in start..7 {
        (*bucket).hash[index] = if hash == 2 { 3 } else { 2 };
        (*bucket).index[index] = -1;
    }
    (*bucket).hash[7] = hash;
    (*bucket).index[7] = 0;
    (api.hmdel)(
        map,
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(key).cast(),
        size_of::<u64>(),
        0,
        HM_BINARY,
    );
}

unsafe fn corrupt_moved_missing_assert(api: &Api) {
    let map = make_binary_map(api, &[1, 2]);
    (*map.cast::<BinaryEntry>().add(1)).key = 999;
    let mut deleted = 1_u64;
    (api.hmdel)(
        map,
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(deleted).cast(),
        size_of::<u64>(),
        0,
        HM_BINARY,
    );
}

unsafe fn corrupt_moved_index_assert(api: &Api) {
    let map = make_binary_map(api, &[1, 2, 3]);
    let hash_table = table(map, size_of::<BinaryEntry>());
    let mut changed = false;
    for bucket_index in 0..((*hash_table).slot_count >> 3) {
        let bucket = (*hash_table).storage.add(bucket_index);
        for index in 0..8 {
            if (*bucket).index[index] == 2 {
                (*bucket).index[index] = 1;
                changed = true;
            }
        }
    }
    assert!(changed);
    (*map.cast::<BinaryEntry>().add(1)).key = 3;
    let mut deleted = 1_u64;
    (api.hmdel)(
        map,
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(deleted).cast(),
        size_of::<u64>(),
        0,
        HM_BINARY,
    );
}

#[test]
fn reachable_internal_assertions_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();
        assert_same_fault(
            "hash-index threshold assertion",
            || corrupt_growth_assert(&c),
            || corrupt_growth_assert(&rust),
        );
        assert_same_fault(
            "delete slot bound assertion",
            || corrupt_slot_bound_assert(&c),
            || corrupt_slot_bound_assert(&rust),
        );
        assert_same_fault(
            "moved entry missing assertion",
            || corrupt_moved_missing_assert(&c),
            || corrupt_moved_missing_assert(&rust),
        );
        assert_same_fault(
            "moved entry index assertion",
            || corrupt_moved_index_assert(&c),
            || corrupt_moved_index_assert(&rust),
        );
    }
}

#[test]
fn non_constructible_postconditions_hold_on_public_inputs() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();

        let mut c_map = ptr::null_mut();
        let mut r_map = ptr::null_mut();
        for key in 0..256_u64 {
            let mut c_key = key;
            let mut r_key = key;
            c_map = (c.hmput)(
                c_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(c_key).cast(),
                size_of::<u64>(),
                HM_BINARY,
            );
            r_map = (rust.hmput)(
                r_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(r_key).cast(),
                size_of::<u64>(),
                HM_BINARY,
            );
        }
        for key in 0..255_u64 {
            let mut c_key = key;
            let mut r_key = key;
            c_map = (c.hmdel)(
                c_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(c_key).cast(),
                size_of::<u64>(),
                0,
                HM_BINARY,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(r_key).cast(),
                size_of::<u64>(),
                0,
                HM_BINARY,
            );
        }
        assert_eq!(
            (*map_header(c_map, size_of::<BinaryEntry>())).length,
            (*map_header(r_map, size_of::<BinaryEntry>())).length
        );
        free_map(&c, c_map, size_of::<BinaryEntry>());
        free_map(&rust, r_map, size_of::<BinaryEntry>());

        let mut c_arena = StringArena::default();
        let mut r_arena = StringArena::default();
        for length in [1_usize, 2, 511, 512, 513, 4096, (1 << 20) + 1] {
            let value = CString::new(vec![b'x'; length - 1]).unwrap();
            let c_result = (c.stralloc)(&mut c_arena, value.as_ptr().cast_mut());
            let r_result = (rust.stralloc)(&mut r_arena, value.as_ptr().cast_mut());
            assert_eq!(
                std::ffi::CStr::from_ptr(c_result),
                std::ffi::CStr::from_ptr(r_result)
            );
        }
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);

        for number in [c_int::MIN, -1, 0, 1, 64, 128, 1000] {
            let c_status = isolated(|| (c.sh_puts)(number));
            let r_status = isolated(|| (rust.sh_puts)(number));
            assert_eq!(c_status, r_status);
            assert_eq!(c_status.exit_code, 0);
        }
    }
}
