mod stb_perlin;

#[no_mangle]
pub extern "C" fn stb_perlin_noise3(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32) -> f32 {
    stb_perlin::stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap)
}

#[no_mangle]
pub extern "C" fn stb_perlin_noise3_seed(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32, seed: i32) -> f32 {
    stb_perlin::stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

#[no_mangle]
pub extern "C" fn stb_perlin_noise3_internal(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32, seed: u8) -> f32 {
    stb_perlin::noise3_internal(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}

#[no_mangle]
pub extern "C" fn stb_perlin_ridge_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, offset: f32, octaves: i32) -> f32 {
    stb_perlin::stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves)
}

#[no_mangle]
pub extern "C" fn stb_perlin_fbm_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    stb_perlin::stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves)
}

#[no_mangle]
pub extern "C" fn stb_perlin_turbulence_noise3(x: f32, y: f32, z: f32, lacunarity: f32, gain: f32, octaves: i32) -> f32 {
    stb_perlin::stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves)
}

#[no_mangle]
pub extern "C" fn stb_perlin_noise3_wrap_nonpow2(x: f32, y: f32, z: f32, x_wrap: i32, y_wrap: i32, z_wrap: i32, seed: u8) -> f32 {
    stb_perlin::stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed)
}
