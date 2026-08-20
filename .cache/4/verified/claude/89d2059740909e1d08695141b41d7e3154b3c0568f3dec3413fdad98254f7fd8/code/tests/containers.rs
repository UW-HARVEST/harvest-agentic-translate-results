//! Phase B — differential tests for the lowest-level entry points: the
//! `DEFINE_ARRAY` / `DEFINE_LIST` families, for all four instantiations
//! (`int`, `double`, `item_t`, `order_t`).
//!
//! Rows B1–B17 of `CONFIGS.md`. Every call goes through the exported C symbols
//! of both shared libraries.

mod common;

use common::*;
use libc::c_int;

/// Runs a generic row against all four element-type instantiations.
macro_rules! all_types {
    ($f:ident) => {{
        $f::<c_int>();
        $f::<f64>();
        $f::<ItemT>();
        $f::<OrderT>();
    }};
}

// ============================================================================
// Arrays
// ============================================================================

unsafe fn build_array<T: Elem>(
    api: &ArrayApi<T>,
    capacity: usize,
    values: &[T],
) -> (*mut ArrayRaw<T>, Vec<c_int>) {
    let arr = (api.create)(capacity);
    assert!(!arr.is_null(), "create({capacity}) returned NULL");
    let mut rets = Vec::with_capacity(values.len());
    for v in values {
        rets.push((api.push)(arr, *v));
    }
    (arr, rets)
}

/// Compares size, capacity, every element, the `push` return codes and the
/// result of `get` on every index between the two libraries.
unsafe fn compare_arrays<T: Elem>(
    what: &str,
    api_c: &ArrayApi<T>,
    api_rs: &ArrayApi<T>,
    c_arr: *mut ArrayRaw<T>,
    rs_arr: *mut ArrayRaw<T>,
    c_rets: &[c_int],
    rs_rets: &[c_int],
) {
    assert_eq!(c_rets, rs_rets, "{what}: push return codes differ");

    let (c_size, c_cap, c_elems) = array_state(c_arr);
    let (rs_size, rs_cap, rs_elems) = array_state(rs_arr);
    assert_eq!(c_size, rs_size, "{what}: size differs");
    assert_eq!(c_cap, rs_cap, "{what}: capacity differs");
    assert!(!(*c_arr).data.is_null() && !(*rs_arr).data.is_null());
    assert_eq!(
        (api_c.size)(c_arr),
        (api_rs.size)(rs_arr),
        "{what}: *_size() differs"
    );
    assert_elems_eq(&format!("{what}: backing store"), &c_elems, &rs_elems);

    let mut c_gets = Vec::new();
    let mut rs_gets = Vec::new();
    for i in 0..c_size {
        c_gets.push((api_c.get)(c_arr, i));
        rs_gets.push((api_rs.get)(rs_arr, i));
    }
    assert_elems_eq(&format!("{what}: get()"), &c_gets, &rs_gets);
}

/// B1 — `create(0)` forces capacity 16.
fn b1<T: Elem>() {
    let p = ArrayPair::<T>::new();
    unsafe {
        for cap in [0usize, 1, 2, 16, 17, 64, 1000] {
            let c = (p.c.create)(cap);
            let rs = (p.rs.create)(cap);
            assert!(!c.is_null() && !rs.is_null());
            assert_eq!((*c).size, 0);
            assert_eq!((*c).capacity, (*rs).capacity, "capacity for create({cap})");
            assert_eq!((*rs).size, 0);
            assert_eq!((p.c.size)(c), (p.rs.size)(rs));
            assert_eq!(
                (*c).capacity,
                if cap > 0 { cap } else { 16 },
                "create({cap}) capacity"
            );
            (p.c.destroy)(c);
            (p.rs.destroy)(rs);
        }
    }
}

#[test]
fn b1_array_create_capacities() {
    all_types!(b1);
}

/// B2/B3/B4/B5 — the growth ladder: exact fit, one doubling, repeated doublings.
fn b2345<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0x0B23_4501);
    unsafe {
        for (cap, count, expect_cap) in [
            (16usize, 16usize, 16usize), // B2: exact fit, no realloc
            (16, 17, 32),                // B3: one doubling
            (1, 1, 1),                   // B4
            (1, 2, 2),                   // B4
            (1, 3, 4),                   // B4
            (1, 500, 512),               // B5: nine doublings
            (0, 17, 32),                 // default capacity then doubling
            (3, 96, 96),                 // non-power-of-two growth: 3,6,12,24,48,96
            (3, 100, 192),               // one more doubling past 96
        ] {
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();
            let (c, c_rets) = build_array(&p.c, cap, &values);
            let (rs, rs_rets) = build_array(&p.rs, cap, &values);
            let what = format!("array<{}> create({cap}) + {count} pushes", T::SUFFIX);
            compare_arrays(&what, &p.c, &p.rs, c, rs, &c_rets, &rs_rets);
            assert_eq!((*c).capacity, expect_cap, "{what}: expected capacity");
            assert_elems_eq(&what, &values, &array_state(c).2);
            (p.c.destroy)(c);
            (p.rs.destroy)(rs);
        }
    }
}

#[test]
fn b2345_array_growth_ladder() {
    all_types!(b2345);
}

/// B6 — randomized capacities, counts and values.
fn b6<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0xB6_5EED);
    unsafe {
        for _ in 0..200 {
            let cap = rng.below(64);
            let count = rng.below(301);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();
            let (c, c_rets) = build_array(&p.c, cap, &values);
            let (rs, rs_rets) = build_array(&p.rs, cap, &values);
            compare_arrays(
                &format!("array<{}> random cap={cap} count={count}", T::SUFFIX),
                &p.c,
                &p.rs,
                c,
                rs,
                &c_rets,
                &rs_rets,
            );
            (p.c.destroy)(c);
            (p.rs.destroy)(rs);
        }
    }
}

#[test]
fn b6_array_randomized() {
    all_types!(b6);
}

/// B7/B8 — `clear` keeps the capacity and the slots; refilling starts at 0.
fn b78<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0xB7_B8_01);
    unsafe {
        for _ in 0..50 {
            let cap = 1 + rng.below(32);
            let count = rng.below(64);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();
            let (c, _) = build_array(&p.c, cap, &values);
            let (rs, _) = build_array(&p.rs, cap, &values);

            let cap_before = (*c).capacity;
            (p.c.clear)(c);
            (p.rs.clear)(rs);
            assert_eq!((p.c.size)(c), 0);
            assert_eq!((p.rs.size)(rs), 0);
            assert_eq!((*c).capacity, cap_before);
            assert_eq!((*rs).capacity, cap_before);

            // B7: the slots are still there -- `get` has no bounds check.
            let mut c_gets = Vec::new();
            let mut rs_gets = Vec::new();
            for i in 0..count {
                c_gets.push((p.c.get)(c, i));
                rs_gets.push((p.rs.get)(rs, i));
            }
            assert_elems_eq("get() after clear()", &c_gets, &rs_gets);
            assert_elems_eq("get() after clear() == pushed", &values, &c_gets);

            // B8: refill from index 0.
            let refill: Vec<T> = (0..rng.below(count + 1)).map(|_| T::rand(&mut rng)).collect();
            let mut c_rets = Vec::new();
            let mut rs_rets = Vec::new();
            for v in &refill {
                c_rets.push((p.c.push)(c, *v));
                rs_rets.push((p.rs.push)(rs, *v));
            }
            compare_arrays(
                &format!("array<{}> refill after clear", T::SUFFIX),
                &p.c,
                &p.rs,
                c,
                rs,
                &c_rets,
                &rs_rets,
            );
            assert_eq!((*c).capacity, cap_before, "no growth while under capacity");

            (p.c.destroy)(c);
            (p.rs.destroy)(rs);
        }
    }
}

#[test]
fn b78_array_clear_and_reuse() {
    all_types!(b78);
}

/// B9 — extreme element values.
#[test]
fn b9_array_extreme_values() {
    unsafe {
        let ints = ArrayPair::<c_int>::new();
        let values: Vec<c_int> = SPECIAL_INTS.to_vec();
        let (c, cr) = build_array(&ints.c, 2, &values);
        let (rs, rr) = build_array(&ints.rs, 2, &values);
        compare_arrays("array<int> extremes", &ints.c, &ints.rs, c, rs, &cr, &rr);
        (ints.c.destroy)(c);
        (ints.rs.destroy)(rs);

        let doubles = ArrayPair::<f64>::new();
        let values: Vec<f64> = SPECIAL_DOUBLES.to_vec();
        let (c, cr) = build_array(&doubles.c, 1, &values);
        let (rs, rr) = build_array(&doubles.rs, 1, &values);
        compare_arrays(
            "array<double> extremes",
            &doubles.c,
            &doubles.rs,
            c,
            rs,
            &cr,
            &rr,
        );
        (doubles.c.destroy)(c);
        (doubles.rs.destroy)(rs);

        // item_t / order_t built from every special double and int.
        let items = ArrayPair::<ItemT>::new();
        let mut values: Vec<ItemT> = Vec::new();
        for (i, price) in SPECIAL_DOUBLES.iter().enumerate() {
            let mut name = [0u8; MAX_NAME_LENGTH];
            name[..3].copy_from_slice(b"abc");
            let mut category = [0u8; MAX_CATEGORY_LENGTH];
            category[..2].copy_from_slice(b"xy");
            values.push(ItemT {
                id: SPECIAL_INTS[i % SPECIAL_INTS.len()],
                name,
                category,
                price: *price,
                quantity: SPECIAL_INTS[(i + 1) % SPECIAL_INTS.len()],
            });
        }
        let (c, cr) = build_array(&items.c, 1, &values);
        let (rs, rr) = build_array(&items.rs, 1, &values);
        compare_arrays("array<item_t> extremes", &items.c, &items.rs, c, rs, &cr, &rr);
        (items.c.destroy)(c);
        (items.rs.destroy)(rs);

        let orders = ArrayPair::<OrderT>::new();
        let values: Vec<OrderT> = SPECIAL_DOUBLES
            .iter()
            .enumerate()
            .map(|(i, amount)| {
                let mut customer_name = [0u8; MAX_NAME_LENGTH];
                customer_name[..4].copy_from_slice(b"Cust");
                OrderT {
                    customer_id: SPECIAL_INTS[i % SPECIAL_INTS.len()],
                    customer_name,
                    total_amount: *amount,
                }
            })
            .collect();
        let (c, cr) = build_array(&orders.c, 1, &values);
        let (rs, rr) = build_array(&orders.rs, 1, &values);
        compare_arrays(
            "array<order_t> extremes",
            &orders.c,
            &orders.rs,
            c,
            rs,
            &cr,
            &rr,
        );
        (orders.c.destroy)(c);
        (orders.rs.destroy)(rs);
    }
}

/// B10 — a container built by one library is read by the other.
fn b10<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0xB10_0001);
    unsafe {
        for _ in 0..20 {
            let count = rng.below(40);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();

            // Built by C, read through the Rust exports.
            let (c_arr, _) = build_array(&p.c, 1 + rng.below(8), &values);
            assert_eq!((p.rs.size)(c_arr), (p.c.size)(c_arr));
            let mut through_rust = Vec::new();
            for i in 0..count {
                through_rust.push((p.rs.get)(c_arr, i));
            }
            assert_elems_eq("C-built array read by Rust", &values, &through_rust);
            (p.rs.clear)(c_arr);
            assert_eq!((p.c.size)(c_arr), 0);
            (p.c.destroy)(c_arr);

            // Built by Rust, read through the C exports.
            let (rs_arr, _) = build_array(&p.rs, 1 + rng.below(8), &values);
            assert_eq!((p.c.size)(rs_arr), (p.rs.size)(rs_arr));
            let mut through_c = Vec::new();
            for i in 0..count {
                through_c.push((p.c.get)(rs_arr, i));
            }
            assert_elems_eq("Rust-built array read by C", &values, &through_c);
            (p.c.clear)(rs_arr);
            assert_eq!((p.rs.size)(rs_arr), 0);
            (p.rs.destroy)(rs_arr);
        }
    }
}

#[test]
fn b10_array_cross_library_interop() {
    all_types!(b10);
}

// ============================================================================
// Lists
// ============================================================================

/// B11 — freshly created list.
fn b11<T: Elem>() {
    let p = ListPair::<T>::new();
    unsafe {
        let c = (p.c.create)();
        let rs = (p.rs.create)();
        assert!(!c.is_null() && !rs.is_null());
        for (label, l) in [("C", c), ("Rust", rs)] {
            assert!((*l).head.is_null(), "{label}: head must be NULL");
            assert!((*l).tail.is_null(), "{label}: tail must be NULL");
            assert_eq!((*l).size, 0, "{label}: size must be 0");
        }
        assert_eq!((p.c.size)(c), (p.rs.size)(rs));
        (p.c.destroy)(c);
        (p.rs.destroy)(rs);
    }
}

#[test]
fn b11_list_create() {
    all_types!(b11);
}

#[derive(Clone, Copy, Debug)]
enum ListOp {
    Append(usize),
    Prepend(usize),
    Size,
    Clear,
}

/// Replays one op script against a library and returns the observable results.
unsafe fn run_list_script<T: Elem>(
    api: &ListApi<T>,
    ops: &[ListOp],
    values: &[T],
) -> (Vec<c_int>, Vec<usize>, usize, bool, bool, Vec<T>) {
    let list = (api.create)();
    assert!(!list.is_null());
    let mut rets = Vec::new();
    let mut sizes = Vec::new();
    for op in ops {
        match *op {
            ListOp::Append(i) => rets.push((api.append)(list, values[i])),
            ListOp::Prepend(i) => rets.push((api.prepend)(list, values[i])),
            ListOp::Size => sizes.push((api.size)(list)),
            ListOp::Clear => (api.clear)(list),
        }
    }
    let (size, head_null, tail_is_last, elems) = list_state(list);
    (api.destroy)(list);
    (rets, sizes, size, head_null, tail_is_last, elems)
}

unsafe fn compare_list_script<T: Elem>(what: &str, ops: &[ListOp], values: &[T]) {
    let p = ListPair::<T>::new();
    let c = run_list_script(&p.c, ops, values);
    let rs = run_list_script(&p.rs, ops, values);
    assert_eq!(c.0, rs.0, "{what}: append/prepend return codes differ");
    assert_eq!(c.1, rs.1, "{what}: size() results differ");
    assert_eq!(c.2, rs.2, "{what}: size field differs");
    assert_eq!(c.3, rs.3, "{what}: head-NULL-ness differs");
    assert_eq!(c.4, rs.4, "{what}: tail does not point at the last node");
    assert!(c.4, "{what}: C tail is not the last node (harness bug)");
    assert_elems_eq(&format!("{what}: chain"), &c.5, &rs.5);
    assert_eq!(c.2, c.5.len(), "{what}: C size field vs chain length");
    assert_eq!(rs.2, rs.5.len(), "{what}: Rust size field vs chain length");
}

/// B12 — `append` into empty and non-empty lists.
fn b12<T: Elem>() {
    let mut rng = Rng::new(0xB12_0002);
    unsafe {
        for count in [0usize, 1, 2, 3, 17, 200] {
            let values: Vec<T> = (0..count.max(1)).map(|_| T::rand(&mut rng)).collect();
            let ops: Vec<ListOp> = (0..count)
                .flat_map(|i| [ListOp::Append(i), ListOp::Size])
                .collect();
            compare_list_script(
                &format!("list<{}> append x{count}", T::SUFFIX),
                &ops,
                &values,
            );
        }
    }
}

#[test]
fn b12_list_append() {
    all_types!(b12);
}

/// B13 — `prepend` into empty and non-empty lists (reverses the order, and the
/// first prepend is the one that sets `tail`).
fn b13<T: Elem>() {
    let mut rng = Rng::new(0xB13_0003);
    unsafe {
        for count in [0usize, 1, 2, 3, 17, 200] {
            let values: Vec<T> = (0..count.max(1)).map(|_| T::rand(&mut rng)).collect();
            let ops: Vec<ListOp> = (0..count)
                .flat_map(|i| [ListOp::Prepend(i), ListOp::Size])
                .collect();
            compare_list_script(
                &format!("list<{}> prepend x{count}", T::SUFFIX),
                &ops,
                &values,
            );
        }
    }
}

#[test]
fn b13_list_prepend() {
    all_types!(b13);
}

/// B14 — interleaved append/prepend.
fn b14<T: Elem>() {
    let mut rng = Rng::new(0xB14_0004);
    unsafe {
        for _ in 0..20 {
            let count = 1 + rng.below(300);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();
            let ops: Vec<ListOp> = (0..count)
                .map(|i| {
                    if rng.below(2) == 0 {
                        ListOp::Append(i)
                    } else {
                        ListOp::Prepend(i)
                    }
                })
                .collect();
            compare_list_script(
                &format!("list<{}> interleaved x{count}", T::SUFFIX),
                &ops,
                &values,
            );
        }
    }
}

#[test]
fn b14_list_interleaved() {
    all_types!(b14);
}

/// B15 — reuse after `clear` (the next insert must take the "empty" branch).
fn b15<T: Elem>() {
    let mut rng = Rng::new(0xB15_0005);
    unsafe {
        for _ in 0..20 {
            let count = 1 + rng.below(20);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();
            let mut ops: Vec<ListOp> = (0..count).map(ListOp::Append).collect();
            ops.push(ListOp::Clear);
            ops.push(ListOp::Size);
            for i in 0..count {
                ops.push(if i % 2 == 0 {
                    ListOp::Append(i)
                } else {
                    ListOp::Prepend(i)
                });
            }
            ops.push(ListOp::Clear);
            ops.push(ListOp::Clear); // clearing twice must stay consistent
            ops.push(ListOp::Append(0));
            compare_list_script(&format!("list<{}> clear+reuse", T::SUFFIX), &ops, &values);
        }
    }
}

#[test]
fn b15_list_clear_and_reuse() {
    all_types!(b15);
}

/// B16 — long randomized op scripts mixing every list entry point.
fn b16<T: Elem>() {
    let mut rng = Rng::new(0xB16_0006);
    unsafe {
        for _ in 0..10 {
            let values: Vec<T> = (0..64).map(|_| T::rand(&mut rng)).collect();
            let ops: Vec<ListOp> = (0..500)
                .map(|_| match rng.below(20) {
                    0 => ListOp::Clear,
                    1..=3 => ListOp::Size,
                    4..=11 => ListOp::Append(rng.below(64)),
                    _ => ListOp::Prepend(rng.below(64)),
                })
                .collect();
            compare_list_script(&format!("list<{}> random script", T::SUFFIX), &ops, &values);
        }
    }
}

#[test]
fn b16_list_random_scripts() {
    all_types!(b16);
}

/// B17 — a list built by one library is read by the other.
fn b17<T: Elem>() {
    let p = ListPair::<T>::new();
    let mut rng = Rng::new(0xB17_0007);
    unsafe {
        for _ in 0..20 {
            let count = rng.below(30);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();

            let c_list = (p.c.create)();
            for v in &values {
                assert_eq!((p.rs.append)(c_list, *v), 0, "Rust append into C list");
            }
            assert_eq!((p.c.size)(c_list), count);
            assert_eq!((p.rs.size)(c_list), count);
            let (_, _, _, elems) = list_state(c_list);
            assert_elems_eq("C-created list filled by Rust", &values, &elems);
            (p.rs.clear)(c_list);
            assert_eq!((p.c.size)(c_list), 0);
            (p.c.destroy)(c_list);

            let rs_list = (p.rs.create)();
            for v in &values {
                assert_eq!((p.c.prepend)(rs_list, *v), 0, "C prepend into Rust list");
            }
            assert_eq!((p.rs.size)(rs_list), count);
            let (_, _, _, elems) = list_state(rs_list);
            let mut reversed = values.clone();
            reversed.reverse();
            assert_elems_eq("Rust-created list filled by C", &reversed, &elems);
            (p.c.clear)(rs_list);
            assert_eq!((p.rs.size)(rs_list), 0);
            (p.rs.destroy)(rs_list);
        }
    }
}

#[test]
fn b17_list_cross_library_interop() {
    all_types!(b17);
}
