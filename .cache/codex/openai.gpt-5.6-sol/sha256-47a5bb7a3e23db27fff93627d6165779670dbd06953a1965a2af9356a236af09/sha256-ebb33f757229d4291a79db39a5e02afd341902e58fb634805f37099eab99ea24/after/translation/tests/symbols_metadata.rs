mod common;

use common::{Libraries, manifest_path};
use std::ffi::{CStr, c_char, c_int, c_uint};
use std::fs;

#[test]
fn every_c_symbol_resolves_from_both_shared_libraries() {
    unsafe {
        let libs = Libraries::load();
        let symbols = fs::read_to_string(manifest_path("SYMBOLS.md")).unwrap();
        let mut count = 0;
        for line in symbols
            .lines()
            .filter(|line| line.starts_with("| ") && line.contains("`"))
        {
            let Some(start) = line.find('`') else {
                continue;
            };
            let Some(end) = line[start + 1..].find('`') else {
                continue;
            };
            let name = &line[start + 1..start + 1 + end];
            let mut nul_name = name.as_bytes().to_vec();
            nul_name.push(0);
            let _: libloading::Symbol<'_, unsafe extern "C" fn()> = libs
                .c
                .get(&nul_name)
                .unwrap_or_else(|e| panic!("C {name}: {e}"));
            let _: libloading::Symbol<'_, unsafe extern "C" fn()> = libs
                .rust
                .get(&nul_name)
                .unwrap_or_else(|e| panic!("Rust {name}: {e}"));
            count += 1;
        }
        assert_eq!(count, 143);
    }
}

#[test]
fn scalar_metadata_and_bounds_match() {
    unsafe {
        let libs = Libraries::load();

        macro_rules! compare0 {
            ($name:literal, $ty:ty) => {{
                let (c, r) =
                    libs.pair::<unsafe extern "C" fn() -> $ty>(concat!($name, "\0").as_bytes());
                assert_eq!(c(), r(), $name);
            }};
        }
        compare0!("LZ4_versionNumber", c_int);
        compare0!("LZ4_sizeofState", c_int);
        compare0!("LZ4_sizeofStateHC", c_int);
        compare0!("LZ4_sizeofStreamState", c_int);
        compare0!("LZ4_sizeofStreamStateHC", c_int);
        compare0!("LZ4F_getVersion", c_uint);
        compare0!("LZ4F_compressionLevel_max", c_int);
        compare0!("LZ4_XXH_versionNumber", c_uint);

        let (c_version, r_version) =
            libs.pair::<unsafe extern "C" fn() -> *const c_char>(b"LZ4_versionString\0");
        assert_eq!(
            CStr::from_ptr(c_version()).to_bytes(),
            CStr::from_ptr(r_version()).to_bytes()
        );

        let (c_bound, r_bound) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_compressBound\0");
        for value in [-1, 0, 1, 12, 13, 255, 256, 65_535, 0x7e00_0000, 0x7e00_0001] {
            assert_eq!(c_bound(value), r_bound(value), "LZ4_compressBound({value})");
        }

        let (c_ring, r_ring) =
            libs.pair::<unsafe extern "C" fn(c_int) -> c_int>(b"LZ4_decoderRingBufferSize\0");
        for value in [-1, 0, 1, 16, 17, 64 * 1024, 0x7e00_0000, 0x7e00_0001] {
            assert_eq!(c_ring(value), r_ring(value), "ring size {value}");
        }

        let (c_block, r_block) =
            libs.pair::<unsafe extern "C" fn(c_uint) -> usize>(b"LZ4F_getBlockSize\0");
        for value in [0, 1, 3, 4, 5, 6, 7, 8, u32::MAX] {
            assert_eq!(c_block(value), r_block(value), "block enum {value}");
        }

        let (c_is_error, r_is_error) =
            libs.pair::<unsafe extern "C" fn(usize) -> c_uint>(b"LZ4F_isError\0");
        let (c_error_name, r_error_name) =
            libs.pair::<unsafe extern "C" fn(usize) -> *const c_char>(b"LZ4F_getErrorName\0");
        let (c_error_code, r_error_code) =
            libs.pair::<unsafe extern "C" fn(usize) -> c_int>(b"LZ4F_getErrorCode\0");
        for value in [0, 1, 19, usize::MAX, usize::MAX - 1, usize::MAX - 24] {
            assert_eq!(c_is_error(value), r_is_error(value));
            assert_eq!(c_error_code(value), r_error_code(value));
            assert_eq!(
                CStr::from_ptr(c_error_name(value)).to_bytes(),
                CStr::from_ptr(r_error_name(value)).to_bytes()
            );
        }
    }
}
