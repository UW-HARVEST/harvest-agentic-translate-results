#!/usr/bin/env python3

import ctypes
import random
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
C_BUILD_DIR = Path(Path("/tmp/harvest-c-build-dir").read_text().strip())
C_SO = C_BUILD_DIR / "libtranslated_rust.so"
RUST_SO = ROOT / "target/release/libtranslated_rust.so"


class Pixel(ctypes.Structure):
    _fields_ = [
        ("r", ctypes.c_uint8),
        ("g", ctypes.c_uint8),
        ("b", ctypes.c_uint8),
        ("a", ctypes.c_uint8),
    ]


def load(path):
    library = ctypes.CDLL(str(path))
    library.convert_pix.argtypes = [
        ctypes.c_int,
        ctypes.c_int,
        ctypes.c_int,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.POINTER(Pixel),
    ]
    library.convert_pix.restype = None
    library.cp_inflate.argtypes = [
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    library.cp_inflate.restype = ctypes.c_int
    return library


C = load(C_SO)
RUST = load(RUST_SO)


def bytes_in_dll(library, name, length):
    value = (ctypes.c_uint8 * length).in_dll(library, name)
    return bytes(value)


def check_globals():
    sizes = {
        "cp_fixed_table": 320,
        "cp_permutation_order": 19,
        "cp_len_extra_bits": 31,
        "cp_len_base": 124,
        "cp_dist_extra_bits": 32,
        "cp_dist_base": 128,
    }
    for name, size in sizes.items():
        assert bytes_in_dll(C, name, size) == bytes_in_dll(RUST, name, size), name


def check_pixels():
    rng = random.Random(0xC0FFEE)
    for bpp in [1, 2, 3, 4, 5]:
        for width in [0, 1, 2, 9]:
            for height in [0, 1, 3]:
                source_size = max(1, height * (1 + width * bpp))
                source_bytes = bytes(rng.randrange(256) for _ in range(source_size))
                c_source = (ctypes.c_uint8 * source_size).from_buffer_copy(source_bytes)
                rust_source = (ctypes.c_uint8 * source_size).from_buffer_copy(source_bytes)
                pixel_count = max(1, width * height)
                c_output = (Pixel * pixel_count)()
                rust_output = (Pixel * pixel_count)()
                ctypes.memset(c_output, 0xA5, ctypes.sizeof(c_output))
                ctypes.memset(rust_output, 0xA5, ctypes.sizeof(rust_output))
                C.convert_pix(bpp, width, height, c_source, c_output)
                RUST.convert_pix(bpp, width, height, rust_source, rust_output)
                assert bytes(c_output) == bytes(rust_output), (bpp, width, height)


def aligned_input(data, alignment):
    storage = ctypes.create_string_buffer(len(data) + 8)
    base = ctypes.addressof(storage)
    offset = next(i for i in range(8) if (base + i) % 4 == alignment)
    ctypes.memmove(base + offset, data, len(data))
    return storage, ctypes.c_void_p(base + offset)


def error_reason(library):
    address = ctypes.c_void_p.in_dll(library, "cp_error_reason").value
    return ctypes.string_at(address) if address else None


def inflate_once(library, compressed, output_size, alignment):
    storage, input_pointer = aligned_input(compressed, alignment)
    output = ctypes.create_string_buffer(max(1, output_size))
    ctypes.memset(output, 0xA5, len(output))
    result = library.cp_inflate(input_pointer, len(compressed), output, output_size)
    return result, bytes(output), error_reason(library), storage


def raw_deflate(data, level, strategy=zlib.Z_DEFAULT_STRATEGY):
    compressor = zlib.compressobj(level, zlib.DEFLATED, -15, 8, strategy)
    return compressor.compress(data) + compressor.flush()


def check_inflate():
    rng = random.Random(0x5EED)
    data_cases = [
        b"",
        b"a",
        b"hello world",
        b"A" * 4096,
        bytes(range(256)) * 8,
        bytes(rng.randrange(256) for _ in range(8192)),
        (b"abcdef0123456789" * 5000)[:70000],
    ]
    streams = []
    for data in data_cases:
        for level in [0, 1, 6, 9]:
            streams.append((data, raw_deflate(data, level)))
        streams.append((data, raw_deflate(data, 6, zlib.Z_FIXED)))

    for data, compressed in streams:
        for alignment in range(4):
            c_result, c_output, c_error, _ = inflate_once(
                C, compressed, len(data) + 16, alignment
            )
            rust_result, rust_output, rust_error, _ = inflate_once(
                RUST, compressed, len(data) + 16, alignment
            )
            assert (c_result, c_output, c_error) == (
                rust_result,
                rust_output,
                rust_error,
            ), (len(data), len(compressed), alignment, c_result, rust_result)
            if data and compressed[0] & 0x06:
                small_size = max(0, len(data) // 2)
                c_small = inflate_once(C, compressed, small_size, alignment)[:3]
                rust_small = inflate_once(RUST, compressed, small_size, alignment)[:3]
                assert c_small == rust_small, (
                    "small",
                    len(data),
                    len(compressed),
                    alignment,
                )

    invalid = b"\x07"
    for alignment in range(4):
        c_value = inflate_once(C, invalid, 16, alignment)[:3]
        rust_value = inflate_once(RUST, invalid, 16, alignment)[:3]
        assert c_value == rust_value


def main():
    check_globals()
    check_pixels()
    check_inflate()
    print("differential checks passed")


if __name__ == "__main__":
    main()
