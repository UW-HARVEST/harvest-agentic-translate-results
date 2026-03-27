pub mod blake256;
pub mod blake512;
pub mod hash_blake;
#[cfg(feature = "simple")]
pub mod thash_blake_simple;
#[cfg(feature = "robust")]
pub mod thash_blake_robust;

// C-compatible no_mangle exports for blake256 functions
#[unsafe(no_mangle)]
#[export_name = "blake256_compress"]
pub unsafe extern "C" fn _blake256_compress_export(s: *mut blake256::Blakestate256, block: *const u8) {
    blake256::blake256_compress(&mut *s, block);
}

#[unsafe(no_mangle)]
#[export_name = "blake256_init"]
pub unsafe extern "C" fn _blake256_init_export(s: *mut blake256::Blakestate256) {
    blake256::blake256_init(&mut *s);
}

#[unsafe(no_mangle)]
#[export_name = "blake256_update"]
pub unsafe extern "C" fn _blake256_update_export(s: *mut blake256::Blakestate256, data: *const u8, datalen: u64) {
    blake256::blake256_update(&mut *s, data, datalen);
}

#[unsafe(no_mangle)]
#[export_name = "blake256_final"]
pub unsafe extern "C" fn _blake256_final_export(s: *mut blake256::Blakestate256, digest: *mut u8) {
    blake256::blake256_final(&mut *s, digest);
}

#[unsafe(no_mangle)]
#[export_name = "blake256"]
pub unsafe extern "C" fn _blake256_export(out: *mut u8, in_: *const u8, inlen: u64) -> i32 {
    blake256::blake256(out, in_, inlen)
}

// C-compatible no_mangle exports for blake512 functions
#[unsafe(no_mangle)]
#[export_name = "blake512_compress"]
pub unsafe extern "C" fn _blake512_compress_export(s: *mut blake512::Blakestate512, block: *const u8) {
    blake512::blake512_compress(&mut *s, block);
}

#[unsafe(no_mangle)]
#[export_name = "blake512_init"]
pub unsafe extern "C" fn _blake512_init_export(s: *mut blake512::Blakestate512) {
    blake512::blake512_init(&mut *s);
}

#[unsafe(no_mangle)]
#[export_name = "blake512_update"]
pub unsafe extern "C" fn _blake512_update_export(s: *mut blake512::Blakestate512, data: *const u8, datalen: u64) {
    blake512::blake512_update(&mut *s, data, datalen);
}

#[unsafe(no_mangle)]
#[export_name = "blake512_final"]
pub unsafe extern "C" fn _blake512_final_export(s: *mut blake512::Blakestate512, digest: *mut u8) {
    blake512::blake512_final(&mut *s, digest);
}

#[unsafe(no_mangle)]
#[export_name = "blake512"]
pub unsafe extern "C" fn _blake512_export(out: *mut u8, in_: *const u8, inlen: u64) -> i32 {
    blake512::blake512(out, in_, inlen)
}
