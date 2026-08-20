//! Phase B — differential tests for the `inventory.c` entry points
//! (rows B18–B35 of `CONFIGS.md`): `create_item`, `create_order`, `print_item`,
//! `print_order`, `calculate_inventory_stats`, `calculate_order_stats`,
//! `find_items_by_category`, `find_expensive_items`.

mod common;

use common::*;
use libc::{c_char, c_int};

fn apis() -> (InventoryApi, InventoryApi) {
    (InventoryApi::new(c_lib()), InventoryApi::new(rs_lib()))
}

unsafe fn build_item_array(api: &ArrayApi<ItemT>, items: &[ItemT]) -> *mut ArrayRaw<ItemT> {
    let arr = (api.create)(items.len().max(1));
    assert!(!arr.is_null());
    for it in items {
        assert_eq!((api.push)(arr, *it), 0);
    }
    arr
}

unsafe fn build_item_list(api: &ListApi<ItemT>, items: &[ItemT]) -> *mut ListRaw<ItemT> {
    let list = (api.create)();
    assert!(!list.is_null());
    for it in items {
        assert_eq!((api.append)(list, *it), 0);
    }
    list
}

fn item_with(name: &[u8], category: &[u8], price: f64, id: c_int, quantity: c_int) -> ItemT {
    let mut n = [0u8; MAX_NAME_LENGTH];
    let mut c = [0u8; MAX_CATEGORY_LENGTH];
    let nl = name.len().min(MAX_NAME_LENGTH - 1);
    let cl = category.len().min(MAX_CATEGORY_LENGTH - 1);
    n[..nl].copy_from_slice(&name[..nl]);
    c[..cl].copy_from_slice(&category[..cl]);
    ItemT {
        id,
        name: n,
        category: c,
        price,
        quantity,
    }
}

fn order_with(name: &[u8], total: f64, id: c_int) -> OrderT {
    let mut n = [0u8; MAX_NAME_LENGTH];
    let nl = name.len().min(MAX_NAME_LENGTH - 1);
    n[..nl].copy_from_slice(&name[..nl]);
    OrderT {
        customer_id: id,
        customer_name: n,
        total_amount: total,
    }
}

// ============================================================================
// B18/B19/B20 — create_item
// ============================================================================

#[test]
fn create_item_random_strings() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xB18_0018);
    unsafe {
        for _ in 0..3000 {
            let name = rng.ascii_string(80);
            let category = rng.ascii_string(40);
            let name_c = cstring(&name);
            let cat_c = cstring(&category);
            let id = if rng.below(4) == 0 {
                SPECIAL_INTS[rng.below(SPECIAL_INTS.len())]
            } else {
                rng.i32()
            };
            let quantity = if rng.below(4) == 0 {
                SPECIAL_INTS[rng.below(SPECIAL_INTS.len())]
            } else {
                rng.i32()
            };
            let price = rng.f64_any();

            let a = (c.create_item)(
                id,
                name_c.as_ptr() as *const c_char,
                cat_c.as_ptr() as *const c_char,
                price,
                quantity,
            );
            let b = (rs.create_item)(
                id,
                name_c.as_ptr() as *const c_char,
                cat_c.as_ptr() as *const c_char,
                price,
                quantity,
            );
            assert!(
                a.bit_eq(&b),
                "create_item(name len {}, category len {}) differs:\n  C:    {}\n  Rust: {}",
                name.len(),
                category.len(),
                a.describe(),
                b.describe()
            );
            // The C code forces the final byte to NUL and `strncpy` NUL-pads.
            assert_eq!(a.name[MAX_NAME_LENGTH - 1], 0);
            assert_eq!(a.category[MAX_CATEGORY_LENGTH - 1], 0);
        }
    }
}

#[test]
fn create_item_boundary_lengths() {
    let (c, rs) = apis();
    unsafe {
        for name_len in [0usize, 1, 61, 62, 63, 64, 65, 100] {
            for cat_len in [0usize, 1, 29, 30, 31, 32, 33, 50] {
                let name = vec![b'N'; name_len];
                let category = vec![b'C'; cat_len];
                let name_c = cstring(&name);
                let cat_c = cstring(&category);
                let a = (c.create_item)(
                    7,
                    name_c.as_ptr() as *const c_char,
                    cat_c.as_ptr() as *const c_char,
                    12.5,
                    3,
                );
                let b = (rs.create_item)(
                    7,
                    name_c.as_ptr() as *const c_char,
                    cat_c.as_ptr() as *const c_char,
                    12.5,
                    3,
                );
                assert!(
                    a.bit_eq(&b),
                    "create_item(name_len={name_len}, cat_len={cat_len}) differs:\n  C:    {}\n  Rust: {}",
                    a.describe(),
                    b.describe()
                );
                assert_eq!(
                    a.name.iter().position(|&x| x == 0).unwrap(),
                    name_len.min(MAX_NAME_LENGTH - 1),
                    "truncation point"
                );
            }
        }
    }
}

#[test]
fn create_item_non_ascii_and_specials() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xB20_0020);
    unsafe {
        for _ in 0..2000 {
            let name = rng.byte_string(80);
            let category = rng.byte_string(40);
            let name_c = cstring(&name);
            let cat_c = cstring(&category);
            for &price in SPECIAL_DOUBLES.iter() {
                let id = SPECIAL_INTS[rng.below(SPECIAL_INTS.len())];
                let quantity = SPECIAL_INTS[rng.below(SPECIAL_INTS.len())];
                let a = (c.create_item)(
                    id,
                    name_c.as_ptr() as *const c_char,
                    cat_c.as_ptr() as *const c_char,
                    price,
                    quantity,
                );
                let b = (rs.create_item)(
                    id,
                    name_c.as_ptr() as *const c_char,
                    cat_c.as_ptr() as *const c_char,
                    price,
                    quantity,
                );
                assert!(
                    a.bit_eq(&b),
                    "create_item with high bytes differs:\n  C:    {}\n  Rust: {}",
                    a.describe(),
                    b.describe()
                );
            }
        }
    }
}

// ============================================================================
// B21 — create_order
// ============================================================================

#[test]
fn create_order_random() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xB21_0021);
    unsafe {
        for _ in 0..3000 {
            let name = if rng.below(2) == 0 {
                rng.ascii_string(80)
            } else {
                rng.byte_string(80)
            };
            let name_c = cstring(&name);
            let id = if rng.below(4) == 0 {
                SPECIAL_INTS[rng.below(SPECIAL_INTS.len())]
            } else {
                rng.i32()
            };
            let total = rng.f64_any();
            let a = (c.create_order)(id, name_c.as_ptr() as *const c_char, total);
            let b = (rs.create_order)(id, name_c.as_ptr() as *const c_char, total);
            assert!(
                a.bit_eq(&b),
                "create_order(name len {}) differs:\n  C:    {}\n  Rust: {}",
                name.len(),
                a.describe(),
                b.describe()
            );
            assert_eq!(a.customer_name[MAX_NAME_LENGTH - 1], 0);
        }
    }
}

// ============================================================================
// B22/B23/B24 — print_item / print_order
// ============================================================================

#[test]
fn print_item_random() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xB22_0022);
    for i in 0..1500 {
        let item = ItemT::rand(&mut rng);
        assert_same_output(
            &format!("print_item random #{i}"),
            || unsafe { (c.print_item)(item) },
            || unsafe { (rs.print_item)(item) },
        );
    }
}

#[test]
fn print_item_price_shapes() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xB23_0023);
    let mut prices: Vec<f64> = SPECIAL_DOUBLES.to_vec();
    prices.extend([
        0.015, 0.025, 0.045, 2.5, 3.5, -2.5, -0.005, 1e-7, 999999.995, 1e21, 1e-320,
        f64::from_bits(0x7ff8_0000_0000_0001), // NaN with payload
        f64::from_bits(0xfff8_0000_0000_0001), // negative NaN with payload
        f64::from_bits(0x7ff0_0000_0000_0001), // signalling NaN
    ]);
    for _ in 0..200 {
        prices.push(rng.f64_any());
    }
    for (i, price) in prices.iter().enumerate() {
        let item = item_with(b"Item", b"Cat", *price, i as c_int, -3);
        assert_same_output(
            &format!("print_item price {price:?} ({:#018x})", price.to_bits()),
            || unsafe { (c.print_item)(item) },
            || unsafe { (rs.print_item)(item) },
        );
    }
}

#[test]
fn print_item_string_shapes() {
    let (c, rs) = apis();
    let (mut rng, mut cases): (Rng, Vec<ItemT>) = (Rng::new(0xB22_5555), Vec::new());
    // Empty, 1-byte, exactly-truncated and high-byte names/categories.
    for name_len in [0usize, 1, 62, 63, 70] {
        for cat_len in [0usize, 1, 30, 31, 40] {
            cases.push(item_with(
                &vec![b'x'; name_len],
                &vec![b'y'; cat_len],
                19.99,
                1,
                2,
            ));
        }
    }
    for _ in 0..300 {
        let name = rng.byte_string(63);
        let category = rng.byte_string(31);
        cases.push(item_with(&name, &category, rng.price(), rng.i32(), rng.i32()));
    }
    for (i, item) in cases.iter().enumerate() {
        let item = *item;
        assert_same_output(
            &format!("print_item strings #{i}"),
            || unsafe { (c.print_item)(item) },
            || unsafe { (rs.print_item)(item) },
        );
    }
}

#[test]
fn print_order_random_and_shapes() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xB24_0024);
    for i in 0..1000 {
        let order = OrderT::rand(&mut rng);
        assert_same_output(
            &format!("print_order random #{i}"),
            || unsafe { (c.print_order)(order) },
            || unsafe { (rs.print_order)(order) },
        );
    }
    for (i, &total) in SPECIAL_DOUBLES.iter().enumerate() {
        for name_len in [0usize, 1, 63, 80] {
            let order = order_with(&vec![b'N'; name_len], total, i as c_int);
            assert_same_output(
                &format!("print_order total {total:?} name_len {name_len}"),
                || unsafe { (c.print_order)(order) },
                || unsafe { (rs.print_order)(order) },
            );
        }
    }
    for _ in 0..300 {
        let order = order_with(&rng.byte_string(63), rng.f64_any(), rng.i32());
        assert_same_output(
            "print_order high bytes",
            || unsafe { (c.print_order)(order) },
            || unsafe { (rs.print_order)(order) },
        );
    }
}

// ============================================================================
// B25–B28 — calculate_inventory_stats
// ============================================================================

/// Runs `calculate_inventory_stats` on an array built inside each library and
/// compares the captured output.
fn compare_inventory_stats(what: &str, items: &[ItemT]) {
    let (c, rs) = apis();
    let ac = ArrayApi::<ItemT>::new(c_lib());
    let ars = ArrayApi::<ItemT>::new(rs_lib());
    unsafe {
        let c_arr = build_item_array(&ac, items);
        let rs_arr = build_item_array(&ars, items);
        assert_same_output(
            what,
            || (c.calculate_inventory_stats)(c_arr),
            || (rs.calculate_inventory_stats)(rs_arr),
        );
        // The same array object handed to the other library must also agree.
        assert_same_output(
            &format!("{what} (cross-library array)"),
            || (c.calculate_inventory_stats)(rs_arr),
            || (rs.calculate_inventory_stats)(c_arr),
        );
        (ac.destroy)(c_arr);
        (ars.destroy)(rs_arr);
    }
}

#[test]
fn inventory_stats_counts_and_random_data() {
    let mut rng = Rng::new(0xB25_0025);
    for count in [1usize, 2, 3, 10, 100] {
        for round in 0..40 {
            let items: Vec<ItemT> = (0..count)
                .map(|i| {
                    item_with(
                        b"Item",
                        b"Cat",
                        rng.price(),
                        i as c_int,
                        (rng.below(200) as c_int) - 50,
                    )
                })
                .collect();
            compare_inventory_stats(
                &format!("inventory_stats count={count} round={round}"),
                &items,
            );
        }
    }
}

#[test]
fn inventory_stats_price_sign_shapes() {
    let mut rng = Rng::new(0xB26_0026);
    for round in 0..60 {
        let count = 1 + rng.below(20);
        // All negative: `max_price` never leaves its 0.0 seed.
        let negatives: Vec<ItemT> = (0..count)
            .map(|i| item_with(b"N", b"C", -(rng.price() + 0.01), i as c_int, 1 + (i as c_int)))
            .collect();
        compare_inventory_stats(&format!("inventory_stats all-negative #{round}"), &negatives);

        // All positive.
        let positives: Vec<ItemT> = (0..count)
            .map(|i| item_with(b"N", b"C", rng.price() + 0.01, i as c_int, 1 + (i as c_int)))
            .collect();
        compare_inventory_stats(&format!("inventory_stats all-positive #{round}"), &positives);

        // Mixed, plus exact zeros.
        let mixed: Vec<ItemT> = (0..count)
            .map(|i| {
                let price = match rng.below(4) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => -rng.price(),
                    _ => rng.price(),
                };
                item_with(b"N", b"C", price, i as c_int, (rng.below(10) as c_int) - 5)
            })
            .collect();
        compare_inventory_stats(&format!("inventory_stats mixed #{round}"), &mixed);
    }
}

/// Rows B27 / ERRORS.md #29 — `total_value / total_items` with a zero divisor.
#[test]
fn stats_division_edge_cases() {
    // quantities cancel to exactly zero, with a non-zero and a zero total value
    let cases: Vec<Vec<ItemT>> = vec![
        vec![item_with(b"a", b"c", 10.0, 1, 0)],
        vec![item_with(b"a", b"c", 0.0, 1, 0)],
        vec![item_with(b"a", b"c", -10.0, 1, 0)],
        vec![
            item_with(b"a", b"c", 5.0, 1, 3),
            item_with(b"b", b"c", 5.0, 2, -3),
        ],
        vec![
            item_with(b"a", b"c", 5.0, 1, 3),
            item_with(b"b", b"c", -5.0, 2, 3),
            item_with(b"c", b"c", 1.0, 3, -6),
        ],
        vec![item_with(b"a", b"c", f64::INFINITY, 1, 0)],
        vec![item_with(b"a", b"c", f64::NAN, 1, 0)],
        vec![item_with(b"a", b"c", 0.0, 1, 0), item_with(b"b", b"c", 0.0, 2, 0)],
    ];
    for (i, items) in cases.iter().enumerate() {
        compare_inventory_stats(&format!("stats division-by-zero case #{i}"), items);
    }
}

/// Rows B27 / ERRORS.md #30 — `int total_items` overflow.
#[test]
fn stats_quantity_overflow() {
    let cases: Vec<Vec<ItemT>> = vec![
        vec![
            item_with(b"a", b"c", 1.0, 1, i32::MAX),
            item_with(b"b", b"c", 1.0, 2, 1),
        ],
        vec![
            item_with(b"a", b"c", 1.0, 1, i32::MAX),
            item_with(b"b", b"c", 1.0, 2, i32::MAX),
        ],
        vec![
            item_with(b"a", b"c", 1.0, 1, i32::MIN),
            item_with(b"b", b"c", 1.0, 2, -1),
        ],
        vec![
            item_with(b"a", b"c", 2.5, 1, i32::MIN),
            item_with(b"b", b"c", 2.5, 2, i32::MIN),
        ],
        vec![
            item_with(b"a", b"c", 1.0, 1, 2_000_000_000),
            item_with(b"b", b"c", 1.0, 2, 2_000_000_000),
            item_with(b"c", b"c", 1.0, 3, 2_000_000_000),
        ],
    ];
    for (i, items) in cases.iter().enumerate() {
        compare_inventory_stats(&format!("stats int-overflow case #{i}"), items);
    }
}

#[test]
fn inventory_stats_special_prices() {
    let mut rng = Rng::new(0xB28_0028);
    for round in 0..80 {
        let count = 1 + rng.below(8);
        let items: Vec<ItemT> = (0..count)
            .map(|i| {
                item_with(
                    b"N",
                    b"C",
                    SPECIAL_DOUBLES[rng.below(SPECIAL_DOUBLES.len())],
                    i as c_int,
                    (rng.below(20) as c_int) - 10,
                )
            })
            .collect();
        compare_inventory_stats(&format!("inventory_stats specials #{round}"), &items);
    }
    // NaN in the first position seeds `min_price` with NaN.
    compare_inventory_stats(
        "inventory_stats NaN first",
        &[
            item_with(b"a", b"c", f64::NAN, 1, 2),
            item_with(b"b", b"c", 5.0, 2, 3),
            item_with(b"c", b"c", -5.0, 3, 4),
        ],
    );
    compare_inventory_stats(
        "inventory_stats inf mix",
        &[
            item_with(b"a", b"c", f64::INFINITY, 1, 2),
            item_with(b"b", b"c", f64::NEG_INFINITY, 2, 3),
            item_with(b"c", b"c", 1.0, 3, 4),
        ],
    );
}

// ============================================================================
// B29–B31 — calculate_order_stats
// ============================================================================

fn compare_order_stats(what: &str, orders: &[OrderT], prepend: bool) {
    let (c, rs) = apis();
    let lc = ListApi::<OrderT>::new(c_lib());
    let lrs = ListApi::<OrderT>::new(rs_lib());
    unsafe {
        let c_list = (lc.create)();
        let rs_list = (lrs.create)();
        for o in orders {
            if prepend {
                assert_eq!((lc.prepend)(c_list, *o), 0);
                assert_eq!((lrs.prepend)(rs_list, *o), 0);
            } else {
                assert_eq!((lc.append)(c_list, *o), 0);
                assert_eq!((lrs.append)(rs_list, *o), 0);
            }
        }
        assert_same_output(
            what,
            || (c.calculate_order_stats)(c_list),
            || (rs.calculate_order_stats)(rs_list),
        );
        assert_same_output(
            &format!("{what} (cross-library list)"),
            || (c.calculate_order_stats)(rs_list),
            || (rs.calculate_order_stats)(c_list),
        );
        (lc.destroy)(c_list);
        (lrs.destroy)(rs_list);
    }
}

#[test]
fn order_stats_counts_and_random_data() {
    let mut rng = Rng::new(0xB29_0029);
    for count in [1usize, 2, 8, 100] {
        for round in 0..30 {
            let orders: Vec<OrderT> = (0..count)
                .map(|i| order_with(b"Customer", rng.price(), 1000 + i as c_int))
                .collect();
            compare_order_stats(
                &format!("order_stats count={count} round={round} append"),
                &orders,
                false,
            );
            compare_order_stats(
                &format!("order_stats count={count} round={round} prepend"),
                &orders,
                true,
            );
        }
    }
}

/// Row B30 — the `min_order < 0` quirk: with negative amounts the guard stays
/// true, so the printed "smallest" order is the *last* one visited.
#[test]
fn order_stats_negative_and_zero_amounts() {
    let mut rng = Rng::new(0xB30_0030);
    for round in 0..60 {
        let count = 1 + rng.below(10);
        let negatives: Vec<OrderT> = (0..count)
            .map(|i| order_with(b"Neg", -(rng.price() + 0.01), i as c_int))
            .collect();
        compare_order_stats(&format!("order_stats all-negative #{round}"), &negatives, false);
        compare_order_stats(
            &format!("order_stats all-negative prepend #{round}"),
            &negatives,
            true,
        );

        let zeros: Vec<OrderT> = (0..count)
            .map(|i| order_with(b"Zero", if i % 2 == 0 { 0.0 } else { -0.0 }, i as c_int))
            .collect();
        compare_order_stats(&format!("order_stats zeros #{round}"), &zeros, false);

        let mixed: Vec<OrderT> = (0..count)
            .map(|i| {
                let amount = match rng.below(4) {
                    0 => 0.0,
                    1 => -0.0,
                    2 => -rng.price(),
                    _ => rng.price(),
                };
                order_with(b"Mix", amount, i as c_int)
            })
            .collect();
        compare_order_stats(&format!("order_stats mixed #{round}"), &mixed, false);
    }
}

#[test]
fn order_stats_special_amounts() {
    let mut rng = Rng::new(0xB31_0031);
    for round in 0..80 {
        let count = 1 + rng.below(6);
        let orders: Vec<OrderT> = (0..count)
            .map(|i| {
                order_with(
                    b"Special",
                    SPECIAL_DOUBLES[rng.below(SPECIAL_DOUBLES.len())],
                    i as c_int,
                )
            })
            .collect();
        compare_order_stats(&format!("order_stats specials #{round}"), &orders, false);
    }
}

// ============================================================================
// B32–B34 — find_items_by_category
// ============================================================================

const CATEGORIES: [&[u8]; 6] = [
    b"Electronics",
    b"Furniture",
    b"Office",
    b"",
    b"Elec",
    b"ElectronicsExtraLongCategoryName",
];

fn compare_find_by_category(what: &str, items: &[ItemT], category: &[u8]) {
    let (c, rs) = apis();
    let ac = ArrayApi::<ItemT>::new(c_lib());
    let ars = ArrayApi::<ItemT>::new(rs_lib());
    let cat = cstring(category);
    unsafe {
        let c_arr = build_item_array(&ac, items);
        let rs_arr = build_item_array(&ars, items);
        assert_same_output(
            what,
            || (c.find_items_by_category)(c_arr, cat.as_ptr() as *const c_char),
            || (rs.find_items_by_category)(rs_arr, cat.as_ptr() as *const c_char),
        );
        (ac.destroy)(c_arr);
        (ars.destroy)(rs_arr);
    }
}

#[test]
fn find_by_category_match_counts() {
    let mut rng = Rng::new(0xB32_0032);
    for round in 0..80 {
        let count = rng.below(51);
        let items: Vec<ItemT> = (0..count)
            .map(|i| {
                item_with(
                    b"Name",
                    CATEGORIES[rng.below(CATEGORIES.len())],
                    rng.price(),
                    i as c_int,
                    rng.i32(),
                )
            })
            .collect();
        for category in CATEGORIES {
            compare_find_by_category(
                &format!(
                    "find_by_category round={round} count={count} cat={:?}",
                    String::from_utf8_lossy(category)
                ),
                &items,
                category,
            );
        }
        // all-match and no-match extremes
        let all: Vec<ItemT> = (0..count.max(1))
            .map(|i| item_with(b"Name", b"Same", 1.0, i as c_int, 1))
            .collect();
        compare_find_by_category(&format!("find_by_category all-match #{round}"), &all, b"Same");
        compare_find_by_category(&format!("find_by_category no-match #{round}"), &all, b"Other");
    }
}

#[test]
fn find_by_category_string_shapes() {
    let mut rng = Rng::new(0xB33_0033);
    let long31 = vec![b'L'; 31];
    let long32 = vec![b'L'; 32];
    let long40 = vec![b'L'; 40];
    let shapes: Vec<Vec<u8>> = vec![
        vec![],
        vec![b'x'],
        long31.clone(),
        long32.clone(),
        long40.clone(),
        b"Electronics".to_vec(),
        b"electronics".to_vec(), // case-sensitive strcmp
        b"Electronic".to_vec(),  // prefix
        b"Electronics ".to_vec(),
        vec![0x80, 0xff, 0x41],
    ];
    for shape in &shapes {
        let items: Vec<ItemT> = shapes
            .iter()
            .enumerate()
            .map(|(i, s)| item_with(b"Name", s, 1.0 + i as f64, i as c_int, i as c_int))
            .collect();
        compare_find_by_category(
            &format!("find_by_category shape {:?}", String::from_utf8_lossy(shape)),
            &items,
            shape,
        );
    }
    // Random high-byte categories.
    for _ in 0..200 {
        let items: Vec<ItemT> = (0..1 + rng.below(6))
            .map(|i| item_with(b"N", &rng.byte_string(35), rng.price(), i as c_int, 1))
            .collect();
        let query = rng.byte_string(35);
        compare_find_by_category("find_by_category random bytes", &items, &query);
    }
}

/// Row B34 — a non-NULL but empty array still prints the header and the
/// "no items" line (unlike the NULL case, which prints nothing at all).
#[test]
fn find_by_category_empty_array() {
    compare_find_by_category("find_by_category empty array", &[], b"Electronics");
    compare_find_by_category("find_by_category empty array, empty cat", &[], b"");
}

// ============================================================================
// B35 — find_expensive_items
// ============================================================================

fn compare_find_expensive(what: &str, items: &[ItemT], min_price: f64) {
    let (c, rs) = apis();
    let lc = ListApi::<ItemT>::new(c_lib());
    let lrs = ListApi::<ItemT>::new(rs_lib());
    unsafe {
        let c_list = build_item_list(&lc, items);
        let rs_list = build_item_list(&lrs, items);
        assert_same_output(
            what,
            || (c.find_expensive_items)(c_list, min_price),
            || (rs.find_expensive_items)(rs_list, min_price),
        );
        assert_same_output(
            &format!("{what} (cross-library list)"),
            || (c.find_expensive_items)(rs_list, min_price),
            || (rs.find_expensive_items)(c_list, min_price),
        );
        (lc.destroy)(c_list);
        (lrs.destroy)(rs_list);
    }
}

#[test]
fn find_expensive_thresholds() {
    let mut rng = Rng::new(0xB35_0035);
    let thresholds: Vec<f64> = {
        let mut t = vec![
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NAN,
            -f64::NAN,
            0.0,
            -0.0,
            -1.0,
            0.005,
            100.0,
            1e308,
            5e-324,
        ];
        for _ in 0..10 {
            t.push(rng.price());
        }
        t
    };
    for round in 0..25 {
        let count = rng.below(51);
        let items: Vec<ItemT> = (0..count)
            .map(|i| {
                let price = match rng.below(6) {
                    0 => SPECIAL_DOUBLES[rng.below(SPECIAL_DOUBLES.len())],
                    1 => -rng.price(),
                    2 => 0.0,
                    _ => rng.price(),
                };
                item_with(b"Item", b"Cat", price, i as c_int, rng.small_i32())
            })
            .collect();
        for &min_price in &thresholds {
            compare_find_expensive(
                &format!("find_expensive round={round} count={count} min={min_price:?}"),
                &items,
                min_price,
            );
        }
    }
    // Empty list: header plus the "nothing found" line.
    compare_find_expensive("find_expensive empty list", &[], 10.0);
}

// ============================================================================
// Exhaustive NaN / infinity coverage for the accumulator paths
//
// `total_value += price * quantity` and `total_revenue += amount` are the only
// places where two NaNs can meet. Which NaN survives depends on the operand
// order the compiler picks, so every ordered combination is checked rather than
// sampled (this is the class of bug the randomized rows found at round 24 of 80).
// ============================================================================

const ACC_SPECIALS: [f64; 8] = [
    0.0,
    -0.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
    -f64::NAN,
    1.5,
    -1.5,
];

#[test]
fn inventory_stats_nan_inf_exhaustive() {
    // All ordered pairs and triples of special prices, with quantities that make
    // the product NaN (`inf * 0`), keep it, or cancel the divisor to zero.
    for (qi, quantities) in [[1i32, 1, 1], [0, 1, -1], [2, -2, 0], [0, 0, 0]]
        .iter()
        .enumerate()
    {
        for a in ACC_SPECIALS {
            for b in ACC_SPECIALS {
                compare_inventory_stats(
                    &format!("stats pair q{qi} {a:?} {b:?}"),
                    &[
                        item_with(b"a", b"c", a, 1, quantities[0]),
                        item_with(b"b", b"c", b, 2, quantities[1]),
                    ],
                );
                for c in [f64::NAN, -f64::NAN, f64::INFINITY, 2.0] {
                    compare_inventory_stats(
                        &format!("stats triple q{qi} {a:?} {b:?} {c:?}"),
                        &[
                            item_with(b"a", b"c", a, 1, quantities[0]),
                            item_with(b"b", b"c", b, 2, quantities[1]),
                            item_with(b"c", b"c", c, 3, quantities[2]),
                        ],
                    );
                }
            }
        }
    }
}

#[test]
fn order_stats_nan_inf_exhaustive() {
    for a in ACC_SPECIALS {
        for b in ACC_SPECIALS {
            for prepend in [false, true] {
                compare_order_stats(
                    &format!("order pair {a:?} {b:?} prepend={prepend}"),
                    &[order_with(b"a", a, 1), order_with(b"b", b, 2)],
                    prepend,
                );
            }
            for c in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 3.5] {
                compare_order_stats(
                    &format!("order triple {a:?} {b:?} {c:?}"),
                    &[
                        order_with(b"a", a, 1),
                        order_with(b"b", b, 2),
                        order_with(b"c", c, 3),
                    ],
                    false,
                );
            }
        }
    }
}

/// Longer sums where NaNs appear at every position, so no single operand order
/// can accidentally look right.
#[test]
fn stats_nan_positions_in_long_sums() {
    let mut rng = Rng::new(0xACC_1234);
    for round in 0..200 {
        let count = 2 + rng.below(8);
        let nan_at = rng.below(count);
        let items: Vec<ItemT> = (0..count)
            .map(|i| {
                let price = if i == nan_at {
                    if rng.below(2) == 0 {
                        f64::NAN
                    } else {
                        -f64::NAN
                    }
                } else {
                    match rng.below(5) {
                        0 => f64::INFINITY,
                        1 => f64::NEG_INFINITY,
                        2 => f64::NAN,
                        3 => -f64::NAN,
                        _ => rng.price(),
                    }
                };
                item_with(b"n", b"c", price, i as c_int, (rng.below(7) as c_int) - 3)
            })
            .collect();
        compare_inventory_stats(&format!("long NaN sum #{round}"), &items);

        let orders: Vec<OrderT> = items
            .iter()
            .map(|it| order_with(b"n", it.price, it.id))
            .collect();
        compare_order_stats(&format!("long NaN order sum #{round}"), &orders, round % 2 == 0);
    }
}

// ============================================================================
// `%.2f` formatting stress: tens of thousands of random bit patterns, batched
// into a few captures so it stays fast. glibc and Rust must agree on every
// finite value (exact conversion, ties to even) and on inf/nan spelling.
// ============================================================================

#[test]
fn print_item_formatting_stress() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xF0F_2222);
    for batch in 0..40 {
        let items: Vec<ItemT> = (0..500)
            .map(|_| {
                let price = match rng.below(4) {
                    0 => f64::from_bits(rng.next_u64()),
                    1 => SPECIAL_DOUBLES[rng.below(SPECIAL_DOUBLES.len())],
                    2 => f64::from_bits(rng.next_u64() & 0x000f_ffff_ffff_ffff), // subnormals
                    _ => rng.f64_any(),
                };
                item_with(b"Stress", b"Cat", price, rng.i32(), rng.i32())
            })
            .collect();
        assert_same_output(
            &format!("print_item formatting batch #{batch}"),
            || unsafe {
                for it in &items {
                    (c.print_item)(*it);
                }
            },
            || unsafe {
                for it in &items {
                    (rs.print_item)(*it);
                }
            },
        );
    }
}

#[test]
fn print_order_formatting_stress() {
    let (c, rs) = apis();
    let mut rng = Rng::new(0xF0F_3333);
    for batch in 0..20 {
        let orders: Vec<OrderT> = (0..500)
            .map(|_| {
                let total = if rng.below(2) == 0 {
                    f64::from_bits(rng.next_u64())
                } else {
                    rng.f64_any()
                };
                order_with(b"Stress", total, rng.i32())
            })
            .collect();
        assert_same_output(
            &format!("print_order formatting batch #{batch}"),
            || unsafe {
                for o in &orders {
                    (c.print_order)(*o);
                }
            },
            || unsafe {
                for o in &orders {
                    (rs.print_order)(*o);
                }
            },
        );
    }
}

/// The `min_price` argument of `find_expensive_items` is the other `%.2f` an
/// external caller controls directly; an empty list keeps the output to the
/// header plus the "nothing found" line.
#[test]
fn find_expensive_header_formatting_stress() {
    let (c, rs) = apis();
    let lc = ListApi::<ItemT>::new(c_lib());
    let lrs = ListApi::<ItemT>::new(rs_lib());
    let mut rng = Rng::new(0xF0F_4444);
    unsafe {
        let cl = (lc.create)();
        let rl = (lrs.create)();
        for batch in 0..20 {
            let thresholds: Vec<f64> = (0..500)
                .map(|_| {
                    if rng.below(2) == 0 {
                        f64::from_bits(rng.next_u64())
                    } else {
                        rng.f64_any()
                    }
                })
                .collect();
            assert_same_output(
                &format!("find_expensive header batch #{batch}"),
                || {
                    for t in &thresholds {
                        (c.find_expensive_items)(cl, *t);
                    }
                },
                || {
                    for t in &thresholds {
                        (rs.find_expensive_items)(rl, *t);
                    }
                },
            );
        }
        (lc.destroy)(cl);
        (lrs.destroy)(rl);
    }
}
