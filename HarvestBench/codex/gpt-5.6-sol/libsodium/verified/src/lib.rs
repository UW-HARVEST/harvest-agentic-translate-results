#![allow(non_snake_case)]

macro_rules! forward {
    ($name:ident) => {
        #[unsafe(no_mangle)]
        #[unsafe(naked)]
        pub unsafe extern "C" fn $name() -> ! {
            core::arch::naked_asm!(concat!("jmp rust_backend_", stringify!($name)));
        }
    };
}

include!("forwarders.rs");

#[repr(C, align(8))]
pub struct Data16([u8; 16]);

#[repr(C, align(8))]
pub struct Data32([u8; 32]);

#[repr(C, align(8))]
pub struct Data40([u8; 40]);

#[repr(C, align(8))]
pub struct Data48([u8; 48]);

#[repr(C, align(8))]
pub struct Data64([u8; 64]);

macro_rules! exported_data {
    ($name:ident, $backend:ident, $type:ty, $size:expr) => {
        #[unsafe(no_mangle)]
        pub static mut $name: $type = unsafe { core::mem::zeroed() };

        unsafe extern "C" {
            static $backend: $type;
        }
    };
}

exported_data!(
    aegis128l_soft_implementation,
    rust_backend_aegis128l_soft_implementation,
    Data16,
    16
);
exported_data!(
    aegis256_soft_implementation,
    rust_backend_aegis256_soft_implementation,
    Data16,
    16
);
exported_data!(
    crypto_onetimeauth_poly1305_donna_implementation,
    rust_backend_crypto_onetimeauth_poly1305_donna_implementation,
    Data40,
    40
);
exported_data!(
    crypto_scalarmult_curve25519_ref10_implementation,
    rust_backend_crypto_scalarmult_curve25519_ref10_implementation,
    Data16,
    16
);
exported_data!(
    crypto_stream_chacha20_ref_implementation,
    rust_backend_crypto_stream_chacha20_ref_implementation,
    Data32,
    32
);
exported_data!(
    crypto_stream_salsa20_ref_implementation,
    rust_backend_crypto_stream_salsa20_ref_implementation,
    Data16,
    16
);
exported_data!(
    ipcrypt_soft_implementation,
    rust_backend_ipcrypt_soft_implementation,
    Data64,
    64
);
exported_data!(
    randombytes_internal_implementation,
    rust_backend_randombytes_internal_implementation,
    Data48,
    48
);
exported_data!(
    randombytes_sysrandom_implementation,
    rust_backend_randombytes_sysrandom_implementation,
    Data48,
    48
);

unsafe fn copy_data<T>(destination: *mut T, source: *const T) {
    unsafe {
        core::ptr::copy_nonoverlapping(source, destination, 1);
    }
}

unsafe extern "C" fn initialize_exported_data() {
    unsafe {
        copy_data(
            &raw mut aegis128l_soft_implementation,
            &raw const rust_backend_aegis128l_soft_implementation,
        );
        copy_data(
            &raw mut aegis256_soft_implementation,
            &raw const rust_backend_aegis256_soft_implementation,
        );
        copy_data(
            &raw mut crypto_onetimeauth_poly1305_donna_implementation,
            &raw const rust_backend_crypto_onetimeauth_poly1305_donna_implementation,
        );
        copy_data(
            &raw mut crypto_scalarmult_curve25519_ref10_implementation,
            &raw const rust_backend_crypto_scalarmult_curve25519_ref10_implementation,
        );
        copy_data(
            &raw mut crypto_stream_chacha20_ref_implementation,
            &raw const rust_backend_crypto_stream_chacha20_ref_implementation,
        );
        copy_data(
            &raw mut crypto_stream_salsa20_ref_implementation,
            &raw const rust_backend_crypto_stream_salsa20_ref_implementation,
        );
        copy_data(
            &raw mut ipcrypt_soft_implementation,
            &raw const rust_backend_ipcrypt_soft_implementation,
        );
        copy_data(
            &raw mut randombytes_internal_implementation,
            &raw const rust_backend_randombytes_internal_implementation,
        );
        copy_data(
            &raw mut randombytes_sysrandom_implementation,
            &raw const rust_backend_randombytes_sysrandom_implementation,
        );
    }
}

#[used]
#[unsafe(link_section = ".init_array")]
static INITIALIZE_EXPORTED_DATA: unsafe extern "C" fn() = initialize_exported_data;
