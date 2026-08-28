#!/usr/bin/env python3
import ctypes as ct
import math
import random
import struct
import sys


class V(ct.Structure):
    _fields_ = [("x", ct.c_float), ("y", ct.c_float)]


class R(ct.Structure):
    _fields_ = [("c", ct.c_float), ("s", ct.c_float)]


class X(ct.Structure):
    _fields_ = [("p", V), ("r", R)]


class Circle(ct.Structure):
    _fields_ = [("p", V), ("r", ct.c_float)]


class Aabb(ct.Structure):
    _fields_ = [("min", V), ("max", V)]


class Capsule(ct.Structure):
    _fields_ = [("a", V), ("b", V), ("r", ct.c_float)]


class Cache(ct.Structure):
    _fields_ = [
        ("metric", ct.c_float),
        ("count", ct.c_int),
        ("i_a", ct.c_int * 3),
        ("i_b", ct.c_int * 3),
        ("div", ct.c_float),
    ]


class Proxy(ct.Structure):
    _fields_ = [("radius", ct.c_float), ("count", ct.c_int), ("verts", V * 8)]


class Sv(ct.Structure):
    _fields_ = [
        ("s_a", V),
        ("s_b", V),
        ("p", V),
        ("u", ct.c_float),
        ("i_a", ct.c_int),
        ("i_b", ct.c_int),
    ]


class Simplex(ct.Structure):
    _fields_ = [
        ("a", Sv),
        ("b", Sv),
        ("c", Sv),
        ("d", Sv),
        ("div", ct.c_float),
        ("count", ct.c_int),
    ]


EXPECTED_SIZES = {
    V: 8,
    R: 8,
    X: 16,
    Circle: 12,
    Aabb: 16,
    Capsule: 20,
    Cache: 36,
    Proxy: 72,
    Sv: 36,
    Simplex: 152,
}


def raw(value):
    return ct.string_at(ct.addressof(value), ct.sizeof(value))


def clone(value):
    return type(value).from_buffer_copy(raw(value))


def f32_bytes(value):
    return struct.pack("=f", value)


def same(label, left, right):
    left_raw = f32_bytes(left) if isinstance(left, float) else raw(left)
    right_raw = f32_bytes(right) if isinstance(right, float) else raw(right)
    if left_raw != right_raw:
        raise AssertionError(
            f"{label}: {left_raw.hex()} != {right_raw.hex()}"
        )


def bind(lib, name, args, result):
    fn = getattr(lib, name)
    fn.argtypes = args
    fn.restype = result
    return fn


def rv(rng, scale=100.0):
    return V(rng.uniform(-scale, scale), rng.uniform(-scale, scale))


def rsv(rng):
    return Sv(rv(rng), rv(rng), rv(rng), rng.uniform(-20, 20), rng.randrange(4), rng.randrange(4))


def rsimplex(rng, count):
    return Simplex(rsv(rng), rsv(rng), rsv(rng), rsv(rng), rng.uniform(0.1, 20), count)


def bind_all(lib):
    specs = {
        "c2V": ([ct.c_float, ct.c_float], V),
        "c2Mulvs": ([V, ct.c_float], V),
        "c2Maxv": ([V, V], V),
        "c2Minv": ([V, V], V),
        "c2Clampv": ([V, V, V], V),
        "c2Sub": ([V, V], V),
        "c2Dot": ([V, V], ct.c_float),
        "c2RotIdentity": ([], R),
        "c2xIdentity": ([], X),
        "c2BBVerts": ([ct.POINTER(V), ct.POINTER(Aabb)], None),
        "c2MakeProxy": ([ct.c_void_p, ct.c_int, ct.POINTER(Proxy)], None),
        "c2Len": ([V], ct.c_float),
        "c2Det2": ([V, V], ct.c_float),
        "c2GJKSimplexMetric": ([ct.POINTER(Simplex)], ct.c_float),
        "c2Mulrv": ([R, V], V),
        "c2Add": ([V, V], V),
        "c2Mulxv": ([X, V], V),
        "c22": ([ct.POINTER(Simplex)], None),
        "c23": ([ct.POINTER(Simplex)], None),
        "c2Neg": ([V], V),
        "c2Skew": ([V], V),
        "c2CCW90": ([V], V),
        "c2D": ([ct.POINTER(Simplex)], V),
        "c2Support": ([ct.POINTER(V), ct.c_int, V], ct.c_int),
        "c2Witness": ([ct.POINTER(Simplex), ct.POINTER(V), ct.POINTER(V)], None),
        "c2Div": ([V, ct.c_float], V),
        "c2Norm": ([V], V),
        "c2L": ([ct.POINTER(Simplex)], V),
        "c2MulrvT": ([R, V], V),
        "c2GJK": (
            [
                ct.c_void_p,
                ct.c_int,
                ct.POINTER(X),
                ct.c_void_p,
                ct.c_int,
                ct.POINTER(X),
                ct.POINTER(V),
                ct.POINTER(V),
                ct.c_int,
                ct.POINTER(ct.c_int),
                ct.POINTER(Cache),
            ],
            ct.c_float,
        ),
        "gjk": (
            [
                ct.c_char,
                ct.POINTER(V),
                ct.POINTER(V),
                ct.c_float,
                ct.c_float,
                ct.c_float,
                ct.c_float,
                ct.c_float,
                ct.c_float,
                ct.c_float,
                ct.c_float,
                ct.c_float,
            ],
            None,
        ),
    }
    return {name: bind(lib, name, *spec) for name, spec in specs.items()}


def check_value_functions(c, rust, rng):
    unary_v = ["c2Len", "c2Neg", "c2Skew", "c2CCW90", "c2Norm"]
    binary_v = ["c2Maxv", "c2Minv", "c2Sub", "c2Dot", "c2Det2", "c2Add"]
    rotation_v = ["c2Mulrv", "c2MulrvT"]

    same("c2RotIdentity", c["c2RotIdentity"](), rust["c2RotIdentity"]())
    same("c2xIdentity", c["c2xIdentity"](), rust["c2xIdentity"]())
    for i in range(5000):
        a = rv(rng)
        b = rv(rng)
        scalar = rng.uniform(0.01, 20.0)
        same(f"c2V[{i}]", c["c2V"](a.x, a.y), rust["c2V"](a.x, a.y))
        same(f"c2Mulvs[{i}]", c["c2Mulvs"](a, scalar), rust["c2Mulvs"](a, scalar))
        same(f"c2Div[{i}]", c["c2Div"](a, scalar), rust["c2Div"](a, scalar))
        lo = V(min(a.x, b.x), min(a.y, b.y))
        hi = V(max(a.x, b.x), max(a.y, b.y))
        value = rv(rng)
        same(f"c2Clampv[{i}]", c["c2Clampv"](value, lo, hi), rust["c2Clampv"](value, lo, hi))
        for name in unary_v:
            same(f"{name}[{i}]", c[name](a), rust[name](a))
        for name in binary_v:
            same(f"{name}[{i}]", c[name](a, b), rust[name](a, b))
        angle = rng.uniform(-math.pi, math.pi)
        rotation = R(math.cos(angle), math.sin(angle))
        for name in rotation_v:
            same(f"{name}[{i}]", c[name](rotation, a), rust[name](rotation, a))
        transform = X(rv(rng), rotation)
        same(f"c2Mulxv[{i}]", c["c2Mulxv"](transform, a), rust["c2Mulxv"](transform, a))


def check_pointer_helpers(c, rust, rng):
    for i in range(1000):
        bb = Aabb(rv(rng), rv(rng))
        c_out = (V * 4)()
        r_out = (V * 4)()
        c["c2BBVerts"](c_out, ct.byref(bb))
        rust["c2BBVerts"](r_out, ct.byref(bb))
        same(f"c2BBVerts[{i}]", c_out, r_out)

        shapes = [
            (0, Circle(rv(rng), rng.uniform(0, 20))),
            (1, bb),
            (2, Capsule(rv(rng), rv(rng), rng.uniform(0, 20))),
        ]
        for shape_type, shape in shapes:
            cp = Proxy()
            rp = Proxy()
            ct.memset(ct.byref(cp), 0xA5, ct.sizeof(cp))
            ct.memset(ct.byref(rp), 0xA5, ct.sizeof(rp))
            c["c2MakeProxy"](ct.byref(shape), shape_type, ct.byref(cp))
            rust["c2MakeProxy"](ct.byref(shape), shape_type, ct.byref(rp))
            same(f"c2MakeProxy[{i},{shape_type}]", cp, rp)

        verts = (V * 8)(*[rv(rng) for _ in range(8)])
        direction = rv(rng)
        for count in range(1, 9):
            ci = c["c2Support"](verts, count, direction)
            ri = rust["c2Support"](verts, count, direction)
            if ci != ri:
                raise AssertionError(f"c2Support[{i},{count}]: {ci} != {ri}")

        for count in range(4):
            source = rsimplex(rng, count)
            cs = clone(source)
            rs = clone(source)
            same(
                f"c2GJKSimplexMetric[{i},{count}]",
                c["c2GJKSimplexMetric"](ct.byref(cs)),
                rust["c2GJKSimplexMetric"](ct.byref(rs)),
            )
            same(f"c2D[{i},{count}]", c["c2D"](ct.byref(cs)), rust["c2D"](ct.byref(rs)))
            same(f"c2L[{i},{count}]", c["c2L"](ct.byref(cs)), rust["c2L"](ct.byref(rs)))
            ca, cb, ra, rb = V(), V(), V(), V()
            c["c2Witness"](ct.byref(cs), ct.byref(ca), ct.byref(cb))
            rust["c2Witness"](ct.byref(rs), ct.byref(ra), ct.byref(rb))
            same(f"c2Witness-a[{i},{count}]", ca, ra)
            same(f"c2Witness-b[{i},{count}]", cb, rb)

        for name, count in (("c22", 2), ("c23", 3)):
            source = rsimplex(rng, count)
            cs = clone(source)
            rs = clone(source)
            c[name](ct.byref(cs))
            rust[name](ct.byref(rs))
            same(f"{name}[{i}]", cs, rs)


def make_shape(rng, shape_type):
    if shape_type == 0:
        return Circle(rv(rng), rng.uniform(0, 10))
    if shape_type == 1:
        center = rv(rng)
        half = V(rng.uniform(0.01, 10), rng.uniform(0.01, 10))
        return Aabb(V(center.x - half.x, center.y - half.y), V(center.x + half.x, center.y + half.y))
    return Capsule(rv(rng), rv(rng), rng.uniform(0, 10))


def random_transform(rng):
    angle = rng.uniform(-math.pi, math.pi)
    return X(rv(rng), R(math.cos(angle), math.sin(angle)))


def run_gjk(fn, a_shape, type_a, ax, b_shape, type_b, bx, use_radius, cache):
    out_a = V()
    out_b = V()
    iterations = ct.c_int(-1)
    distance = fn(
        ct.byref(a_shape),
        type_a,
        None if ax is None else ct.byref(ax),
        ct.byref(b_shape),
        type_b,
        None if bx is None else ct.byref(bx),
        ct.byref(out_a),
        ct.byref(out_b),
        use_radius,
        ct.byref(iterations),
        ct.byref(cache),
    )
    return distance, out_a, out_b, iterations, cache


def compare_gjk_result(label, c_result, r_result):
    for field, left, right in zip(
        ("distance", "out_a", "out_b", "iterations", "cache"),
        c_result,
        r_result,
    ):
        same(f"{label}-{field}", left, right)


def check_gjk(c, rust, rng):
    case = 0
    for type_a in range(3):
        for type_b in range(3):
            for i in range(300):
                a_shape = make_shape(rng, type_a)
                b_shape = make_shape(rng, type_b)
                ax = None if i % 3 == 0 else random_transform(rng)
                bx = None if i % 4 == 0 else random_transform(rng)
                use_radius = i & 1
                c_cache = Cache()
                r_cache = Cache()
                c_result = run_gjk(c["c2GJK"], a_shape, type_a, ax, b_shape, type_b, bx, use_radius, c_cache)
                r_result = run_gjk(rust["c2GJK"], a_shape, type_a, ax, b_shape, type_b, bx, use_radius, r_cache)
                compare_gjk_result(f"c2GJK-cold[{case}]", c_result, r_result)

                c_result = run_gjk(c["c2GJK"], a_shape, type_a, ax, b_shape, type_b, bx, use_radius, c_result[4])
                r_result = run_gjk(rust["c2GJK"], a_shape, type_a, ax, b_shape, type_b, bx, use_radius, r_result[4])
                compare_gjk_result(f"c2GJK-hot[{case}]", c_result, r_result)
                case += 1

    for i in range(5000):
        values = [rng.uniform(-100, 100) for _ in range(8)]
        radius = rng.uniform(0, 20)
        ca, cb, ra, rb = V(), V(), V(), V()
        args = [ct.c_char(i & 1), *values, radius]
        c["gjk"](args[0], ct.byref(ca), ct.byref(cb), *args[1:])
        rust["gjk"](args[0], ct.byref(ra), ct.byref(rb), *args[1:])
        same(f"gjk-a[{i}]", ca, ra)
        same(f"gjk-b[{i}]", cb, rb)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(f"usage: {sys.argv[0]} C_SO RUST_SO")
    for kind, expected in EXPECTED_SIZES.items():
        if ct.sizeof(kind) != expected:
            raise AssertionError(f"unexpected {kind.__name__} size: {ct.sizeof(kind)}")

    c = bind_all(ct.CDLL(sys.argv[1]))
    rust = bind_all(ct.CDLL(sys.argv[2]))
    rng = random.Random(0xC2_2026)
    check_value_functions(c, rust, rng)
    check_pointer_helpers(c, rust, rng)
    check_gjk(c, rust, rng)
    print("all 31 exports matched byte-for-byte")


if __name__ == "__main__":
    main()
