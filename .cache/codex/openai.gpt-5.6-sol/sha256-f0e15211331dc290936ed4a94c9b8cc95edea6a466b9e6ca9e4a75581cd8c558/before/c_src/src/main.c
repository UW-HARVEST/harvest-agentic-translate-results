#include <math.h>
#include <stdint.h>
#include <stdio.h>

#define STB_PERLIN_IMPLEMENTATION
#include "stb_perlin.h"

float inner(
    int which,
    float x,
    float y,
    float z,
    int x_wrap,
    int y_wrap,
    int z_wrap,
    int seed,
    float lacunarity,
    float gain,
    float offset,
    int octaves)
{
    switch (which)
    {
    case 0:
        return stb_perlin_noise3(x, y, z, x_wrap, y_wrap, z_wrap);
    case 1:
        return stb_perlin_noise3_seed(x, y, z, x_wrap, y_wrap, z_wrap, seed);
    case 2:
        return stb_perlin_ridge_noise3(x, y, z, lacunarity, gain, offset, octaves);
    case 3:
        return stb_perlin_fbm_noise3(x, y, z, lacunarity, gain, octaves);
    case 4:
        return stb_perlin_turbulence_noise3(x, y, z, lacunarity, gain, octaves);
    case 5:
        return stb_perlin_noise3_wrap_nonpow2(x, y, z, x_wrap, y_wrap, z_wrap, seed);
    default:
        return NAN;
    }
}

int main()
{
    int which = 0;
    float x = 0.0f;
    float y = 0.0f;
    float z = 0.0f;
    int x_wrap = 0;
    int y_wrap = 0;
    int z_wrap = 0;
    int seed = 0;
    float lacunarity = 0.0f;
    float gain = 0.0f;
    float offset = 0.0f;
    int octaves = 0;
    scanf(
        "%d%f%f%f%d%d%d%d%f%f%f%d",
        &which,
        &x,
        &y,
        &z,
        &x_wrap,
        &y_wrap,
        &z_wrap,
        &seed,
        &lacunarity,
        &gain,
        &offset,
        &octaves);
    float res = inner(
        which,
        x,
        y,
        z,
        x_wrap,
        y_wrap,
        z_wrap,
        seed,
        lacunarity,
        gain,
        offset,
        octaves);
    printf("%.9g\n", res);
    return 0;
}