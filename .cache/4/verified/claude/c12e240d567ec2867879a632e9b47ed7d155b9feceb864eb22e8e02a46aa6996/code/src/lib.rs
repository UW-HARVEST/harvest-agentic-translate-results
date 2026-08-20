//! Rust translation of `c_src/src/main.c` + `c_src/src/stb_perlin.h`.
//!
//! Building this crate as a `cdylib` produces a shared object that exports the
//! exact same C symbols as the shared object built from `c_src/src/main.c`, so
//! an external caller (see `tests/`) can dlopen both and compare results.

pub mod cfmt;
pub mod cscan;
pub mod driver;
pub mod stb_perlin;
pub mod tables;

// ---------------------------------------------------------------------------
// C ABI exports.  One `#[no_mangle] extern "C"` wrapper per non-static
// function of the C translation unit (see `nm -D` on the C shared object).
// ---------------------------------------------------------------------------

use std::os::raw::{c_float, c_int, c_uchar};

/// `float stb_perlin_noise3_internal(float x, float y, float z, int x_wrap,
///     int y_wrap, int z_wrap, unsigned char seed)`
#[no_mangle]
pub extern "C" fn stb_perlin_noise3_internal(
    x: c_float,
    y: c_float,
    z: c_float,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: c_uchar,
) -> c_float {
    stb_perlin::stb_perlin_noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

/// `float stb_perlin_noise3(float x, float y, float z, int x_wrap, int y_wrap, int z_wrap)`
#[no_mangle]
pub extern "C" fn stb_perlin_noise3(
    x: c_float,
    y: c_float,
    z: c_float,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
) -> c_float {
    stb_perlin::stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap)
}

/// `float stb_perlin_noise3_seed(float x, float y, float z, int x_wrap,
///     int y_wrap, int z_wrap, int seed)`
#[no_mangle]
pub extern "C" fn stb_perlin_noise3_seed(
    x: c_float,
    y: c_float,
    z: c_float,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: c_int,
) -> c_float {
    stb_perlin::stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

/// `float stb_perlin_ridge_noise3(float x, float y, float z, float lacunarity,
///     float gain, float offset, int octaves)`
#[no_mangle]
pub extern "C" fn stb_perlin_ridge_noise3(
    x: c_float,
    y: c_float,
    z: c_float,
    lacunarity: c_float,
    gain: c_float,
    offset: c_float,
    octaves: c_int,
) -> c_float {
    stb_perlin::stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves)
}

/// `float stb_perlin_fbm_noise3(float x, float y, float z, float lacunarity,
///     float gain, int octaves)`
#[no_mangle]
pub extern "C" fn stb_perlin_fbm_noise3(
    x: c_float,
    y: c_float,
    z: c_float,
    lacunarity: c_float,
    gain: c_float,
    octaves: c_int,
) -> c_float {
    stb_perlin::stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves)
}

/// `float stb_perlin_turbulence_noise3(float x, float y, float z,
///     float lacunarity, float gain, int octaves)`
#[no_mangle]
pub extern "C" fn stb_perlin_turbulence_noise3(
    x: c_float,
    y: c_float,
    z: c_float,
    lacunarity: c_float,
    gain: c_float,
    octaves: c_int,
) -> c_float {
    stb_perlin::stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves)
}

/// `float stb_perlin_noise3_wrap_nonpow2(float x, float y, float z, int x_wrap,
///     int y_wrap, int z_wrap, unsigned char seed)`
#[no_mangle]
pub extern "C" fn stb_perlin_noise3_wrap_nonpow2(
    x: c_float,
    y: c_float,
    z: c_float,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: c_uchar,
) -> c_float {
    stb_perlin::stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

/// `float inner(int which, float x, float y, float z, int x_wrap, int y_wrap,
///     int z_wrap, int seed, float lacunarity, float gain, float offset,
///     int octaves)`
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn inner(
    which: c_int,
    x: c_float,
    y: c_float,
    z: c_float,
    x_wrap: c_int,
    y_wrap: c_int,
    z_wrap: c_int,
    seed: c_int,
    lacunarity: c_float,
    gain: c_float,
    offset: c_float,
    octaves: c_int,
) -> c_float {
    driver::inner(
        which, x, y, z, x_wrap, y_wrap, z_wrap, seed, lacunarity, gain, offset, octaves,
    )
}

/// `int main()` -- reads the twelve values from stdin and prints `%.9g`.
///
/// The C translation unit is compiled into the shared object as-is, so `main`
/// is one of its exported symbols; it is mirrored here for symbol parity.
///
/// `#[cfg(not(test))]`: when the library target is compiled as a *test*
/// harness, that harness generates its own entry point and the linker would
/// reject the duplicate `main`.  The `cdylib` (what the tests dlopen) is always
/// built without `test`, so it does export the symbol.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    driver::c_main()
}
