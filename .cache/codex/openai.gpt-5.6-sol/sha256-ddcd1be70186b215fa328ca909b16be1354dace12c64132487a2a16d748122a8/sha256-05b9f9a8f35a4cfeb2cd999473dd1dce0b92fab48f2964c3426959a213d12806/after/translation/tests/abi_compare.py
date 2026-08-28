#!/usr/bin/env python3

import ctypes as C
import pathlib
import random
import struct


class V(C.Structure):
    _fields_ = [("x", C.c_float), ("y", C.c_float)]


class R(C.Structure):
    _fields_ = [("c", C.c_float), ("s", C.c_float)]


class X(C.Structure):
    _fields_ = [("p", V), ("r", R)]


class Circle(C.Structure):
    _fields_ = [("p", V), ("r", C.c_float)]


class AABB(C.Structure):
    _fields_ = [("min", V), ("max", V)]


class Capsule(C.Structure):
    _fields_ = [("a", V), ("b", V), ("r", C.c_float)]


class Cache(C.Structure):
    _fields_ = [
        ("metric", C.c_float),
        ("count", C.c_int),
        ("iA", C.c_int * 3),
        ("iB", C.c_int * 3),
        ("div", C.c_float),
    ]


class Proxy(C.Structure):
    _fields_ = [("radius", C.c_float), ("count", C.c_int), ("verts", V * 8)]


class SV(C.Structure):
    _fields_ = [
        ("sA", V),
        ("sB", V),
        ("p", V),
        ("u", C.c_float),
        ("iA", C.c_int),
        ("iB", C.c_int),
    ]


class Simplex(C.Structure):
    _fields_ = [
        ("a", SV),
        ("b", SV),
        ("c", SV),
        ("d", SV),
        ("div", C.c_float),
        ("count", C.c_int),
    ]


ROOT = pathlib.Path(__file__).resolve().parents[2]
C_LIB = C.CDLL(str(ROOT / ".reference-build" / "libharvest-work-NK53rq.so"))
RUST_LIB = C.CDLL(str(ROOT / "translation" / "target" / "release" / "libtranslation.so"))

SPECS = {
    "c2V": (V, [C.c_float, C.c_float]),
    "c2Mulvs": (V, [V, C.c_float]),
    "c2Maxv": (V, [V, V]),
    "c2Minv": (V, [V, V]),
    "c2Clampv": (V, [V, V, V]),
    "c2Sub": (V, [V, V]),
    "c2Dot": (C.c_float, [V, V]),
    "c2RotIdentity": (R, []),
    "c2xIdentity": (X, []),
    "c2BBVerts": (None, [C.POINTER(V), C.POINTER(AABB)]),
    "c2MakeProxy": (None, [C.c_void_p, C.c_int, C.POINTER(Proxy)]),
    "c2Len": (C.c_float, [V]),
    "c2Det2": (C.c_float, [V, V]),
    "c2GJKSimplexMetric": (C.c_float, [C.POINTER(Simplex)]),
    "c2Mulrv": (V, [R, V]),
    "c2Add": (V, [V, V]),
    "c2Mulxv": (V, [X, V]),
    "c22": (None, [C.POINTER(Simplex)]),
    "c23": (None, [C.POINTER(Simplex)]),
    "c2Neg": (V, [V]),
    "c2Skew": (V, [V]),
    "c2CCW90": (V, [V]),
    "c2D": (V, [C.POINTER(Simplex)]),
    "c2Support": (C.c_int, [C.POINTER(V), C.c_int, V]),
    "c2Witness": (None, [C.POINTER(Simplex), C.POINTER(V), C.POINTER(V)]),
    "c2Div": (V, [V, C.c_float]),
    "c2Norm": (V, [V]),
    "c2L": (V, [C.POINTER(Simplex)]),
    "c2MulrvT": (V, [R, V]),
    "c2GJK": (
        C.c_float,
        [
            C.c_void_p,
            C.c_int,
            C.POINTER(X),
            C.c_void_p,
            C.c_int,
            C.POINTER(X),
            C.POINTER(V),
            C.POINTER(V),
            C.c_int,
            C.POINTER(C.c_int),
            C.POINTER(Cache),
        ],
    ),
    "c2AABBtoAABB": (C.c_int, [AABB, AABB]),
    "c2AABBtoCapsule": (C.c_int, [AABB, Capsule]),
    "c2CapsuletoCapsule": (C.c_int, [Capsule, Capsule]),
    "c2CircletoCircle": (C.c_int, [Circle, Circle]),
    "c2CircletoAABB": (C.c_int, [Circle, AABB]),
    "c2CircletoCapsule": (C.c_int, [Circle, Capsule]),
    "c2Collided": (C.c_int, [C.c_void_p, C.c_int, C.c_void_p, C.c_int]),
    "capsule": (C.c_int, [C.c_float] * 5),
}

for lib in (C_LIB, RUST_LIB):
    for name, (restype, argtypes) in SPECS.items():
        fn = getattr(lib, name)
        fn.restype = restype
        fn.argtypes = argtypes

covered = set()


def raw(value):
    if isinstance(value, float):
        return struct.pack("=f", value)
    if isinstance(value, int):
        return struct.pack("=i", value)
    return C.string_at(C.byref(value), C.sizeof(value))


def same(name, left, right):
    covered.add(name)
    if raw(left) != raw(right):
        raise AssertionError(f"{name} mismatch: {raw(left).hex()} != {raw(right).hex()}")


def clone(value):
    return type(value).from_buffer_copy(raw(value))


def f32(value):
    return C.c_float(value).value


def rf():
    return f32(random.uniform(-1000.0, 1000.0))


def rv():
    return V(rf(), rf())


def rr():
    return R(rf(), rf())


def rx():
    return X(rv(), rr())


def rcircle():
    return Circle(rv(), f32(random.uniform(0.0, 100.0)))


def raabb():
    a = rv()
    b = rv()
    return AABB(V(min(a.x, b.x), min(a.y, b.y)), V(max(a.x, b.x), max(a.y, b.y)))


def rcapsule():
    return Capsule(rv(), rv(), f32(random.uniform(0.0, 100.0)))


def rsv():
    return SV(rv(), rv(), rv(), rf(), random.randrange(4), random.randrange(4))


def rsimplex(count=None):
    return Simplex(
        rsv(),
        rsv(),
        rsv(),
        rsv(),
        f32(random.uniform(0.1, 1000.0)),
        count if count is not None else random.randrange(1, 4),
    )


assert {
    V: 8,
    R: 8,
    X: 16,
    Circle: 12,
    AABB: 16,
    Capsule: 20,
    Cache: 36,
    Proxy: 72,
    SV: 36,
    Simplex: 152,
} == {
    kind: C.sizeof(kind)
    for kind in (V, R, X, Circle, AABB, Capsule, Cache, Proxy, SV, Simplex)
}

random.seed(0xC2)

edge_floats = [
    0.0,
    -0.0,
    f32(1.401298464324817e-45),
    f32(-1.401298464324817e-45),
    1.0,
    -1.0,
    float("inf"),
    float("-inf"),
    float("nan"),
]
for x in edge_floats:
    for y in edge_floats:
        a = V(x, y)
        b = V(y, x)
        for name, args in (
            ("c2V", (x, y)),
            ("c2Mulvs", (a, y)),
            ("c2Maxv", (a, b)),
            ("c2Minv", (a, b)),
            ("c2Sub", (a, b)),
            ("c2Dot", (a, b)),
            ("c2Len", (a,)),
            ("c2Det2", (a, b)),
            ("c2Add", (a, b)),
            ("c2Neg", (a,)),
            ("c2Skew", (a,)),
            ("c2CCW90", (a,)),
        ):
            same(name, getattr(C_LIB, name)(*args), getattr(RUST_LIB, name)(*args))

for _ in range(2000):
    a, b, lo, hi = rv(), rv(), rv(), rv()
    scalar = rf()
    rotation = rr()
    transform = rx()
    for name, args in (
        ("c2V", (a.x, a.y)),
        ("c2Mulvs", (a, scalar)),
        ("c2Maxv", (a, b)),
        ("c2Minv", (a, b)),
        ("c2Clampv", (a, lo, hi)),
        ("c2Sub", (a, b)),
        ("c2Dot", (a, b)),
        ("c2Len", (a,)),
        ("c2Det2", (a, b)),
        ("c2Mulrv", (rotation, a)),
        ("c2Add", (a, b)),
        ("c2Mulxv", (transform, a)),
        ("c2Neg", (a,)),
        ("c2Skew", (a,)),
        ("c2CCW90", (a,)),
        ("c2Div", (a, scalar if scalar != 0.0 else 1.0)),
        ("c2Norm", (a,)),
        ("c2MulrvT", (rotation, a)),
    ):
        same(name, getattr(C_LIB, name)(*args), getattr(RUST_LIB, name)(*args))

same("c2RotIdentity", C_LIB.c2RotIdentity(), RUST_LIB.c2RotIdentity())
same("c2xIdentity", C_LIB.c2xIdentity(), RUST_LIB.c2xIdentity())

for _ in range(1000):
    box = raabb()
    c_out = (V * 4)()
    r_out = (V * 4)()
    C_LIB.c2BBVerts(c_out, C.byref(box))
    RUST_LIB.c2BBVerts(r_out, C.byref(box))
    covered.add("c2BBVerts")
    if bytes(c_out) != bytes(r_out):
        raise AssertionError("c2BBVerts mismatch")

    for type_, shape in enumerate((rcircle(), raabb(), rcapsule())):
        initial = Proxy(rf(), random.randrange(-10, 10), (V * 8)(*(rv() for _ in range(8))))
        cp = clone(initial)
        rp = clone(initial)
        C_LIB.c2MakeProxy(C.byref(shape), type_, C.byref(cp))
        RUST_LIB.c2MakeProxy(C.byref(shape), type_, C.byref(rp))
        same("c2MakeProxy", cp, rp)
    initial = Proxy(rf(), random.randrange(-10, 10), (V * 8)(*(rv() for _ in range(8))))
    cp = clone(initial)
    rp = clone(initial)
    C_LIB.c2MakeProxy(C.byref(box), 99, C.byref(cp))
    RUST_LIB.c2MakeProxy(C.byref(box), 99, C.byref(rp))
    same("c2MakeProxy", cp, rp)

for count in (1, 2, 3, 4):
    for _ in range(1000):
        simplex = rsimplex(count)
        same(
            "c2GJKSimplexMetric",
            C_LIB.c2GJKSimplexMetric(C.byref(simplex)),
            RUST_LIB.c2GJKSimplexMetric(C.byref(simplex)),
        )
        same("c2D", C_LIB.c2D(C.byref(simplex)), RUST_LIB.c2D(C.byref(simplex)))
        same("c2L", C_LIB.c2L(C.byref(simplex)), RUST_LIB.c2L(C.byref(simplex)))
        ca, cb, ra, rb = V(), V(), V(), V()
        C_LIB.c2Witness(C.byref(simplex), C.byref(ca), C.byref(cb))
        RUST_LIB.c2Witness(C.byref(simplex), C.byref(ra), C.byref(rb))
        same("c2Witness", ca, ra)
        same("c2Witness", cb, rb)

for name, count in (("c22", 2), ("c23", 3)):
    for _ in range(5000):
        original = rsimplex(count)
        cs = clone(original)
        rs = clone(original)
        getattr(C_LIB, name)(C.byref(cs))
        getattr(RUST_LIB, name)(C.byref(rs))
        same(name, cs, rs)

for _ in range(2000):
    values = (V * 8)(*(rv() for _ in range(8)))
    count = random.randrange(1, 9)
    direction = rv()
    same(
        "c2Support",
        C_LIB.c2Support(values, count, direction),
        RUST_LIB.c2Support(values, count, direction),
    )


def shape_for(type_):
    return (rcircle, raabb, rcapsule)[type_]()


for _ in range(3000):
    type_a = random.randrange(3)
    type_b = random.randrange(3)
    shape_a = shape_for(type_a)
    shape_b = shape_for(type_b)
    ax = rx()
    bx = rx()
    use_radius = random.randrange(2)
    ax_ptr = C.byref(ax) if random.randrange(2) else None
    bx_ptr = C.byref(bx) if random.randrange(2) else None
    ca, cb, ra, rb = V(), V(), V(), V()
    ci, ri = C.c_int(), C.c_int()
    cc, rc = Cache(), Cache()
    cd = C_LIB.c2GJK(
        C.byref(shape_a),
        type_a,
        ax_ptr,
        C.byref(shape_b),
        type_b,
        bx_ptr,
        C.byref(ca),
        C.byref(cb),
        use_radius,
        C.byref(ci),
        C.byref(cc),
    )
    rd = RUST_LIB.c2GJK(
        C.byref(shape_a),
        type_a,
        ax_ptr,
        C.byref(shape_b),
        type_b,
        bx_ptr,
        C.byref(ra),
        C.byref(rb),
        use_radius,
        C.byref(ri),
        C.byref(rc),
    )
    same("c2GJK", cd, rd)
    same("c2GJK", ca, ra)
    same("c2GJK", cb, rb)
    same("c2GJK", ci.value, ri.value)
    same("c2GJK", cc, rc)
    ca, cb, ra, rb = V(), V(), V(), V()
    ci, ri = C.c_int(), C.c_int()
    cd = C_LIB.c2GJK(
        C.byref(shape_a),
        type_a,
        ax_ptr,
        C.byref(shape_b),
        type_b,
        bx_ptr,
        C.byref(ca),
        C.byref(cb),
        use_radius,
        C.byref(ci),
        C.byref(cc),
    )
    rd = RUST_LIB.c2GJK(
        C.byref(shape_a),
        type_a,
        ax_ptr,
        C.byref(shape_b),
        type_b,
        bx_ptr,
        C.byref(ra),
        C.byref(rb),
        use_radius,
        C.byref(ri),
        C.byref(rc),
    )
    same("c2GJK", cd, rd)
    same("c2GJK", ca, ra)
    same("c2GJK", cb, rb)
    same("c2GJK", ci.value, ri.value)
    same("c2GJK", cc, rc)

for _ in range(5000):
    circle_a, circle_b = rcircle(), rcircle()
    box_a, box_b = raabb(), raabb()
    cap_a, cap_b = rcapsule(), rcapsule()
    for name, args in (
        ("c2AABBtoAABB", (box_a, box_b)),
        ("c2AABBtoCapsule", (box_a, cap_b)),
        ("c2CapsuletoCapsule", (cap_a, cap_b)),
        ("c2CircletoCircle", (circle_a, circle_b)),
        ("c2CircletoAABB", (circle_a, box_b)),
        ("c2CircletoCapsule", (circle_a, cap_b)),
    ):
        same(name, getattr(C_LIB, name)(*args), getattr(RUST_LIB, name)(*args))

    shapes = (circle_a, box_a, cap_a)
    other_shapes = (circle_b, box_b, cap_b)
    for type_a in range(3):
        for type_b in range(3):
            args = (
                C.byref(shapes[type_a]),
                type_a,
                C.byref(other_shapes[type_b]),
                type_b,
            )
            same("c2Collided", C_LIB.c2Collided(*args), RUST_LIB.c2Collided(*args))
    same(
        "c2Collided",
        C_LIB.c2Collided(None, 99, None, 99),
        RUST_LIB.c2Collided(None, 99, None, 99),
    )

    args = (rf(), rf(), rf(), rf(), f32(random.uniform(0.0, 100.0)))
    same("capsule", C_LIB.capsule(*args), RUST_LIB.capsule(*args))

missing = set(SPECS) - covered
if missing:
    raise AssertionError(f"uncovered exports: {sorted(missing)}")

print(f"all {len(covered)} exports matched")
