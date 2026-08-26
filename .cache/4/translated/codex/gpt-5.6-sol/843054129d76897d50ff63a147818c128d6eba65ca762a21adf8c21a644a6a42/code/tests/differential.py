#!/usr/bin/env python3

import ctypes
import math
import random
import struct
import sys
from pathlib import Path


class V(ctypes.Structure):
    _fields_ = [("x", ctypes.c_float), ("y", ctypes.c_float)]


class Raycast(ctypes.Structure):
    _fields_ = [("t", ctypes.c_float), ("n", V)]


class Circle(ctypes.Structure):
    _fields_ = [("p", V), ("r", ctypes.c_float)]


class Aabb(ctypes.Structure):
    _fields_ = [("min", V), ("max", V)]


class Capsule(ctypes.Structure):
    _fields_ = [("a", V), ("b", V), ("r", ctypes.c_float)]


class Ray(ctypes.Structure):
    _fields_ = [("p", V), ("d", V), ("t", ctypes.c_float)]


class M(ctypes.Structure):
    _fields_ = [("x", V), ("y", V)]


F = ctypes.c_float
I = ctypes.c_int
P = ctypes.c_void_p


def configure(lib):
    signatures = {
        "c2V": ([F, F], V),
        "c2Dot": ([V, V], F),
        "c2Len": ([V], F),
        "c2Add": ([V, V], V),
        "c2Sub": ([V, V], V),
        "c2Mulvs": ([V, F], V),
        "c2Div": ([V, F], V),
        "c2Norm": ([V], V),
        "c2Minv": ([V, V], V),
        "c2Maxv": ([V, V], V),
        "c2Skew": ([V], V),
        "c2Absv": ([V], V),
        "c2RaytoCircle": ([Ray, Circle, ctypes.POINTER(Raycast)], I),
        "c2AABBtoAABB": ([Aabb, Aabb], I),
        "c2RaytoAABB": ([Ray, Aabb, ctypes.POINTER(Raycast)], I),
        "c2CCW90": ([V], V),
        "c2MulmvT": ([M, V], V),
        "c2AABBtoPoint": ([Aabb, V], I),
        "c2CircleToPoint": ([Circle, V], I),
        "c2RaytoCapsule": ([Ray, Capsule, ctypes.POINTER(Raycast)], I),
        "c2CastRay": ([Ray, P, I, ctypes.POINTER(Raycast)], I),
        "spec_ray": ([ctypes.POINTER(Raycast)] + [F] * 7, I),
    }
    for name, (args, result) in signatures.items():
        function = getattr(lib, name)
        function.argtypes = args
        function.restype = result


def raw(value):
    return ctypes.string_at(ctypes.byref(value), ctypes.sizeof(value))


def float_raw(value):
    return struct.pack("=f", value)


def check(name, left, right, case):
    left_raw = raw(left) if isinstance(left, ctypes.Structure) else float_raw(left)
    right_raw = raw(right) if isinstance(right, ctypes.Structure) else float_raw(right)
    if left_raw != right_raw:
        raise AssertionError(
            f"{name} mismatch at case {case}: {left_raw.hex()} != {right_raw.hex()}"
        )


def check_int(name, left, right, case):
    if left != right:
        raise AssertionError(f"{name} mismatch at case {case}: {left} != {right}")


def f32(value):
    return F(value).value


def finite_from_bits(rng):
    while True:
        bits = rng.getrandbits(32)
        if bits & 0x7F800000 != 0x7F800000:
            return struct.unpack("=f", struct.pack("=I", bits))[0]


def from_bits(bits):
    return struct.unpack("=f", struct.pack("=I", bits))[0]


def ieee_values():
    return [
        from_bits(bits)
        for bits in (
            0x00000000,
            0x80000000,
            0x00000001,
            0x80000001,
            0x007FFFFF,
            0x00800000,
            0x3F800000,
            0xBF800000,
            0x7F7FFFFF,
            0xFF7FFFFF,
            0x7F800000,
            0xFF800000,
            0x7FC00000,
            0xFFC00000,
        )
    ]


def bounded(rng, low=-100.0, high=100.0):
    return f32(rng.uniform(low, high))


def vector(rng):
    return V(bounded(rng), bounded(rng))


def box(rng):
    x0, x1 = sorted((bounded(rng), bounded(rng)))
    y0, y1 = sorted((bounded(rng), bounded(rng)))
    return Aabb(V(x0, y0), V(x1, y1))


def ray(rng):
    return Ray(vector(rng), vector(rng), bounded(rng, 0.0, 200.0))


def circle(rng):
    return Circle(vector(rng), bounded(rng, 0.0, 40.0))


def capsule(rng):
    a = vector(rng)
    b = vector(rng)
    if a.x == b.x and a.y == b.y:
        b.x = f32(b.x + 1.0)
    return Capsule(a, b, bounded(rng, 0.0, 40.0))


def sentinel():
    return Raycast(f32(123.25), V(f32(-456.5), f32(789.75)))


def test_primitives(c_lib, r_lib, rng, count):
    binary_vector = ["c2Add", "c2Sub", "c2Minv", "c2Maxv"]
    unary_vector = ["c2Norm", "c2Skew", "c2Absv", "c2CCW90"]

    def test_case(case, a, b, scalar):
        check("c2V", c_lib.c2V(a.x, a.y), r_lib.c2V(a.x, a.y), case)
        check("c2Dot", c_lib.c2Dot(a, b), r_lib.c2Dot(a, b), case)
        check("c2Len", c_lib.c2Len(a), r_lib.c2Len(a), case)
        for name in binary_vector:
            check(name, getattr(c_lib, name)(a, b), getattr(r_lib, name)(a, b), case)
        for name in unary_vector:
            check(name, getattr(c_lib, name)(a), getattr(r_lib, name)(a), case)
        check("c2Mulvs", c_lib.c2Mulvs(a, scalar), r_lib.c2Mulvs(a, scalar), case)
        check("c2Div", c_lib.c2Div(a, scalar), r_lib.c2Div(a, scalar), case)

        matrix = M(a, b)
        check(
            "c2MulmvT",
            c_lib.c2MulmvT(matrix, V(scalar, a.x)),
            r_lib.c2MulmvT(matrix, V(scalar, a.x)),
            case,
        )

    edge_values = ieee_values()
    edge_case = 0
    for index, left in enumerate(edge_values):
        for right in edge_values:
            scalar = edge_values[(index + edge_case) % len(edge_values)]
            test_case(("edge", edge_case), V(left, right), V(right, left), scalar)
            edge_case += 1

    for case in range(count):
        a = V(finite_from_bits(rng), finite_from_bits(rng))
        b = V(finite_from_bits(rng), finite_from_bits(rng))
        scalar = finite_from_bits(rng)
        test_case(("random", case), a, b, scalar)


def test_geometry(c_lib, r_lib, rng, count):
    for case in range(count):
        a_box = box(rng)
        b_box = box(rng)
        point = vector(rng)
        c = circle(rng)
        cap = capsule(rng)
        test_ray = ray(rng)

        check_int(
            "c2AABBtoAABB",
            c_lib.c2AABBtoAABB(a_box, b_box),
            r_lib.c2AABBtoAABB(a_box, b_box),
            case,
        )
        check_int(
            "c2AABBtoPoint",
            c_lib.c2AABBtoPoint(a_box, point),
            r_lib.c2AABBtoPoint(a_box, point),
            case,
        )
        check_int(
            "c2CircleToPoint",
            c_lib.c2CircleToPoint(c, point),
            r_lib.c2CircleToPoint(c, point),
            case,
        )

        for name, shape in (
            ("c2RaytoCircle", c),
            ("c2RaytoAABB", a_box),
            ("c2RaytoCapsule", cap),
        ):
            c_out = sentinel()
            r_out = sentinel()
            c_status = getattr(c_lib, name)(test_ray, shape, ctypes.byref(c_out))
            r_status = getattr(r_lib, name)(test_ray, shape, ctypes.byref(r_out))
            check_int(name, c_status, r_status, case)
            check(name, c_out, r_out, case)

        for shape_type, shape in enumerate((c, a_box, cap)):
            c_out = sentinel()
            r_out = sentinel()
            c_status = c_lib.c2CastRay(
                test_ray, ctypes.byref(shape), shape_type, ctypes.byref(c_out)
            )
            r_status = r_lib.c2CastRay(
                test_ray, ctypes.byref(shape), shape_type, ctypes.byref(r_out)
            )
            check_int("c2CastRay", c_status, r_status, (case, shape_type))
            check("c2CastRay", c_out, r_out, (case, shape_type))

        values = [bounded(rng) for _ in range(7)]
        values[4] = bounded(rng, 0.0, 40.0)
        c_out = sentinel()
        r_out = sentinel()
        c_status = c_lib.spec_ray(ctypes.byref(c_out), *values)
        r_status = r_lib.spec_ray(ctypes.byref(r_out), *values)
        check_int("spec_ray", c_status, r_status, case)
        check("spec_ray", c_out, r_out, case)


def test_geometry_edges(c_lib, r_lib, count):
    values = ieee_values()

    def value(case, offset):
        return values[(case * 5 + offset) % len(values)]

    for case in range(count):
        test_ray = Ray(
            V(value(case, 0), value(case, 1)),
            V(value(case, 2), value(case, 3)),
            value(case, 4),
        )
        a_box = Aabb(
            V(value(case, 5), value(case, 6)),
            V(value(case, 7), value(case, 8)),
        )
        c = Circle(V(value(case, 9), value(case, 10)), value(case, 11))
        cap = Capsule(
            V(value(case, 12), value(case, 13)),
            V(value(case, 14), value(case, 15)),
            value(case, 16),
        )

        for name, shape in (
            ("c2RaytoCircle", c),
            ("c2RaytoAABB", a_box),
            ("c2RaytoCapsule", cap),
        ):
            c_out = sentinel()
            r_out = sentinel()
            c_status = getattr(c_lib, name)(test_ray, shape, ctypes.byref(c_out))
            r_status = getattr(r_lib, name)(test_ray, shape, ctypes.byref(r_out))
            check_int(name, c_status, r_status, ("edge", case))
            check(name, c_out, r_out, ("edge", case))

        for shape_type, shape in enumerate((c, a_box, cap)):
            c_out = sentinel()
            r_out = sentinel()
            c_status = c_lib.c2CastRay(
                test_ray, ctypes.byref(shape), shape_type, ctypes.byref(c_out)
            )
            r_status = r_lib.c2CastRay(
                test_ray, ctypes.byref(shape), shape_type, ctypes.byref(r_out)
            )
            check_int("c2CastRay", c_status, r_status, ("edge", case, shape_type))
            check("c2CastRay", c_out, r_out, ("edge", case, shape_type))

        spec_values = [value(case, offset) for offset in range(17, 24)]
        c_out = sentinel()
        r_out = sentinel()
        c_status = c_lib.spec_ray(ctypes.byref(c_out), *spec_values)
        r_status = r_lib.spec_ray(ctypes.byref(r_out), *spec_values)
        check_int("spec_ray", c_status, r_status, ("edge", case))
        check("spec_ray", c_out, r_out, ("edge", case))


def main():
    root = Path(__file__).resolve().parents[1]
    c_path = root / ".c-reference-build" / "libtranslated_rust.so"
    rust_path = root / "target" / "release" / "libtranslated_rust.so"
    if not c_path.is_file() or not rust_path.is_file():
        raise SystemExit("build both reference and Rust shared libraries first")

    c_lib = ctypes.CDLL(str(c_path))
    r_lib = ctypes.CDLL(str(rust_path))
    configure(c_lib)
    configure(r_lib)
    rng = random.Random(0xC2CA57)

    test_primitives(c_lib, r_lib, rng, 20_000)
    test_geometry(c_lib, r_lib, rng, 20_000)
    test_geometry_edges(c_lib, r_lib, 2_000)
    print("PASS: all 22 exports matched across 42,196 differential cases")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
