//! Phase B — differential tests for the `main.c` display/demo entry points
//! (rows B36–B42 of `CONFIGS.md`): `print_menu` and the five `demo_*`
//! functions, driven through the exported symbols of both shared libraries.

mod common;

use common::*;

fn apis() -> (DemoApi, DemoApi) {
    (DemoApi::new(c_lib()), DemoApi::new(rs_lib()))
}

/// B36 — `print_menu`, byte for byte (also called repeatedly).
#[test]
fn b36_print_menu() {
    let (c, rs) = apis();
    for i in 0..5 {
        assert_same_output(
            &format!("print_menu call #{i}"),
            || unsafe { (c.print_menu)() },
            || unsafe { (rs.print_menu)() },
        );
    }
}

/// B37 — `demo_integer_containers` (array capacity 10 + 5 pushes, list of 5,
/// `%d` / `%.2f` / `%lld`).
#[test]
fn b37_demo_integer_containers() {
    let (c, rs) = apis();
    for i in 0..3 {
        assert_same_output(
            &format!("demo_integer_containers call #{i}"),
            || unsafe { (c.demo_integer_containers)() },
            || unsafe { (rs.demo_integer_containers)() },
        );
    }
}

/// B38 — `demo_double_containers`: capacity 5 with 7 pushes, so the `realloc`
/// growth path runs; prints `%.1f°C` (multi-byte literal) and `%.2f`.
#[test]
fn b38_demo_double_containers() {
    let (c, rs) = apis();
    for i in 0..3 {
        assert_same_output(
            &format!("demo_double_containers call #{i}"),
            || unsafe { (c.demo_double_containers)() },
            || unsafe { (rs.demo_double_containers)() },
        );
    }
}

/// B39 — `demo_inventory_array`: 10 items, `print_item` loop, stats, two
/// category searches, low-stock filter.
#[test]
fn b39_demo_inventory_array() {
    let (c, rs) = apis();
    for i in 0..3 {
        assert_same_output(
            &format!("demo_inventory_array call #{i}"),
            || unsafe { (c.demo_inventory_array)() },
            || unsafe { (rs.demo_inventory_array)() },
        );
    }
}

/// B40 — `demo_order_list`: 8 appended orders, stats, large-order filter.
#[test]
fn b40_demo_order_list() {
    let (c, rs) = apis();
    for i in 0..3 {
        assert_same_output(
            &format!("demo_order_list call #{i}"),
            || unsafe { (c.demo_order_list)() },
            || unsafe { (rs.demo_order_list)() },
        );
    }
}

/// B41 — `demo_mixed_operations`: the same 5 items in an array *and* a list.
#[test]
fn b41_demo_mixed_operations() {
    let (c, rs) = apis();
    for i in 0..3 {
        assert_same_output(
            &format!("demo_mixed_operations call #{i}"),
            || unsafe { (c.demo_mixed_operations)() },
            || unsafe { (rs.demo_mixed_operations)() },
        );
    }
}

/// B42 — all six functions called in sequence in one process, the way menu
/// choice 6 does, to catch state that leaks between calls.
#[test]
fn b42_all_demos_in_sequence() {
    let (c, rs) = apis();
    assert_same_output(
        "all demos in sequence",
        || unsafe {
            (c.print_menu)();
            (c.demo_integer_containers)();
            (c.demo_double_containers)();
            (c.demo_inventory_array)();
            (c.demo_order_list)();
            (c.demo_mixed_operations)();
            (c.print_menu)();
        },
        || unsafe {
            (rs.print_menu)();
            (rs.demo_integer_containers)();
            (rs.demo_double_containers)();
            (rs.demo_inventory_array)();
            (rs.demo_order_list)();
            (rs.demo_mixed_operations)();
            (rs.print_menu)();
        },
    );

    // Interleaving the two libraries must not change either one's output.
    let mut rng = Rng::new(0xB42_0042);
    for round in 0..10 {
        let order: Vec<usize> = (0..6).map(|_| rng.below(6)).collect();
        let run = |api: &DemoApi| {
            for &which in &order {
                unsafe {
                    match which {
                        0 => (api.print_menu)(),
                        1 => (api.demo_integer_containers)(),
                        2 => (api.demo_double_containers)(),
                        3 => (api.demo_inventory_array)(),
                        4 => (api.demo_order_list)(),
                        _ => (api.demo_mixed_operations)(),
                    }
                }
            }
        };
        assert_same_output(
            &format!("random demo sequence #{round} {order:?}"),
            || run(&c),
            || run(&rs),
        );
    }
}
