mod common;

use common::*;
use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe fn apis() -> (Api, Api) {
    let (c_path, rust_path) = library_paths();
    assert!(c_path.is_file(), "missing C library: {}", c_path.display());
    assert!(
        rust_path.is_file(),
        "missing Rust library: {}",
        rust_path.display()
    );
    (unsafe { Api::load(&c_path) }, unsafe {
        Api::load(&rust_path)
    })
}

unsafe fn array_snapshot(data: *mut c_void, elem_size: usize) -> (usize, usize, isize, Vec<u8>) {
    let header = unsafe { &*header(data) };
    let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), header.length * elem_size) }
        .to_vec();
    (header.length, header.capacity, header.temp, bytes)
}

#[test]
fn arrays_and_allocation_branches_match() {
    unsafe {
        let (c, rust) = apis();
        let mut rng = Rng::new(0x7f4a_7c15_d1ce_ba11);

        for elem_size in [1usize, 4, 8, 16] {
            for &(add_len, min_cap) in &[(0, 0), (0, 1), (3, 1), (0, 17)] {
                let c_array = (c.arrgrow)(ptr::null_mut(), elem_size, add_len, min_cap);
                let r_array = (rust.arrgrow)(ptr::null_mut(), elem_size, add_len, min_cap);
                if c_array.is_null() || r_array.is_null() {
                    assert!(c_array.is_null());
                    assert!(r_array.is_null());
                    continue;
                }
                assert_eq!(
                    array_snapshot(c_array, elem_size),
                    array_snapshot(r_array, elem_size)
                );
                (c.arrfree)(c_array);
                (rust.arrfree)(r_array);
            }

            let mut c_array = (c.arrgrow)(ptr::null_mut(), elem_size, 0, 5);
            let mut r_array = (rust.arrgrow)(ptr::null_mut(), elem_size, 0, 5);
            (*header(c_array)).length = 3;
            (*header(r_array)).length = 3;
            let mut contents = vec![0; elem_size * 3];
            rng.fill(&mut contents);
            ptr::copy_nonoverlapping(contents.as_ptr(), c_array.cast(), contents.len());
            ptr::copy_nonoverlapping(contents.as_ptr(), r_array.cast(), contents.len());

            let old_c = c_array;
            let old_r = r_array;
            c_array = (c.arrgrow)(c_array, elem_size, 0, 4);
            r_array = (rust.arrgrow)(r_array, elem_size, 0, 4);
            assert_eq!(c_array, old_c);
            assert_eq!(r_array, old_r);
            assert_eq!(
                array_snapshot(c_array, elem_size),
                array_snapshot(r_array, elem_size)
            );

            c_array = (c.arrgrow)(c_array, elem_size, 3, 0);
            r_array = (rust.arrgrow)(r_array, elem_size, 3, 0);
            assert_eq!((*header(c_array)).capacity, 10);
            assert_eq!(
                array_snapshot(c_array, elem_size),
                array_snapshot(r_array, elem_size)
            );

            c_array = (c.arrgrow)(c_array, elem_size, 0, 25);
            r_array = (rust.arrgrow)(r_array, elem_size, 0, 25);
            assert_eq!((*header(c_array)).capacity, 25);
            assert_eq!(
                array_snapshot(c_array, elem_size),
                array_snapshot(r_array, elem_size)
            );
            (c.arrfree)(c_array);
            (rust.arrfree)(r_array);
        }
    }
}

#[test]
fn hash_functions_match_for_all_tail_shapes_and_values() {
    unsafe {
        let (c, rust) = apis();
        let mut rng = Rng::new(0x243f_6a88_85a3_08d3);
        let seeds = [0, 1, usize::MAX, 0x3141_5926, rng.next_u64() as usize];

        for seed in seeds {
            let mut empty = [0u8; 1];
            assert_eq!(
                (c.hash_bytes)(empty.as_mut_ptr().cast(), 0, seed),
                (rust.hash_bytes)(empty.as_mut_ptr().cast(), 0, seed)
            );
            assert_eq!(
                (c.hash_bytes)(ptr::null_mut(), 0, seed),
                (rust.hash_bytes)(ptr::null_mut(), 0, seed)
            );
            for len in 1..=79 {
                for _ in 0..24 {
                    let mut bytes = vec![0u8; len];
                    rng.fill(&mut bytes);
                    if len % 8 == 4 {
                        bytes[len - 1] |= 0x80;
                    }
                    assert_eq!(
                        (c.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed),
                        (rust.hash_bytes)(bytes.as_mut_ptr().cast(), len, seed),
                        "byte hash mismatch at len={len}, seed={seed:#x}"
                    );
                }
            }

            let string_cases = [
                vec![],
                vec![1],
                vec![0x7f],
                vec![0xff],
                b"multiple ascii bytes".to_vec(),
                vec![0x80, 0xfe, b'a', 0x81],
            ];
            for mut bytes in string_cases {
                bytes.push(0);
                assert_eq!(
                    (c.hash_string)(bytes.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(bytes.as_mut_ptr().cast(), seed)
                );
            }
            for _ in 0..128 {
                let len = (rng.next_u64() % 47 + 1) as usize;
                let mut bytes = vec![0u8; len + 1];
                for byte in &mut bytes[..len] {
                    *byte = (rng.next_u64() as u8).max(1);
                }
                assert_eq!(
                    (c.hash_string)(bytes.as_mut_ptr().cast(), seed),
                    (rust.hash_string)(bytes.as_mut_ptr().cast(), seed)
                );
            }
        }
    }
}

#[test]
fn seed_modes_and_default_maps_match() {
    unsafe {
        let (c, rust) = apis();
        for seed in [0, 1, usize::MAX, 0xdead_beef_cafe_babe] {
            (c.rand_seed)(seed);
            (rust.rand_seed)(seed);
            let c_map = (c.shmode)(size_of::<BinEntry>(), SH_NONE);
            let r_map = (rust.shmode)(size_of::<BinEntry>(), SH_NONE);
            assert_eq!(
                table_snapshot(c_map, size_of::<BinEntry>()),
                table_snapshot(r_map, size_of::<BinEntry>())
            );
            free_bin_map(&c, c_map);
            free_bin_map(&rust, r_map);
        }

        for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, -1, 4, 256] {
            (c.rand_seed)(99);
            (rust.rand_seed)(99);
            let c_map = (c.shmode)(size_of::<StringEntry>(), mode);
            let r_map = (rust.shmode)(size_of::<StringEntry>(), mode);
            assert_eq!(
                table_snapshot(c_map, size_of::<StringEntry>()),
                table_snapshot(r_map, size_of::<StringEntry>()),
                "mode {mode}"
            );
            free_string_map(&c, c_map);
            free_string_map(&rust, r_map);
        }

        let mut c_map = (c.hmput_default)(ptr::null_mut(), size_of::<BinEntry>());
        let mut r_map = (rust.hmput_default)(ptr::null_mut(), size_of::<BinEntry>());
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        let old_c = c_map;
        let old_r = r_map;
        c_map = (c.hmput_default)(c_map, size_of::<BinEntry>());
        r_map = (rust.hmput_default)(r_map, size_of::<BinEntry>());
        assert_eq!(c_map, old_c);
        assert_eq!(r_map, old_r);
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);

        let c_raw = (c.arrgrow)(ptr::null_mut(), size_of::<BinEntry>(), 0, 1);
        let r_raw = (rust.arrgrow)(ptr::null_mut(), size_of::<BinEntry>(), 0, 1);
        let c_map = (c.hmput_default)(
            c_raw.cast::<u8>().add(size_of::<BinEntry>()).cast(),
            size_of::<BinEntry>(),
        );
        let r_map = (rust.hmput_default)(
            r_raw.cast::<u8>().add(size_of::<BinEntry>()).cast(),
            size_of::<BinEntry>(),
        );
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);

        let mut key = 0x1020_3040_5060_7080u64;
        let c_map = (c.hmput)(
            ptr::null_mut(),
            size_of::<u64>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        let r_map = (rust.hmput)(
            ptr::null_mut(),
            size_of::<u64>(),
            ptr::addr_of_mut!(key).cast(),
            size_of::<u64>(),
            HM_BINARY,
        );
        assert_eq!(
            std::slice::from_raw_parts(c_map.cast::<u8>(), 8),
            std::slice::from_raw_parts(r_map.cast::<u8>(), 8)
        );
        assert_eq!(
            (
                (*map_header(c_map, 8)).length,
                (*map_header(c_map, 8)).capacity,
                (*map_header(c_map, 8)).temp,
                table_snapshot(c_map, 8),
            ),
            (
                (*map_header(r_map, 8)).length,
                (*map_header(r_map, 8)).capacity,
                (*map_header(r_map, 8)).temp,
                table_snapshot(r_map, 8),
            )
        );
        (c.hmfree)(raw_map(c_map, 8), 8);
        (rust.hmfree)(raw_map(r_map, 8), 8);
    }
}

#[test]
fn binary_map_insert_get_growth_collision_and_delete_match() {
    unsafe {
        let (c, rust) = apis();
        (c.rand_seed)(0x1234_5678);
        (rust.rand_seed)(0x1234_5678);
        let mut c_map = ptr::null_mut();
        let mut r_map = ptr::null_mut();
        let mut rng = Rng::new(0xa409_3822_299f_31d0);

        for index in 0..80 {
            let key = if index == 0 { 7 } else { rng.next_u64() | 1 };
            let value = rng.next_u64() as i64;
            c_map = put_bin(&c, c_map, key, value);
            r_map = put_bin(&rust, r_map, key, value);
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        }
        assert!(
            table_snapshot(c_map, size_of::<BinEntry>())
                .unwrap()
                .slot_count
                >= 128
        );

        let repeated = (*c_map.cast::<BinEntry>().add(12)).key;
        let old_len = (*map_header(c_map, size_of::<BinEntry>())).length;
        c_map = put_bin(&c, c_map, repeated, -7788);
        r_map = put_bin(&rust, r_map, repeated, -7788);
        assert_eq!((*map_header(c_map, size_of::<BinEntry>())).length, old_len);
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));

        for index in [0usize, 7, 19, 43, 79] {
            let mut key = (*c_map.cast::<BinEntry>().add(index)).key;
            let mut c_temp = 99;
            let mut r_temp = 99;
            let c_result = (c.hmget_ts)(
                c_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                &mut c_temp,
                HM_BINARY,
            );
            let r_result = (rust.hmget_ts)(
                r_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                &mut r_temp,
                HM_BINARY,
            );
            assert_eq!(c_result, c_map);
            assert_eq!(r_result, r_map);
            assert_eq!(c_temp, r_temp);
            assert_eq!(
                *c_map.cast::<BinEntry>().add(c_temp as usize),
                *r_map.cast::<BinEntry>().add(r_temp as usize)
            );

            (c.hmget)(
                c_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                HM_BINARY,
            );
            (rust.hmget)(
                r_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                size_of::<u64>(),
                HM_BINARY,
            );
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        }

        for mut missing in [0, 2, 4, u64::MAX] {
            let mut c_temp = 77;
            let mut r_temp = 77;
            (c.hmget_ts)(
                c_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(missing).cast(),
                8,
                &mut c_temp,
                HM_BINARY,
            );
            (rust.hmget_ts)(
                r_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(missing).cast(),
                8,
                &mut r_temp,
                HM_BINARY,
            );
            assert_eq!((c_temp, r_temp), (-1, -1));
            (c.hmget)(
                c_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(missing).cast(),
                8,
                HM_BINARY,
            );
            (rust.hmget)(
                r_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(missing).cast(),
                8,
                HM_BINARY,
            );
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        }

        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);

        (c.rand_seed)(0x55aa);
        (rust.rand_seed)(0x55aa);
        c_map = ptr::null_mut();
        r_map = ptr::null_mut();
        let seed = 0x55aa;
        let mut colliding = Vec::new();
        let mut candidate = 1u64;
        while colliding.len() < 6 {
            let hash = (c.hash_bytes)(ptr::addr_of_mut!(candidate).cast(), 8, seed);
            let normalized = if hash < 2 { hash + 2 } else { hash };
            if normalized & 7 == 7 {
                colliding.push(candidate);
            }
            candidate += 1;
        }
        for (index, key) in colliding.iter().copied().enumerate() {
            c_map = put_bin(&c, c_map, key, index as i64);
            r_map = put_bin(&rust, r_map, key, index as i64);
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        }
        let mut absent = candidate;
        loop {
            let hash = (c.hash_bytes)(ptr::addr_of_mut!(absent).cast(), 8, seed);
            if (if hash < 2 { hash + 2 } else { hash }) & 7 == 7 {
                break;
            }
            absent += 1;
        }
        let mut c_temp = 0;
        let mut r_temp = 0;
        (c.hmget_ts)(
            c_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(absent).cast(),
            8,
            &mut c_temp,
            HM_BINARY,
        );
        (rust.hmget_ts)(
            r_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(absent).cast(),
            8,
            &mut r_temp,
            HM_BINARY,
        );
        assert_eq!((c_temp, r_temp), (-1, -1));

        let mut deleted = colliding[2];
        c_map = (c.hmdel)(
            c_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(deleted).cast(),
            8,
            0,
            HM_BINARY,
        );
        r_map = (rust.hmdel)(
            r_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(deleted).cast(),
            8,
            0,
            HM_BINARY,
        );
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        c_map = put_bin(&c, c_map, absent, 9123);
        r_map = put_bin(&rust, r_map, absent, 9123);
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);
    }
}

#[test]
fn null_default_and_missing_map_results_match() {
    unsafe {
        let (c, rust) = apis();
        (c.hmfree)(ptr::null_mut(), size_of::<BinEntry>());
        (rust.hmfree)(ptr::null_mut(), size_of::<BinEntry>());

        let mut key = 42u64;
        let mut c_temp = 55;
        let mut r_temp = 55;
        let mut c_map = (c.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut c_temp,
            HM_BINARY,
        );
        let mut r_map = (rust.hmget_ts)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut r_temp,
            HM_BINARY,
        );
        assert_eq!((c_temp, r_temp), (-1, -1));
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        (c.hmget)(
            c_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            HM_BINARY,
        );
        (rust.hmget)(
            r_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            HM_BINARY,
        );
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);

        c_map = (c.hmget)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            HM_BINARY,
        );
        r_map = (rust.hmget)(
            ptr::null_mut(),
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            HM_BINARY,
        );
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        assert_eq!((*map_header(c_map, size_of::<BinEntry>())).temp, -1);
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);

        c_map = (c.hmput_default)(ptr::null_mut(), size_of::<BinEntry>());
        r_map = (rust.hmput_default)(ptr::null_mut(), size_of::<BinEntry>());
        (c.hmget_ts)(
            c_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut c_temp,
            HM_BINARY,
        );
        (rust.hmget_ts)(
            r_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            &mut r_temp,
            HM_BINARY,
        );
        assert_eq!((c_temp, r_temp), (-1, -1));
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));

        let old_c = c_map;
        let old_r = r_map;
        c_map = (c.hmdel)(
            c_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            0,
            HM_BINARY,
        );
        r_map = (rust.hmdel)(
            r_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(key).cast(),
            8,
            0,
            HM_BINARY,
        );
        assert_eq!(c_map, old_c);
        assert_eq!(r_map, old_r);
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);

        assert!((c.hmdel)(ptr::null_mut(), 16, ptr::null_mut(), 0, 0, HM_BINARY).is_null());
        assert!((rust.hmdel)(ptr::null_mut(), 16, ptr::null_mut(), 0, 0, HM_BINARY).is_null());
    }
}

#[test]
fn string_map_ownership_get_update_and_delete_match() {
    unsafe {
        let (c, rust) = apis();
        for ownership in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(0xabcdef);
            (rust.rand_seed)(0xabcdef);
            let mut c_map = (c.shmode)(size_of::<StringEntry>(), ownership);
            let mut r_map = (rust.shmode)(size_of::<StringEntry>(), ownership);
            let keys: Vec<CString> = (0..28)
                .map(|index| CString::new(format!("key_{ownership}_{index:03}")).unwrap())
                .collect();

            for (index, key) in keys.iter().enumerate() {
                c_map = put_string(
                    &c,
                    c_map,
                    key.as_ptr().cast_mut(),
                    index as i64 * -17,
                    HM_STRING,
                );
                r_map = put_string(
                    &rust,
                    r_map,
                    key.as_ptr().cast_mut(),
                    index as i64 * -17,
                    HM_STRING,
                );
                assert_eq!(
                    string_map_snapshot(c_map),
                    string_map_snapshot(r_map),
                    "ownership={ownership}, insertion={index}"
                );
            }

            c_map = put_string(&c, c_map, keys[7].as_ptr().cast_mut(), 998_877, HM_STRING);
            r_map = put_string(
                &rust,
                r_map,
                keys[7].as_ptr().cast_mut(),
                998_877,
                HM_STRING,
            );
            assert_eq!(string_map_snapshot(c_map), string_map_snapshot(r_map));

            for index in [0usize, 7, 15, 27] {
                let mut c_temp = 77;
                let mut r_temp = 77;
                (c.hmget_ts)(
                    c_map,
                    size_of::<StringEntry>(),
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut c_temp,
                    HM_STRING,
                );
                (rust.hmget_ts)(
                    r_map,
                    size_of::<StringEntry>(),
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    &mut r_temp,
                    HM_STRING,
                );
                assert_eq!(c_temp, r_temp);
                (c.hmget)(
                    c_map,
                    size_of::<StringEntry>(),
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
                (rust.hmget)(
                    r_map,
                    size_of::<StringEntry>(),
                    keys[index].as_ptr().cast_mut().cast(),
                    size_of::<*mut c_char>(),
                    HM_STRING,
                );
                assert_eq!(string_map_snapshot(c_map), string_map_snapshot(r_map));
            }

            let missing = CString::new("definitely_missing").unwrap();
            let mut c_temp = 0;
            let mut r_temp = 0;
            (c.hmget_ts)(
                c_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                &mut c_temp,
                HM_STRING,
            );
            (rust.hmget_ts)(
                r_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                &mut r_temp,
                HM_STRING,
            );
            assert_eq!((c_temp, r_temp), (-1, -1));
            (c.hmget)(
                c_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                HM_STRING,
            );
            (rust.hmget)(
                r_map,
                size_of::<StringEntry>(),
                missing.as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                HM_STRING,
            );
            assert_eq!(string_map_snapshot(c_map), string_map_snapshot(r_map));

            for index in [9usize, 27] {
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
                assert_eq!(string_map_snapshot(c_map), string_map_snapshot(r_map));
            }
            free_string_map(&c, c_map);
            free_string_map(&rust, r_map);
        }

        for ownership in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (c.rand_seed)(8080);
            (rust.rand_seed)(8080);
            let keys = [
                CString::new("first").unwrap(),
                CString::new("final").unwrap(),
            ];
            let mut c_map = (c.shmode)(size_of::<StringEntry>(), ownership);
            let mut r_map = (rust.shmode)(size_of::<StringEntry>(), ownership);
            for (index, key) in keys.iter().enumerate() {
                c_map = put_string(&c, c_map, key.as_ptr().cast_mut(), index as i64, HM_STRING);
                r_map = put_string(
                    &rust,
                    r_map,
                    key.as_ptr().cast_mut(),
                    index as i64,
                    HM_STRING,
                );
            }
            c_map = (c.hmdel)(
                c_map,
                size_of::<StringEntry>(),
                keys[1].as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                HM_STRING,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<StringEntry>(),
                keys[1].as_ptr().cast_mut().cast(),
                size_of::<*mut c_char>(),
                0,
                HM_STRING,
            );
            assert_eq!(string_map_snapshot(c_map), string_map_snapshot(r_map));
            free_string_map(&c, c_map);
            free_string_map(&rust, r_map);
        }

        for mode in [2, 7, c_int::MAX] {
            (c.rand_seed)(31337);
            (rust.rand_seed)(31337);
            let key = CString::new(format!("mode-{mode}")).unwrap();
            let c_map = put_string(&c, ptr::null_mut(), key.as_ptr().cast_mut(), -91, mode);
            let r_map = put_string(&rust, ptr::null_mut(), key.as_ptr().cast_mut(), -91, mode);
            assert_eq!(string_map_snapshot(c_map), string_map_snapshot(r_map));
            free_string_map(&c, c_map);
            free_string_map(&rust, r_map);
        }

        for mode in [-1, c_int::MIN] {
            (c.rand_seed)(44);
            (rust.rand_seed)(44);
            let mut key = 0x8877_6655_4433_2211u64;
            let c_map = (c.hmput)(
                ptr::null_mut(),
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                8,
                mode,
            );
            let r_map = (rust.hmput)(
                ptr::null_mut(),
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                8,
                mode,
            );
            (*c_map.cast::<BinEntry>()).value = -5;
            (*r_map.cast::<BinEntry>()).value = -5;
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
            free_bin_map(&c, c_map);
            free_bin_map(&rust, r_map);
        }
    }
}

#[test]
fn deletion_move_shrink_and_rebuild_match() {
    unsafe {
        let (c, rust) = apis();
        for &(count, delete_count, expected_slots) in &[(13usize, 6usize, 16usize), (10, 4, 16)] {
            (c.rand_seed)(0x2024);
            (rust.rand_seed)(0x2024);
            let mut c_map = ptr::null_mut();
            let mut r_map = ptr::null_mut();
            for key in 0..count as u64 {
                c_map = put_bin(&c, c_map, key * 19 + 3, key as i64);
                r_map = put_bin(&rust, r_map, key * 19 + 3, key as i64);
            }
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));

            let mut missing = u64::MAX;
            let old_c = c_map;
            let old_r = r_map;
            c_map = (c.hmdel)(
                c_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(missing).cast(),
                8,
                0,
                HM_BINARY,
            );
            r_map = (rust.hmdel)(
                r_map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(missing).cast(),
                8,
                0,
                HM_BINARY,
            );
            assert_eq!(c_map, old_c);
            assert_eq!(r_map, old_r);
            assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));

            for _ in 0..delete_count {
                let mut key = (*c_map.cast::<BinEntry>()).key;
                c_map = (c.hmdel)(
                    c_map,
                    size_of::<BinEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    8,
                    0,
                    HM_BINARY,
                );
                r_map = (rust.hmdel)(
                    r_map,
                    size_of::<BinEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    8,
                    0,
                    HM_BINARY,
                );
                assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
            }
            let snapshot = table_snapshot(c_map, size_of::<BinEntry>()).unwrap();
            assert_eq!(snapshot.slot_count, expected_slots);
            assert_eq!(snapshot.tombstone_count, 0);
            free_bin_map(&c, c_map);
            free_bin_map(&rust, r_map);
        }

        let c_raw = (c.arrgrow)(ptr::null_mut(), size_of::<BinEntry>(), 0, 2);
        let r_raw = (rust.arrgrow)(ptr::null_mut(), size_of::<BinEntry>(), 0, 2);
        (c.hmfree)(c_raw, size_of::<BinEntry>());
        (rust.hmfree)(r_raw, size_of::<BinEntry>());

        let mut c_map = ptr::null_mut();
        let mut r_map = ptr::null_mut();
        for key in [1u64, 2] {
            c_map = put_bin(&c, c_map, key, key as i64);
            r_map = put_bin(&rust, r_map, key, key as i64);
        }
        let mut final_key = 2u64;
        c_map = (c.hmdel)(
            c_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(final_key).cast(),
            8,
            0,
            HM_BINARY,
        );
        r_map = (rust.hmdel)(
            r_map,
            size_of::<BinEntry>(),
            ptr::addr_of_mut!(final_key).cast(),
            8,
            0,
            HM_BINARY,
        );
        assert_eq!(bin_map_snapshot(c_map), bin_map_snapshot(r_map));
        free_bin_map(&c, c_map);
        free_bin_map(&rust, r_map);
    }
}

fn arena_shape(arena: &StringArena) -> (bool, usize, u8, u8) {
    (
        !arena.storage.is_null(),
        arena.remaining,
        arena.block,
        arena.mode,
    )
}

#[test]
fn string_arena_boundaries_and_reset_match() {
    unsafe {
        let (c, rust) = apis();
        let mut c_arena = StringArena::default();
        let mut r_arena = StringArena::default();
        let mut rng = Rng::new(0x1319_8a2e_0370_7344);

        let mut cases = vec![
            CString::new("").unwrap(),
            CString::new("a").unwrap(),
            CString::new(vec![b'x'; 509]).unwrap(),
            CString::new(vec![b'y'; 511]).unwrap(),
            CString::new(vec![b'z'; 512]).unwrap(),
            CString::new(vec![b'q'; 700]).unwrap(),
        ];
        for _ in 0..100 {
            let len = (rng.next_u64() % 73 + 1) as usize;
            let mut bytes = vec![0u8; len];
            for byte in &mut bytes {
                *byte = (rng.next_u64() as u8 % 127).max(1);
            }
            cases.push(CString::new(bytes).unwrap());
        }

        for string in &cases {
            let c_result = (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut());
            let r_result = (rust.stralloc)(&mut r_arena, string.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));
        }
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
        assert_eq!(arena_shape(&c_arena), (false, 0, 0, 0));
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));

        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));

        let exact = CString::new(vec![b'e'; 511]).unwrap();
        let c_result = (c.stralloc)(&mut c_arena, exact.as_ptr().cast_mut());
        let r_result = (rust.stralloc)(&mut r_arena, exact.as_ptr().cast_mut());
        assert_eq!(
            CStr::from_ptr(c_result).to_bytes(),
            CStr::from_ptr(r_result).to_bytes()
        );
        assert_eq!(c_arena.remaining, 0);
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);

        let dedicated = CString::new(vec![b'd'; 700]).unwrap();
        let c_result = (c.stralloc)(&mut c_arena, dedicated.as_ptr().cast_mut());
        let r_result = (rust.stralloc)(&mut r_arena, dedicated.as_ptr().cast_mut());
        assert_eq!(
            CStr::from_ptr(c_result).to_bytes(),
            CStr::from_ptr(r_result).to_bytes()
        );
        assert_eq!(c_arena.remaining, 0);
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);

        for iteration in 0..24 {
            let block_size = 512usize << ((c_arena.block as usize) >> 1);
            let capped = block_size.min(1 << 20);
            let len = capped.saturating_sub(1).max(1);
            let string = CString::new(vec![b'a' + (iteration % 20) as u8; len]).unwrap();
            let c_result = (c.stralloc)(&mut c_arena, string.as_ptr().cast_mut());
            let r_result = (rust.stralloc)(&mut r_arena, string.as_ptr().cast_mut());
            assert_eq!(
                CStr::from_ptr(c_result).to_bytes(),
                CStr::from_ptr(r_result).to_bytes()
            );
            assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));
        }
        assert!(c_arena.block >= 22);
        (c.strreset)(&mut c_arena);
        (rust.strreset)(&mut r_arena);
        assert_eq!(arena_shape(&c_arena), arena_shape(&r_arena));
    }
}

#[test]
fn strkey_and_sh_puts_exports_match() {
    unsafe {
        let (c, rust) = apis();
        for number in [c_int::MIN, -1000, -1, 0, 1, 42, c_int::MAX] {
            let c_value = CStr::from_ptr((c.strkey)(number)).to_bytes().to_vec();
            let r_value = CStr::from_ptr((rust.strkey)(number)).to_bytes().to_vec();
            assert_eq!(c_value, r_value);
            assert_eq!(c_value, format!("test_{number}").as_bytes());
        }

        for number in [-7, 0, 1, 64, 2_000] {
            let c_output = capture_stdout(|| (c.sh_puts)(number));
            let r_output = capture_stdout(|| (rust.sh_puts)(number));
            assert_eq!(c_output, r_output, "sh_puts({number})");
            assert_eq!(c_output, format!("a {number}\n").as_bytes());
        }
    }
}

unsafe fn run_corrupted_assertion_case(api: &Api, scenario: &str) {
    unsafe { (api.rand_seed)(0x5151) };
    if scenario == "slot_bounds" {
        let mut map = ptr::null_mut();
        let mut colliding = Vec::new();
        let mut candidate = 1u64;
        while colliding.len() < 8 {
            let hash = unsafe {
                (api.hash_bytes)(
                    ptr::addr_of_mut!(candidate).cast(),
                    size_of::<u64>(),
                    0x5151,
                )
            };
            let normalized = if hash < 2 { hash + 2 } else { hash };
            if normalized & 15 == 8 {
                colliding.push(candidate);
                map = unsafe { put_bin(api, map, candidate, candidate as i64) };
            }
            candidate += 1;
        }
        let table = unsafe { &mut *table(map, size_of::<BinEntry>()) };
        assert_eq!(table.slot_count, 16);
        table.slot_count = 9;
        let mut key = colliding[7];
        unsafe {
            (api.hmdel)(
                map,
                size_of::<BinEntry>(),
                ptr::addr_of_mut!(key).cast(),
                8,
                0,
                HM_BINARY,
            )
        };
        panic!("corrupted slot count did not trigger an assertion");
    }

    let mut map = ptr::null_mut();
    for key in [11u64, 22, 33] {
        map = unsafe { put_bin(api, map, key, key as i64) };
    }

    match scenario {
        "threshold" => {
            let table = unsafe { &mut *table(map, size_of::<BinEntry>()) };
            table.slot_count = 1;
            table.used_count_threshold = 0;
            table.used_count = 0;
            let mut key = 44u64;
            unsafe {
                (api.hmput)(
                    map,
                    size_of::<BinEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    8,
                    HM_BINARY,
                )
            };
        }
        "moved_missing" | "moved_wrong_index" => {
            let table = unsafe { &mut *table(map, size_of::<BinEntry>()) };
            let mut found = false;
            for bucket_index in 0..table.slot_count / 8 {
                let bucket = unsafe { &mut *table.storage.add(bucket_index) };
                for index in 0..8 {
                    if bucket.index[index] == 2 {
                        if scenario == "moved_missing" {
                            bucket.hash[index] = 1;
                            bucket.index[index] = -2;
                        } else {
                            bucket.index[index] = 0;
                        }
                        found = true;
                    }
                }
            }
            assert!(found);
            let mut key = 11u64;
            unsafe {
                (api.hmdel)(
                    map,
                    size_of::<BinEntry>(),
                    ptr::addr_of_mut!(key).cast(),
                    8,
                    0,
                    HM_BINARY,
                )
            };
        }
        _ => panic!("unknown assertion scenario"),
    }
    panic!("corrupted state did not trigger an assertion");
}

#[test]
fn assertion_child() {
    let Ok(kind) = std::env::var("DIFF_ASSERT_LIBRARY") else {
        return;
    };
    let scenario = std::env::var("DIFF_ASSERT_SCENARIO").unwrap();
    let (c_path, rust_path) = library_paths();
    let path = if kind == "c" { c_path } else { rust_path };
    unsafe {
        let api = Api::load(&path);
        run_corrupted_assertion_case(&api, &scenario);
    }
}

#[test]
fn corrupted_state_assertions_abort_identically() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let executable = std::env::current_exe().unwrap();
    for scenario in [
        "threshold",
        "slot_bounds",
        "moved_missing",
        "moved_wrong_index",
    ] {
        let run = |library: &str| {
            Command::new(&executable)
                .args(["--exact", "assertion_child", "--nocapture"])
                .env("DIFF_ASSERT_LIBRARY", library)
                .env("DIFF_ASSERT_SCENARIO", scenario)
                .output()
                .unwrap()
        };
        let c = run("c");
        let rust = run("rust");
        assert!(!c.status.success(), "C did not reject {scenario}");
        assert!(!rust.status.success(), "Rust did not reject {scenario}");
        assert_eq!(
            c.status.signal(),
            rust.status.signal(),
            "different termination for {scenario}\nC stderr: {}\nRust stderr: {}",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&rust.stderr)
        );
        assert_eq!(c.status.signal(), Some(6), "{scenario} did not SIGABRT");
    }
}

#[test]
fn all_c_invariant_assertions_are_present_in_rust() {
    let c_source = include_str!("../c_src/src/lib.c");
    let rust_source = include_str!("../src/lib.rs");
    assert_eq!(c_source.matches("STBDS_ASSERT(").count(), 10);
    for counterpart in [
        "tombstone_count_threshold)\n            < (*table).slot_count",
        "assert!(i.wrapping_add(1) <= arr_cap(a));",
        "assert!(slot < (*table).slot_count as isize);",
        "assert!((*table).used_count <= usize::MAX);",
        "assert!(slot >= 0);",
        "assert!((*bucket).index[bucket_index] == final_index);",
        "assert!(len <= (*arena).remaining);",
        "assert!(*(*map.cast::<StringMapEntry>()).key == b'a' as c_char);",
        "assert!((*map.cast::<StringMapEntry>()).key != KEY.as_ptr().cast_mut().cast());",
        "assert!((*map.cast::<StringMapEntry>()).value == num);",
    ] {
        assert!(
            rust_source.contains(counterpart),
            "missing Rust counterpart: {counterpart}"
        );
    }
}
