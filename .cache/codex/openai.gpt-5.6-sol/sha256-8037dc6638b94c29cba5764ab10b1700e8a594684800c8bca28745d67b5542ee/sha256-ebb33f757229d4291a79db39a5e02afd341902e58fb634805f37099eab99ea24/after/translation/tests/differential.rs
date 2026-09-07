#![allow(unsafe_op_in_unsafe_fn)]

mod common;

use common::*;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BinaryEntry {
    key: u64,
    duplicate_key: u64,
    value: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct StringEntry {
    key: *mut c_char,
    value: c_int,
}

unsafe fn assert_array_equal(left: *mut c_void, right: *mut c_void, initialized_bytes: usize) {
    assert_eq!(left.is_null(), right.is_null());
    if left.is_null() {
        return;
    }
    assert_eq!((*header(left)).length, (*header(right)).length);
    assert_eq!((*header(left)).capacity, (*header(right)).capacity);
    assert_eq!((*header(left)).temp, (*header(right)).temp);
    assert_eq!(
        std::slice::from_raw_parts(left.cast::<u8>(), initialized_bytes),
        std::slice::from_raw_parts(right.cast::<u8>(), initialized_bytes)
    );
}

unsafe fn binary_put(api: &Api, map: &mut *mut c_void, key: u64, value: u64) {
    let mut input_key = key;
    *map = (api.hmput)(
        *map,
        size_of::<BinaryEntry>(),
        ptr::addr_of_mut!(input_key).cast(),
        size_of::<u64>(),
        HM_BINARY,
    );
    let index = (*map_header(*map, size_of::<BinaryEntry>())).temp as usize;
    let entry = (*map).cast::<BinaryEntry>().add(index);
    (*entry).key = key;
    (*entry).duplicate_key = key;
    (*entry).value = value;
}

unsafe fn binary_snapshot(map: *mut c_void) -> Vec<BinaryEntry> {
    let logical_length = (*map_header(map, size_of::<BinaryEntry>())).length - 1;
    std::slice::from_raw_parts(map.cast::<BinaryEntry>(), logical_length).to_vec()
}

unsafe fn assert_binary_maps_equal(c_map: *mut c_void, rust_map: *mut c_void) {
    assert_eq!(c_map.is_null(), rust_map.is_null());
    if c_map.is_null() {
        return;
    }
    let c_header = &*map_header(c_map, size_of::<BinaryEntry>());
    let r_header = &*map_header(rust_map, size_of::<BinaryEntry>());
    assert_eq!(c_header.length, r_header.length);
    assert_eq!(c_header.capacity, r_header.capacity);
    assert_eq!(c_header.temp, r_header.temp);
    assert_eq!(binary_snapshot(c_map), binary_snapshot(rust_map));

    let c_table = table(c_map, size_of::<BinaryEntry>());
    let r_table = table(rust_map, size_of::<BinaryEntry>());
    assert_eq!(c_table.is_null(), r_table.is_null());
    if !c_table.is_null() {
        assert_eq!((*c_table).slot_count, (*r_table).slot_count);
        assert_eq!((*c_table).used_count, (*r_table).used_count);
        assert_eq!((*c_table).tombstone_count, (*r_table).tombstone_count);
        assert_eq!((*c_table).seed, (*r_table).seed);
    }
}

unsafe fn string_put(api: &Api, map: &mut *mut c_void, key: *mut c_char, value: c_int) {
    *map = (api.hmput)(
        *map,
        size_of::<StringEntry>(),
        key.cast(),
        size_of::<*mut c_char>(),
        HM_STRING,
    );
    let index = (*map_header(*map, size_of::<StringEntry>())).temp as usize;
    (*(*map).cast::<StringEntry>().add(index)).value = value;
}

unsafe fn string_snapshot(map: *mut c_void) -> Vec<(Vec<u8>, c_int)> {
    let logical_length = (*map_header(map, size_of::<StringEntry>())).length - 1;
    (0..logical_length)
        .map(|index| {
            let entry = &*map.cast::<StringEntry>().add(index);
            (CStr::from_ptr(entry.key).to_bytes().to_vec(), entry.value)
        })
        .collect()
}

unsafe fn assert_string_maps_equal(c_map: *mut c_void, rust_map: *mut c_void) {
    let c_header = &*map_header(c_map, size_of::<StringEntry>());
    let r_header = &*map_header(rust_map, size_of::<StringEntry>());
    assert_eq!(c_header.length, r_header.length);
    assert_eq!(c_header.capacity, r_header.capacity);
    assert_eq!(c_header.temp, r_header.temp);
    assert_eq!(string_snapshot(c_map), string_snapshot(rust_map));

    let c_table = table(c_map, size_of::<StringEntry>());
    let r_table = table(rust_map, size_of::<StringEntry>());
    assert_eq!((*c_table).slot_count, (*r_table).slot_count);
    assert_eq!((*c_table).used_count, (*r_table).used_count);
    assert_eq!((*c_table).tombstone_count, (*r_table).tombstone_count);
    assert_eq!((*c_table).string.mode, (*r_table).string.mode);
    assert_eq!((*c_table).string.remaining, (*r_table).string.remaining);
    assert_eq!((*c_table).string.block, (*r_table).string.block);
}

#[test]
fn arrays_hashes_and_seed_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();

        let c_zero = (c.arrgrow)(ptr::null_mut(), 4, 0, 0);
        let r_zero = (rust.arrgrow)(ptr::null_mut(), 4, 0, 0);
        assert!(c_zero.is_null() && r_zero.is_null());

        for element_size in [1_usize, 4, size_of::<BinaryEntry>()] {
            for minimum in [1_usize, 2, 3, 4, 5, 17] {
                let c_array = (c.arrgrow)(ptr::null_mut(), element_size, 0, minimum);
                let r_array = (rust.arrgrow)(ptr::null_mut(), element_size, 0, minimum);
                assert_array_equal(c_array, r_array, 0);
                assert_eq!(
                    (*header(c_array)).capacity,
                    if minimum < 4 { 4 } else { minimum }
                );
                (c.arrfree)(c_array);
                (rust.arrfree)(r_array);
            }
        }

        let mut rng = Rng::new(0x6a09_e667_f3bc_c909);
        for iteration in 0..128 {
            let element_size = [1_usize, 4, size_of::<BinaryEntry>()][iteration % 3];
            let initial_minimum = 4 + rng.next_usize() % 24;
            let mut c_array = (c.arrgrow)(ptr::null_mut(), element_size, 0, initial_minimum);
            let mut r_array = (rust.arrgrow)(ptr::null_mut(), element_size, 0, initial_minimum);
            let length = rng.next_usize() % ((*header(c_array)).capacity + 1);
            (*header(c_array)).length = length;
            (*header(r_array)).length = length;
            let initialized = length * element_size;
            let mut bytes = vec![0_u8; initialized];
            rng.fill(&mut bytes);
            ptr::copy_nonoverlapping(bytes.as_ptr(), c_array.cast(), initialized);
            ptr::copy_nonoverlapping(bytes.as_ptr(), r_array.cast(), initialized);

            let old_c = c_array;
            let old_r = r_array;
            c_array = (c.arrgrow)(c_array, element_size, 0, length);
            r_array = (rust.arrgrow)(r_array, element_size, 0, length);
            assert_eq!(c_array, old_c);
            assert_eq!(r_array, old_r);
            assert_array_equal(c_array, r_array, initialized);

            let old_capacity = (*header(c_array)).capacity;
            let add = 1 + rng.next_usize() % (old_capacity + 7);
            let explicit_minimum = if iteration % 2 == 0 {
                0
            } else {
                old_capacity * 2 + 3 + rng.next_usize() % 17
            };
            c_array = (c.arrgrow)(c_array, element_size, add, explicit_minimum);
            r_array = (rust.arrgrow)(r_array, element_size, add, explicit_minimum);
            assert_array_equal(c_array, r_array, initialized);
            (c.arrfree)(c_array);
            (rust.arrfree)(r_array);
        }

        for seed in [0_usize, 1, 0x3141_5926, usize::MAX] {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let c_map = (c.shmode)(size_of::<StringEntry>(), SH_DEFAULT);
            let r_map = (rust.shmode)(size_of::<StringEntry>(), SH_DEFAULT);
            assert_eq!(
                (*table(c_map, size_of::<StringEntry>())).seed,
                (*table(r_map, size_of::<StringEntry>())).seed
            );
            assert_eq!((*table(c_map, size_of::<StringEntry>())).seed, seed);
            free_map(&c, c_map, size_of::<StringEntry>());
            free_map(&rust, r_map, size_of::<StringEntry>());
        }

        let strings = [
            vec![],
            b"a".to_vec(),
            b"the quick brown fox".to_vec(),
            vec![0x80, 0xff, b'x'],
        ];
        for seed in [0_usize, 1, 0x0123_4567_89ab_cdef, usize::MAX] {
            for bytes in &strings {
                let value = CString::new(bytes.clone()).unwrap();
                assert_eq!(
                    (c.hash_string)(value.as_ptr().cast_mut(), seed),
                    (rust.hash_string)(value.as_ptr().cast_mut(), seed)
                );
            }
            let mut rng = Rng::new(seed as u64 ^ 0xa54f_f53a_5f1d_36f1);
            for _ in 0..128 {
                let mut bytes = vec![0_u8; rng.next_usize() % 65];
                rng.fill(&mut bytes);
                for byte in &mut bytes {
                    if *byte == 0 {
                        *byte = 1;
                    }
                }
                let value = CString::new(bytes).unwrap();
                assert_eq!(
                    (c.hash_string)(value.as_ptr().cast_mut(), seed),
                    (rust.hash_string)(value.as_ptr().cast_mut(), seed)
                );
            }
        }

        for seed in [0_usize, 1, 0xfeed_face_cafe_beef, usize::MAX] {
            assert_eq!(
                (c.hash_bytes)(ptr::null_mut(), 0, seed),
                (rust.hash_bytes)(ptr::null_mut(), 0, seed)
            );
            let mut rng = Rng::new(seed as u64 ^ 0xbb67_ae85_84ca_a73b);
            for length in 0..=96 {
                for _ in 0..24 {
                    let mut bytes = vec![0_u8; length];
                    rng.fill(&mut bytes);
                    if length > 0 && length % 3 == 0 {
                        bytes[length / 2] |= 0x80;
                    }
                    assert_eq!(
                        (c.hash_bytes)(bytes.as_mut_ptr().cast(), length, seed),
                        (rust.hash_bytes)(bytes.as_mut_ptr().cast(), length, seed),
                        "length={length}, seed={seed:#x}, bytes={bytes:02x?}"
                    );
                }
            }
        }
    }
}

#[test]
fn binary_map_operations_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();

        let mut c_temp = 77_isize;
        let mut r_temp = 77_isize;
        let mut key = 123_u64;
        let c_created = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            &mut c_temp,
            HM_BINARY,
        );
        let r_created = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            &mut r_temp,
            HM_BINARY,
        );
        assert_eq!(c_temp, -1);
        assert_eq!(r_temp, -1);
        assert_binary_maps_equal(c_created, r_created);
        assert_eq!(
            *raw_from_map(c_created, size_of::<BinaryEntry>()).cast::<BinaryEntry>(),
            BinaryEntry {
                key: 0,
                duplicate_key: 0,
                value: 0
            }
        );
        free_map(&c, c_created, size_of::<BinaryEntry>());
        free_map(&rust, r_created, size_of::<BinaryEntry>());

        let c_created = (c.hmget)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        let r_created = (rust.hmget)(
            ptr::null_mut(),
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        assert_binary_maps_equal(c_created, r_created);
        assert_eq!((*map_header(c_created, size_of::<BinaryEntry>())).temp, -1);
        free_map(&c, c_created, size_of::<BinaryEntry>());
        free_map(&rust, r_created, size_of::<BinaryEntry>());

        let c_default = (c.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>());
        let r_default = (rust.hmput_default)(ptr::null_mut(), size_of::<BinaryEntry>());
        assert_binary_maps_equal(c_default, r_default);
        assert!(table(c_default, size_of::<BinaryEntry>()).is_null());
        let c_default_again = (c.hmput_default)(c_default, size_of::<BinaryEntry>());
        let r_default_again = (rust.hmput_default)(r_default, size_of::<BinaryEntry>());
        assert_eq!(c_default_again, c_default);
        assert_eq!(r_default_again, r_default);
        let c_after_delete = (c.hmdel)(
            c_default,
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        let r_after_delete = (rust.hmdel)(
            r_default,
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        assert_binary_maps_equal(c_after_delete, r_after_delete);
        free_map(&c, c_after_delete, size_of::<BinaryEntry>());
        free_map(&rust, r_after_delete, size_of::<BinaryEntry>());

        for key_size in [0_usize, 1, 4, 8, 16, size_of::<BinaryEntry>()] {
            let mut c_map = ptr::null_mut();
            let mut r_map = ptr::null_mut();
            let mut key_bytes = [0xa5_u8; size_of::<BinaryEntry>()];
            c_map = (c.hmput)(
                c_map,
                size_of::<BinaryEntry>(),
                key_bytes.as_mut_ptr().cast(),
                key_size,
                HM_BINARY,
            );
            r_map = (rust.hmput)(
                r_map,
                size_of::<BinaryEntry>(),
                key_bytes.as_mut_ptr().cast(),
                key_size,
                HM_BINARY,
            );
            let c_index = (*map_header(c_map, size_of::<BinaryEntry>())).temp as usize;
            let r_index = (*map_header(r_map, size_of::<BinaryEntry>())).temp as usize;
            *c_map.cast::<BinaryEntry>().add(c_index) = BinaryEntry {
                key: 0x0102_0304_0506_0708,
                duplicate_key: 0x1112_1314_1516_1718,
                value: key_size as u64,
            };
            *r_map.cast::<BinaryEntry>().add(r_index) = BinaryEntry {
                key: 0x0102_0304_0506_0708,
                duplicate_key: 0x1112_1314_1516_1718,
                value: key_size as u64,
            };
            assert_binary_maps_equal(c_map, r_map);
            free_map(&c, c_map, size_of::<BinaryEntry>());
            free_map(&rust, r_map, size_of::<BinaryEntry>());
        }

        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);
        let mut c_map = ptr::null_mut();
        let mut r_map = ptr::null_mut();
        let mut rng = Rng::new(0x3c6e_f372_fe94_f82b);
        let mut inserted = Vec::new();
        for index in 0..96_u64 {
            let key = rng.next_u64() | 1;
            inserted.push(key);
            binary_put(&c, &mut c_map, key, index * 11);
            binary_put(&rust, &mut r_map, key, index * 11);
            assert_binary_maps_equal(c_map, r_map);
        }

        for (index, &existing) in inserted.iter().enumerate().step_by(5) {
            binary_put(&c, &mut c_map, existing, 10_000 + index as u64);
            binary_put(&rust, &mut r_map, existing, 10_000 + index as u64);
            assert_binary_maps_equal(c_map, r_map);
        }

        for &probe in inserted
            .iter()
            .step_by(3)
            .chain([0_u64, 2, 4, u64::MAX].iter())
        {
            let mut c_key = probe;
            let mut r_key = probe;
            let c_result = (c.hmget)(
                c_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(c_key).cast(),
                size_of::<u64>(),
                HM_BINARY,
            );
            let r_result = (rust.hmget)(
                r_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(r_key).cast(),
                size_of::<u64>(),
                HM_BINARY,
            );
            c_map = c_result;
            r_map = r_result;
            assert_binary_maps_equal(c_map, r_map);

            (*map_header(c_map, size_of::<BinaryEntry>())).temp = 444;
            (*map_header(r_map, size_of::<BinaryEntry>())).temp = 444;
            let mut c_ts = 0;
            let mut r_ts = 0;
            c_map = (c.hmget_ts)(
                c_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(c_key).cast(),
                size_of::<u64>(),
                &mut c_ts,
                HM_BINARY,
            );
            r_map = (rust.hmget_ts)(
                r_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(r_key).cast(),
                size_of::<u64>(),
                &mut r_ts,
                HM_BINARY,
            );
            assert_eq!(c_ts, r_ts);
            assert_eq!((*map_header(c_map, size_of::<BinaryEntry>())).temp, 444);
            assert_eq!((*map_header(r_map, size_of::<BinaryEntry>())).temp, 444);
        }

        let mut missing = 0_u64;
        c_map = (c.hmdel)(
            c_map,
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(missing).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        r_map = (rust.hmdel)(
            r_map,
            size_of::<BinaryEntry>(),
            ptr::addr_of_mut!(missing).cast(),
            size_of::<u64>(),
            0,
            HM_BINARY,
        );
        assert_binary_maps_equal(c_map, r_map);
        assert_eq!((*map_header(c_map, size_of::<BinaryEntry>())).temp, 0);

        for &deleted in inserted.iter().take(72) {
            let mut c_key = deleted;
            let mut r_key = deleted;
            c_map = (c.hmdel)(
                c_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(c_key).cast(),
                size_of::<u64>(),
                if deleted == inserted[10] {
                    size_of::<u64>()
                } else {
                    0
                },
                HM_BINARY,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<BinaryEntry>(),
                ptr::addr_of_mut!(r_key).cast(),
                size_of::<u64>(),
                if deleted == inserted[10] {
                    size_of::<u64>()
                } else {
                    0
                },
                HM_BINARY,
            );
            assert_binary_maps_equal(c_map, r_map);
        }

        for index in 0..40_u64 {
            let key = 0x8000_0000_0000_0000 | index;
            binary_put(&c, &mut c_map, key, index);
            binary_put(&rust, &mut r_map, key, index);
            assert_binary_maps_equal(c_map, r_map);
        }
        free_map(&c, c_map, size_of::<BinaryEntry>());
        free_map(&rust, r_map, size_of::<BinaryEntry>());
    }
}

#[test]
fn string_map_modes_and_operations_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();

        for mode in [
            c_int::MIN,
            -1,
            SH_NONE,
            SH_DEFAULT,
            SH_STRDUP,
            SH_ARENA,
            257,
            258,
            c_int::MAX,
        ] {
            (c.rand_seed)(0x55aa);
            (rust.rand_seed)(0x55aa);
            let c_map = (c.shmode)(size_of::<StringEntry>(), mode);
            let r_map = (rust.shmode)(size_of::<StringEntry>(), mode);
            assert_eq!(
                (*table(c_map, size_of::<StringEntry>())).string.mode,
                (*table(r_map, size_of::<StringEntry>())).string.mode
            );
            assert_eq!(
                (*table(c_map, size_of::<StringEntry>())).string.mode,
                mode as u8
            );
            free_map(&c, c_map, size_of::<StringEntry>());
            free_map(&rust, r_map, size_of::<StringEntry>());
        }

        for storage_mode in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(0x9e37_79b9);
            (rust.rand_seed)(0x9e37_79b9);
            let mut c_map = (c.shmode)(size_of::<StringEntry>(), storage_mode);
            let mut r_map = (rust.shmode)(size_of::<StringEntry>(), storage_mode);
            let mut keys = Vec::new();
            let mut rng = Rng::new(0x510e_527f_ade6_82d1 ^ storage_mode as u64);
            for index in 0..64 {
                let bytes = if index == 0 {
                    Vec::new()
                } else if index == 1 {
                    vec![0x80, 0xfe]
                } else {
                    let mut bytes =
                        format!("key_{index:03}_{}", "x".repeat(index % 19)).into_bytes();
                    let mut random_suffix = rng.next_u64().to_le_bytes();
                    for byte in &mut random_suffix {
                        if *byte == 0 {
                            *byte = 1;
                        }
                    }
                    bytes.extend_from_slice(&random_suffix);
                    bytes
                };
                keys.push(CString::new(bytes).unwrap());
                string_put(
                    &c,
                    &mut c_map,
                    keys.last().unwrap().as_ptr().cast_mut(),
                    index as c_int,
                );
                string_put(
                    &rust,
                    &mut r_map,
                    keys.last().unwrap().as_ptr().cast_mut(),
                    index as c_int,
                );
                assert_string_maps_equal(c_map, r_map);
            }

            for index in (0..keys.len()).step_by(7) {
                string_put(
                    &c,
                    &mut c_map,
                    keys[index].as_ptr().cast_mut(),
                    10_000 + index as c_int,
                );
                string_put(
                    &rust,
                    &mut r_map,
                    keys[index].as_ptr().cast_mut(),
                    10_000 + index as c_int,
                );
                assert_string_maps_equal(c_map, r_map);
            }

            let missing = CString::new("not_present").unwrap();
            for key in keys.iter().step_by(5).chain(std::iter::once(&missing)) {
                c_map = (c.hmget)(
                    c_map,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
                r_map = (rust.hmget)(
                    r_map,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
                assert_string_maps_equal(c_map, r_map);

                let mut c_temp = 99;
                let mut r_temp = 99;
                c_map = (c.hmget_ts)(
                    c_map,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut c_temp,
                    HM_STRING,
                );
                r_map = (rust.hmget_ts)(
                    r_map,
                    size_of::<StringEntry>(),
                    key.as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut r_temp,
                    HM_STRING,
                );
                assert_eq!(c_temp, r_temp);
            }

            c_map = (c.hmdel)(
                c_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                HM_STRING,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                HM_STRING,
            );
            assert_string_maps_equal(c_map, r_map);

            for index in (0..keys.len()).take(48) {
                c_map = (c.hmdel)(
                    c_map,
                    size_of::<StringEntry>(),
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                );
                r_map = (rust.hmdel)(
                    r_map,
                    size_of::<StringEntry>(),
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    0,
                    HM_STRING,
                );
                assert_string_maps_equal(c_map, r_map);
            }

            free_map(&c, c_map, size_of::<StringEntry>());
            free_map(&rust, r_map, size_of::<StringEntry>());
        }

        (c.rand_seed)(123);
        (rust.rand_seed)(123);
        let mut c_map = ptr::null_mut();
        let mut r_map = ptr::null_mut();
        let key = CString::new("implicit_default").unwrap();
        string_put(&c, &mut c_map, key.as_ptr().cast_mut(), 7);
        string_put(&rust, &mut r_map, key.as_ptr().cast_mut(), 7);
        assert_string_maps_equal(c_map, r_map);
        assert_eq!(
            (*table(c_map, size_of::<StringEntry>())).string.mode,
            SH_DEFAULT as u8
        );
        free_map(&c, c_map, size_of::<StringEntry>());
        free_map(&rust, r_map, size_of::<StringEntry>());

        (c.hmfree)(ptr::null_mut(), size_of::<StringEntry>());
        (rust.hmfree)(ptr::null_mut(), size_of::<StringEntry>());
    }
}

unsafe fn compare_arena_allocation(
    c: &Api,
    rust: &Api,
    c_arena: &mut StringArena,
    r_arena: &mut StringArena,
    bytes_without_nul: Vec<u8>,
) {
    let value = CString::new(bytes_without_nul).unwrap();
    let c_result = (c.stralloc)(c_arena, value.as_ptr().cast_mut());
    let r_result = (rust.stralloc)(r_arena, value.as_ptr().cast_mut());
    assert_eq!(CStr::from_ptr(c_result), CStr::from_ptr(r_result));
    assert_eq!(c_arena.remaining, r_arena.remaining);
    assert_eq!(c_arena.block, r_arena.block);
    assert_eq!(block_count(c_arena), block_count(r_arena));
}

#[test]
fn arenas_strkey_and_driver_stdout_match() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    unsafe {
        let (c, rust) = load_pair();

        let mut c_empty = StringArena::default();
        let mut r_empty = StringArena::default();
        (c.strreset)(&mut c_empty);
        (rust.strreset)(&mut r_empty);
        assert!(c_empty.storage.is_null() && r_empty.storage.is_null());

        let mut c_arena = StringArena::default();
        let mut r_arena = StringArena::default();
        compare_arena_allocation(&c, &rust, &mut c_arena, &mut r_arena, Vec::new());
        let mut rng = Rng::new(0x1f83_d9ab_fb41_bd6b);
        for index in 0..180 {
            let mut bytes = vec![0_u8; 1 + index % 31];
            rng.fill(&mut bytes);
            for byte in &mut bytes {
                if *byte == 0 {
                    *byte = 1;
                }
            }
            compare_arena_allocation(&c, &rust, &mut c_arena, &mut r_arena, bytes);
        }
        compare_arena_allocation(&c, &rust, &mut c_arena, &mut r_arena, vec![b'L'; 5000]);
        assert!(block_count(&c_arena) > 1);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
        assert!(c_arena.storage.is_null() && r_arena.storage.is_null());
        assert_eq!(c_arena.remaining, 0);
        assert_eq!(r_arena.remaining, 0);
        assert_eq!(c_arena.block, 0);
        assert_eq!(r_arena.block, 0);

        for total_length in [
            1_usize,
            2,
            511,
            512,
            513,
            1023,
            1024,
            1025,
            (1 << 20) - 1,
            1 << 20,
            (1 << 20) + 1,
        ] {
            let mut c_arena = StringArena::default();
            let mut r_arena = StringArena::default();
            compare_arena_allocation(
                &c,
                &rust,
                &mut c_arena,
                &mut r_arena,
                vec![b'b'; total_length - 1],
            );
            (c.strreset)(&mut c_arena);
            (rust.strreset)(&mut r_arena);
        }

        let mut c_arena = StringArena::default();
        let mut r_arena = StringArena::default();
        compare_arena_allocation(&c, &rust, &mut c_arena, &mut r_arena, b"head".to_vec());
        compare_arena_allocation(&c, &rust, &mut c_arena, &mut r_arena, vec![b'z'; 8192]);
        assert_eq!(block_count(&c_arena), 2);
        assert_eq!(block_count(&r_arena), 2);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);

        for number in [c_int::MIN, -1000, -1, 0, 1, 42, c_int::MAX] {
            let c_value = (c.strkey)(number);
            let r_value = (rust.strkey)(number);
            assert_eq!(CStr::from_ptr(c_value), CStr::from_ptr(r_value));
            assert_eq!(
                CStr::from_ptr(c_value).to_bytes(),
                format!("test_{number}").as_bytes()
            );
        }
        let c_first = (c.strkey)(1);
        let r_first = (rust.strkey)(1);
        let c_second = (c.strkey)(2);
        let r_second = (rust.strkey)(2);
        assert_eq!(c_first, c_second);
        assert_eq!(r_first, r_second);
        assert_eq!(CStr::from_ptr(c_first).to_bytes(), b"test_2");
        assert_eq!(CStr::from_ptr(r_first).to_bytes(), b"test_2");

        let mut randomized_numbers = vec![-7, 0, 1, 63, 64, 65, 1000];
        let mut rng = Rng::new(0x5be0_cd19_137e_2179);
        for _ in 0..128 {
            let number = rng.next_u64() as c_int;
            let c_value = (c.strkey)(number);
            let r_value = (rust.strkey)(number);
            assert_eq!(CStr::from_ptr(c_value), CStr::from_ptr(r_value));
        }
        for _ in 0..12 {
            randomized_numbers.push((rng.next_u64() % 200) as c_int);
        }
        for number in randomized_numbers {
            let (c_output, c_status) = capture_stdout(|| (c.sh_puts)(number));
            let (r_output, r_status) = capture_stdout(|| (rust.sh_puts)(number));
            assert_eq!(c_status, r_status);
            assert_eq!(c_status.exit_code, 0);
            assert_eq!(c_output, r_output, "num={number}");
            assert_eq!(c_output, format!("a {number}\n").as_bytes());
        }
    }
}
