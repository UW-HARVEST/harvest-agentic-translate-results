//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every rejection path in the C code is constructed exactly, called in both
//! libraries through their exported symbols, and the *same* sentinel
//! (`NULL` / `-1` / `0` / the exact message / the same fatal signal) is asserted.

mod common;

use std::mem::size_of;

use common::*;
use libc::{c_char, c_int, c_void};

macro_rules! all_types {
    ($f:ident) => {{
        $f::<c_int>();
        $f::<f64>();
        $f::<ItemT>();
        $f::<OrderT>();
    }};
}

fn inventory_apis() -> (InventoryApi, InventoryApi) {
    (InventoryApi::new(c_lib()), InventoryApi::new(rs_lib()))
}

// ============================================================================
// Row 2 — array_TYPE_create: the caller-controlled malloc fails
// ============================================================================

fn huge_capacity<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut saw_null = false;
    unsafe {
        for cap in [
            usize::MAX,
            usize::MAX / 2,
            usize::MAX / 8,
            1usize << 50,
            1usize << 62,
        ] {
            let c = (p.c.create)(cap);
            let rs = (p.rs.create)(cap);
            assert_eq!(
                c.is_null(),
                rs.is_null(),
                "array_{}_create({cap}): C returned {}, Rust returned {}",
                T::SUFFIX,
                if c.is_null() { "NULL" } else { "non-NULL" },
                if rs.is_null() { "NULL" } else { "non-NULL" },
            );
            if c.is_null() {
                saw_null = true;
                // A NULL result must also be accepted by every other entry point.
                assert_eq!((p.c.size)(c), (p.rs.size)(rs));
            } else {
                assert_eq!((*c).capacity, (*rs).capacity);
                assert_eq!((*c).size, (*rs).size);
                (p.c.destroy)(c);
                (p.rs.destroy)(rs);
            }
        }
    }
    assert!(
        saw_null,
        "expected at least one allocation failure for array_{}_create",
        T::SUFFIX
    );
}

#[test]
fn create_huge_capacity_returns_null() {
    all_types!(huge_capacity);
}

// ============================================================================
// Row 3 — array_TYPE_push(NULL, v) == -1
// ============================================================================

fn push_null<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0xDEAD_0003);
    unsafe {
        for _ in 0..20 {
            let v = T::rand(&mut rng);
            let c = (p.c.push)(std::ptr::null_mut(), v);
            let rs = (p.rs.push)(std::ptr::null_mut(), v);
            assert_eq!(c, -1, "C array_{}_push(NULL) must return -1", T::SUFFIX);
            assert_eq!(rs, c, "array_{}_push(NULL) return differs", T::SUFFIX);
        }
    }
}

#[test]
fn push_null_array_returns_minus_one() {
    all_types!(push_null);
}

// ============================================================================
// Row 4 — array_TYPE_push: realloc failure on the grow path
// ============================================================================

fn push_realloc_failure<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0xDEAD_0004);

    // `size >= capacity` forces the grow path; `capacity * 2 * sizeof(T)` is then
    // far beyond any address space, so `realloc` must fail. Both values also
    // exercise the size_t wrap-around that C performs without trapping.
    for saturated in [usize::MAX, usize::MAX / 4] {
        unsafe {
            let value = T::rand(&mut rng);
            let mut results = Vec::new();
            for which in 0..2 {
                let hdr = libc::malloc(size_of::<ArrayRaw<T>>()) as *mut ArrayRaw<T>;
                assert!(!hdr.is_null());
                let data = libc::malloc(size_of::<T>()) as *mut T;
                assert!(!data.is_null());
                (*hdr).data = data;
                (*hdr).size = saturated;
                (*hdr).capacity = saturated;

                let ret = if which == 0 {
                    (p.c.push)(hdr, value)
                } else {
                    (p.rs.push)(hdr, value)
                };
                results.push((ret, (*hdr).size, (*hdr).capacity, (*hdr).data == data));
                libc_free(data);
                libc_free(hdr);
            }
            assert_eq!(
                results[0].0, -1,
                "C array_{}_push must return -1 when realloc fails (capacity {saturated})",
                T::SUFFIX
            );
            assert_eq!(
                results[0], results[1],
                "array_{}_push realloc-failure behaviour differs (capacity {saturated}): \
                 C {:?} vs Rust {:?}",
                T::SUFFIX, results[0], results[1]
            );
            assert!(results[0].3, "the data pointer must be left untouched");
        }
    }
}

#[test]
fn push_realloc_failure_returns_minus_one() {
    all_types!(push_realloc_failure);
}

// ============================================================================
// Row 5 — array_TYPE_get past `size` (no bounds check)
// ============================================================================

fn get_past_size<T: Elem>() {
    let p = ArrayPair::<T>::new();
    let mut rng = Rng::new(0xDEAD_0005);
    unsafe {
        for _ in 0..30 {
            let count = 1 + rng.below(20);
            let values: Vec<T> = (0..count).map(|_| T::rand(&mut rng)).collect();
            let c = (p.c.create)(count);
            let rs = (p.rs.create)(count);
            for v in &values {
                (p.c.push)(c, *v);
                (p.rs.push)(rs, *v);
            }
            // After `clear`, indices 0..count are >= size but still readable, and
            // the C code happily returns the retained slots.
            (p.c.clear)(c);
            (p.rs.clear)(rs);
            assert_eq!((p.c.size)(c), 0);
            assert_eq!((p.rs.size)(rs), 0);
            let mut c_got = Vec::new();
            let mut rs_got = Vec::new();
            for i in 0..count {
                c_got.push((p.c.get)(c, i));
                rs_got.push((p.rs.get)(rs, i));
            }
            assert_elems_eq("get() past size after clear()", &c_got, &rs_got);
            assert_elems_eq("get() past size returns the retained slot", &values, &c_got);
            (p.c.destroy)(c);
            (p.rs.destroy)(rs);
        }
    }
}

#[test]
fn get_index_past_size_reads_slot() {
    all_types!(get_past_size);
}

// ============================================================================
// Rows 6, 26, 27 — unchecked NULL dereferences must die the same way
// ============================================================================

/// Runs `f` in a forked child and reports `(signalled, signal, exit code)`.
fn child_outcome<F: FnOnce()>(f: F) -> (bool, c_int, c_int) {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Silence the child: a crash report would pollute the test output.
            // Set DIFF_SHOW_CHILD_OUTPUT=1 to inspect it while debugging.
            if std::env::var_os("DIFF_SHOW_CHILD_OUTPUT").is_none() {
                let devnull = libc::open(b"/dev/null\0".as_ptr() as *const c_char, libc::O_WRONLY);
                if devnull >= 0 {
                    libc::dup2(devnull, 1);
                    libc::dup2(devnull, 2);
                }
            }
            f();
            libc::_exit(0);
        }
        let mut status: c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid);
        let signalled = libc::WIFSIGNALED(status);
        let signal = if signalled { libc::WTERMSIG(status) } else { 0 };
        let code = if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else {
            -1
        };
        (signalled, signal, code)
    }
}

fn assert_same_crash(what: &str, c_call: impl FnOnce(), rs_call: impl FnOnce()) {
    let c = child_outcome(c_call);
    let rs = child_outcome(rs_call);
    assert!(
        c.0,
        "{what}: the C implementation was expected to die from a signal, got exit code {}",
        c.2
    );
    assert_eq!(
        c, rs,
        "{what}: C died with (signalled, signal, code) {c:?} but Rust with {rs:?}"
    );
}

#[test]
fn ub_null_deref_matches() {
    let (c, rs) = inventory_apis();
    let ints = ArrayPair::<c_int>::new();
    let items = ArrayPair::<ItemT>::new();
    let name = cstring(b"Name");

    // Row 6 — array_TYPE_get(NULL, 0): no NULL check in C.
    assert_same_crash(
        "array_int_get(NULL, 0)",
        || unsafe {
            let _ = (ints.c.get)(std::ptr::null_mut(), 0);
        },
        || unsafe {
            let _ = (ints.rs.get)(std::ptr::null_mut(), 0);
        },
    );
    assert_same_crash(
        "array_item_t_get(NULL, 3)",
        || unsafe {
            let _ = (items.c.get)(std::ptr::null_mut(), 3);
        },
        || unsafe {
            let _ = (items.rs.get)(std::ptr::null_mut(), 3);
        },
    );

    // Row 26 — create_item with a NULL string: strncpy(NULL).
    assert_same_crash(
        "create_item(NULL name)",
        || unsafe {
            let _ = (c.create_item)(1, std::ptr::null(), name.as_ptr() as *const c_char, 1.0, 1);
        },
        || unsafe {
            let _ = (rs.create_item)(1, std::ptr::null(), name.as_ptr() as *const c_char, 1.0, 1);
        },
    );
    assert_same_crash(
        "create_item(NULL category)",
        || unsafe {
            let _ = (c.create_item)(1, name.as_ptr() as *const c_char, std::ptr::null(), 1.0, 1);
        },
        || unsafe {
            let _ = (rs.create_item)(1, name.as_ptr() as *const c_char, std::ptr::null(), 1.0, 1);
        },
    );

    // Row 27 — create_order with a NULL name.
    assert_same_crash(
        "create_order(NULL name)",
        || unsafe {
            let _ = (c.create_order)(1, std::ptr::null(), 1.0);
        },
        || unsafe {
            let _ = (rs.create_order)(1, std::ptr::null(), 1.0);
        },
    );
}

// ============================================================================
// Rows 7, 15 — *_size(NULL) == 0
// ============================================================================

fn size_null<T: Elem>() {
    let a = ArrayPair::<T>::new();
    let l = ListPair::<T>::new();
    unsafe {
        assert_eq!((a.c.size)(std::ptr::null_mut()), 0);
        assert_eq!((a.rs.size)(std::ptr::null_mut()), 0);
        assert_eq!((l.c.size)(std::ptr::null_mut()), 0);
        assert_eq!((l.rs.size)(std::ptr::null_mut()), 0);
    }
}

#[test]
fn size_null_returns_zero() {
    all_types!(size_null);
}

// ============================================================================
// Rows 8, 9, 16 — *_clear(NULL) / *_destroy(NULL) are no-ops
// ============================================================================

fn clear_destroy_null<T: Elem>() {
    let a = ArrayPair::<T>::new();
    let l = ListPair::<T>::new();
    let mut rng = Rng::new(0xDEAD_0008);
    unsafe {
        // Must not crash, and must not disturb anything else.
        (a.c.clear)(std::ptr::null_mut());
        (a.rs.clear)(std::ptr::null_mut());
        (a.c.destroy)(std::ptr::null_mut());
        (a.rs.destroy)(std::ptr::null_mut());
        (l.c.clear)(std::ptr::null_mut());
        (l.rs.clear)(std::ptr::null_mut());
        (l.c.destroy)(std::ptr::null_mut());
        (l.rs.destroy)(std::ptr::null_mut());

        // Both libraries still work afterwards.
        let v = T::rand(&mut rng);
        let ca = (a.c.create)(2);
        let ra = (a.rs.create)(2);
        assert_eq!((a.c.push)(ca, v), (a.rs.push)(ra, v));
        assert_eq!((a.c.size)(ca), (a.rs.size)(ra));
        (a.c.destroy)(ca);
        (a.rs.destroy)(ra);
        let cl = (l.c.create)();
        let rl = (l.rs.create)();
        assert_eq!((l.c.append)(cl, v), (l.rs.append)(rl, v));
        assert_eq!((l.c.size)(cl), (l.rs.size)(rl));
        (l.c.destroy)(cl);
        (l.rs.destroy)(rl);
    }
}

#[test]
fn clear_destroy_null_are_noops() {
    all_types!(clear_destroy_null);
}

// ============================================================================
// Rows 11, 13 — list_TYPE_append/prepend(NULL, v) == -1
// ============================================================================

fn list_null_insert<T: Elem>() {
    let l = ListPair::<T>::new();
    let mut rng = Rng::new(0xDEAD_0011);
    unsafe {
        for _ in 0..20 {
            let v = T::rand(&mut rng);
            let ca = (l.c.append)(std::ptr::null_mut(), v);
            let ra = (l.rs.append)(std::ptr::null_mut(), v);
            let cp = (l.c.prepend)(std::ptr::null_mut(), v);
            let rp = (l.rs.prepend)(std::ptr::null_mut(), v);
            assert_eq!(ca, -1, "C list_{}_append(NULL) must return -1", T::SUFFIX);
            assert_eq!(cp, -1, "C list_{}_prepend(NULL) must return -1", T::SUFFIX);
            assert_eq!(ca, ra, "list_{}_append(NULL) differs", T::SUFFIX);
            assert_eq!(cp, rp, "list_{}_prepend(NULL) differs", T::SUFFIX);
        }
    }
}

#[test]
fn list_null_append_prepend_return_minus_one() {
    all_types!(list_null_insert);
}

// ============================================================================
// Rows 17, 18 — calculate_inventory_stats: NULL and empty
// ============================================================================

#[test]
fn inventory_stats_null_and_empty() {
    let (c, rs) = inventory_apis();
    let a = ArrayPair::<ItemT>::new();

    let c_out = capture(|| unsafe { (c.calculate_inventory_stats)(std::ptr::null_mut()) });
    let rs_out = capture(|| unsafe { (rs.calculate_inventory_stats)(std::ptr::null_mut()) });
    assert_eq!(c_out, b"No items in inventory\n".to_vec());
    assert_eq!(c_out, rs_out, "NULL case output differs");

    unsafe {
        for cap in [0usize, 1, 16] {
            let ca = (a.c.create)(cap);
            let ra = (a.rs.create)(cap);
            let c_out = capture(|| (c.calculate_inventory_stats)(ca));
            let rs_out = capture(|| (rs.calculate_inventory_stats)(ra));
            assert_eq!(c_out, b"No items in inventory\n".to_vec());
            assert_eq!(c_out, rs_out, "empty-array case output differs");
            // ... and after clearing a populated array, too.
            let mut rng = Rng::new(0xDEAD_0018);
            for _ in 0..5 {
                (a.c.push)(ca, ItemT::rand(&mut rng));
            }
            let mut rng = Rng::new(0xDEAD_0018);
            for _ in 0..5 {
                (a.rs.push)(ra, ItemT::rand(&mut rng));
            }
            (a.c.clear)(ca);
            (a.rs.clear)(ra);
            let c_out = capture(|| (c.calculate_inventory_stats)(ca));
            let rs_out = capture(|| (rs.calculate_inventory_stats)(ra));
            assert_eq!(c_out, b"No items in inventory\n".to_vec());
            assert_eq!(c_out, rs_out, "cleared-array case output differs");
            (a.c.destroy)(ca);
            (a.rs.destroy)(ra);
        }
    }
}

// ============================================================================
// Rows 19, 20 — calculate_order_stats: NULL and empty
// ============================================================================

#[test]
fn order_stats_null_and_empty() {
    let (c, rs) = inventory_apis();
    let l = ListPair::<OrderT>::new();

    let c_out = capture(|| unsafe { (c.calculate_order_stats)(std::ptr::null_mut()) });
    let rs_out = capture(|| unsafe { (rs.calculate_order_stats)(std::ptr::null_mut()) });
    assert_eq!(c_out, b"No orders to analyze\n".to_vec());
    assert_eq!(c_out, rs_out, "NULL case output differs");

    unsafe {
        let cl = (l.c.create)();
        let rl = (l.rs.create)();
        let c_out = capture(|| (c.calculate_order_stats)(cl));
        let rs_out = capture(|| (rs.calculate_order_stats)(rl));
        assert_eq!(c_out, b"No orders to analyze\n".to_vec());
        assert_eq!(c_out, rs_out, "empty-list case output differs");

        let mut rng = Rng::new(0xDEAD_0020);
        for _ in 0..4 {
            let o = OrderT::rand(&mut rng);
            (l.c.append)(cl, o);
            (l.rs.append)(rl, o);
        }
        (l.c.clear)(cl);
        (l.rs.clear)(rl);
        let c_out = capture(|| (c.calculate_order_stats)(cl));
        let rs_out = capture(|| (rs.calculate_order_stats)(rl));
        assert_eq!(c_out, b"No orders to analyze\n".to_vec());
        assert_eq!(c_out, rs_out, "cleared-list case output differs");
        (l.c.destroy)(cl);
        (l.rs.destroy)(rl);
    }
}

// ============================================================================
// Rows 21, 22 — find_items_by_category with a NULL argument prints nothing
// ============================================================================

#[test]
fn find_by_category_null_args_silent() {
    let (c, rs) = inventory_apis();
    let a = ArrayPair::<ItemT>::new();
    let cat = cstring(b"Electronics");
    let mut rng = Rng::new(0xDEAD_0021);

    unsafe {
        // items == NULL
        let c_out = capture(|| {
            (c.find_items_by_category)(std::ptr::null_mut(), cat.as_ptr() as *const c_char)
        });
        let rs_out = capture(|| {
            (rs.find_items_by_category)(std::ptr::null_mut(), cat.as_ptr() as *const c_char)
        });
        assert!(c_out.is_empty(), "C printed {:?}", String::from_utf8_lossy(&c_out));
        assert_eq!(c_out, rs_out);

        // category == NULL, on both an empty and a populated array
        let ca = (a.c.create)(4);
        let ra = (a.rs.create)(4);
        for _ in 0..3 {
            let it = ItemT::rand(&mut rng);
            (a.c.push)(ca, it);
            (a.rs.push)(ra, it);
        }
        for (label, arr_c, arr_rs) in [
            ("populated", ca, ra),
            ("null-both", std::ptr::null_mut(), std::ptr::null_mut()),
        ] {
            let c_out = capture(|| (c.find_items_by_category)(arr_c, std::ptr::null()));
            let rs_out = capture(|| (rs.find_items_by_category)(arr_rs, std::ptr::null()));
            assert!(
                c_out.is_empty(),
                "{label}: C printed {:?}",
                String::from_utf8_lossy(&c_out)
            );
            assert_eq!(c_out, rs_out, "{label}: NULL category output differs");
        }
        (a.c.destroy)(ca);
        (a.rs.destroy)(ra);
    }
}

// ============================================================================
// Row 23 — find_items_by_category with no match
// ============================================================================

#[test]
fn find_by_category_no_match_message() {
    let (c, rs) = inventory_apis();
    let a = ArrayPair::<ItemT>::new();
    let cat = cstring(b"NoSuchCategory");
    let mut rng = Rng::new(0xDEAD_0023);
    unsafe {
        let ca = (a.c.create)(4);
        let ra = (a.rs.create)(4);
        for _ in 0..5 {
            let mut it = ItemT::rand(&mut rng);
            it.category = [0u8; MAX_CATEGORY_LENGTH];
            it.category[..3].copy_from_slice(b"Cat");
            (a.c.push)(ca, it);
            (a.rs.push)(ra, it);
        }
        let c_out = capture(|| (c.find_items_by_category)(ca, cat.as_ptr() as *const c_char));
        let rs_out = capture(|| (rs.find_items_by_category)(ra, cat.as_ptr() as *const c_char));
        assert_eq!(
            c_out,
            b"\n=== Items in category 'NoSuchCategory' ===\nNo items found in this category\n"
                .to_vec()
        );
        assert_eq!(c_out, rs_out);
        (a.c.destroy)(ca);
        (a.rs.destroy)(ra);
    }
}

// ============================================================================
// Rows 24, 25 — find_expensive_items: NULL list, and no match
// ============================================================================

#[test]
fn find_expensive_null_silent() {
    let (c, rs) = inventory_apis();
    for min_price in [0.0f64, -1.0, f64::NAN, f64::INFINITY] {
        let c_out = capture(|| unsafe {
            (c.find_expensive_items)(std::ptr::null_mut(), min_price)
        });
        let rs_out = capture(|| unsafe {
            (rs.find_expensive_items)(std::ptr::null_mut(), min_price)
        });
        assert!(
            c_out.is_empty(),
            "C printed {:?} for NULL list",
            String::from_utf8_lossy(&c_out)
        );
        assert_eq!(c_out, rs_out, "NULL list output differs (min_price {min_price:?})");
    }
}

#[test]
fn find_expensive_no_match_message() {
    let (c, rs) = inventory_apis();
    let l = ListPair::<ItemT>::new();
    let mut rng = Rng::new(0xDEAD_0025);
    unsafe {
        let cl = (l.c.create)();
        let rl = (l.rs.create)();
        for _ in 0..6 {
            let mut it = ItemT::rand(&mut rng);
            it.price = 1.0;
            (l.c.append)(cl, it);
            (l.rs.append)(rl, it);
        }
        // Threshold above every price, and NaN (where every comparison is false).
        for (min_price, expected_header) in [
            (1000.0f64, "\n=== Items priced above $1000.00 ===\n"),
            (f64::NAN, "\n=== Items priced above $nan ===\n"),
            (-f64::NAN, "\n=== Items priced above $-nan ===\n"),
            (f64::INFINITY, "\n=== Items priced above $inf ===\n"),
        ] {
            let c_out = capture(|| (c.find_expensive_items)(cl, min_price));
            let rs_out = capture(|| (rs.find_expensive_items)(rl, min_price));
            let mut expected = expected_header.as_bytes().to_vec();
            expected.extend_from_slice(b"No items found above this price\n");
            assert_eq!(
                c_out,
                expected,
                "unexpected C output for min_price {min_price:?}: {:?}",
                String::from_utf8_lossy(&c_out)
            );
            assert_eq!(c_out, rs_out, "output differs for min_price {min_price:?}");
        }
        // An empty list takes the same "nothing found" path.
        let ce = (l.c.create)();
        let re = (l.rs.create)();
        let c_out = capture(|| (c.find_expensive_items)(ce, 0.0));
        let rs_out = capture(|| (rs.find_expensive_items)(re, 0.0));
        assert_eq!(c_out, rs_out);
        assert!(String::from_utf8_lossy(&c_out).contains("No items found above this price"));
        (l.c.destroy)(ce);
        (l.rs.destroy)(re);
        (l.c.destroy)(cl);
        (l.rs.destroy)(rl);
    }
}

// ============================================================================
// Generic boundary sweeps required by Phase C beyond the table
// ============================================================================

/// Zero and one-past-the-end sizes for every entry point that takes a length or
/// an index, plus NULL for every pointer parameter that C checks.
#[test]
fn generic_boundary_sweep() {
    let (c, rs) = inventory_apis();
    let ints = ArrayPair::<c_int>::new();
    let lists = ListPair::<c_int>::new();
    unsafe {
        // zero capacity, one element, index of the last element and of the first
        // slot past it (still inside the allocation)
        let ca = (ints.c.create)(0);
        let ra = (ints.rs.create)(0);
        assert_eq!((*ca).capacity, 16);
        assert_eq!((*ca).capacity, (*ra).capacity);
        assert_eq!((ints.c.push)(ca, 42), (ints.rs.push)(ra, 42));
        assert_eq!((ints.c.get)(ca, 0), (ints.rs.get)(ra, 0));
        assert_eq!((ints.c.size)(ca), 1);
        assert_eq!((ints.c.size)(ca), (ints.rs.size)(ra));
        (ints.c.destroy)(ca);
        (ints.rs.destroy)(ra);

        // list: size/clear/destroy on a fresh list, then one element
        let cl = (lists.c.create)();
        let rl = (lists.rs.create)();
        assert_eq!((lists.c.size)(cl), (lists.rs.size)(rl));
        (lists.c.clear)(cl);
        (lists.rs.clear)(rl);
        assert_eq!((lists.c.size)(cl), 0);
        assert_eq!((lists.rs.size)(rl), 0);
        assert_eq!((lists.c.prepend)(cl, -7), (lists.rs.prepend)(rl, -7));
        assert_eq!((lists.c.size)(cl), (lists.rs.size)(rl));
        (lists.c.destroy)(cl);
        (lists.rs.destroy)(rl);

        // "enum-like" values across the FFI boundary: the menu is the only enum
        // this API accepts, and it is covered end to end in driver_e2e; here the
        // analogous unchecked scalars are the extreme `int`s handed to
        // create_item/create_order, and the extreme `double`s handed to
        // find_expensive_items.
        let name = cstring(b"Boundary");
        for id in SPECIAL_INTS {
            for quantity in SPECIAL_INTS {
                let a = (c.create_item)(
                    id,
                    name.as_ptr() as *const c_char,
                    name.as_ptr() as *const c_char,
                    -0.0,
                    quantity,
                );
                let b = (rs.create_item)(
                    id,
                    name.as_ptr() as *const c_char,
                    name.as_ptr() as *const c_char,
                    -0.0,
                    quantity,
                );
                assert!(a.bit_eq(&b), "create_item({id}, .., {quantity}) differs");
                assert_same_output(
                    &format!("print_item(id={id}, quantity={quantity})"),
                    || (c.print_item)(a),
                    || (rs.print_item)(b),
                );
            }
        }
        for id in SPECIAL_INTS {
            for &total in SPECIAL_DOUBLES.iter() {
                let a = (c.create_order)(id, name.as_ptr() as *const c_char, total);
                let b = (rs.create_order)(id, name.as_ptr() as *const c_char, total);
                assert!(a.bit_eq(&b), "create_order({id}, {total:?}) differs");
                assert_same_output(
                    &format!("print_order(id={id}, total={total:?})"),
                    || (c.print_order)(a),
                    || (rs.print_order)(b),
                );
            }
        }
    }
}

/// An empty (`size == 0`) but non-NULL array passed to every entry point that
/// accepts one, to make sure the "no data" paths agree everywhere.
#[test]
fn empty_containers_everywhere() {
    let (c, rs) = inventory_apis();
    let items = ArrayPair::<ItemT>::new();
    let item_lists = ListPair::<ItemT>::new();
    let order_lists = ListPair::<OrderT>::new();
    let cat = cstring(b"Anything");
    unsafe {
        let ca = (items.c.create)(1);
        let ra = (items.rs.create)(1);
        assert_same_output(
            "empty array: calculate_inventory_stats",
            || (c.calculate_inventory_stats)(ca),
            || (rs.calculate_inventory_stats)(ra),
        );
        assert_same_output(
            "empty array: find_items_by_category",
            || (c.find_items_by_category)(ca, cat.as_ptr() as *const c_char),
            || (rs.find_items_by_category)(ra, cat.as_ptr() as *const c_char),
        );
        (items.c.destroy)(ca);
        (items.rs.destroy)(ra);

        let cil = (item_lists.c.create)();
        let ril = (item_lists.rs.create)();
        assert_same_output(
            "empty list: find_expensive_items",
            || (c.find_expensive_items)(cil, 0.0),
            || (rs.find_expensive_items)(ril, 0.0),
        );
        (item_lists.c.destroy)(cil);
        (item_lists.rs.destroy)(ril);

        let col = (order_lists.c.create)();
        let rol = (order_lists.rs.create)();
        assert_same_output(
            "empty list: calculate_order_stats",
            || (c.calculate_order_stats)(col),
            || (rs.calculate_order_stats)(rol),
        );
        (order_lists.c.destroy)(col);
        (order_lists.rs.destroy)(rol);
    }
}

/// Sanity: `size_of` agreement between the harness' `#[repr(C)]` mirrors and the
/// layout the C compiler produced (checked at build time in SYMBOLS.md).
#[test]
fn struct_layouts_match_c() {
    assert_eq!(size_of::<ItemT>(), 120);
    assert_eq!(size_of::<OrderT>(), 80);
    assert_eq!(size_of::<ArrayRaw<c_int>>(), 24);
    assert_eq!(size_of::<ListRaw<c_int>>(), 24);
    assert_eq!(size_of::<ListNodeRaw<c_int>>(), 16);
    assert_eq!(size_of::<ListNodeRaw<ItemT>>(), 128);
    assert_eq!(size_of::<ListNodeRaw<OrderT>>(), 88);
    // `c_void` is only referenced to keep the libc import honest.
    let _: Option<*mut c_void> = None;
}

// ============================================================================
// Rows 1, 10, 12, 14 — the small `malloc` calls fail
//
// These branches (`if (!arr) return NULL`, `if (!list) return NULL`,
// `if (!node) return -1`) need a heap that refuses even a 24/128-byte request.
// A forked child caps its address space with `RLIMIT_AS` and then allocates
// until every size fails, after which the entry point under test is called and
// its verdict is reported through the child's exit code.
// ============================================================================

/// Verdicts reported by the children (kept tiny: no allocation involved).
const VERDICT_NULL: c_int = 10;
const VERDICT_NON_NULL: c_int = 11;
const VERDICT_MINUS_ONE: c_int = 12;
const VERDICT_ZERO: c_int = 13;
const VERDICT_OTHER: c_int = 14;

/// Allocates until the allocator refuses every size class, so subsequent
/// `malloc`s of any size fail. Nothing is freed; the child exits right after.
///
/// `malloc` is called through a `dlsym`'d function pointer and every result goes
/// through `black_box`: with a direct `libc::malloc` call the optimizer knows the
/// allocator's semantics and deletes the whole loop body (an optimized build then
/// spins forever instead of exhausting anything).
unsafe fn exhaust_heap(malloc: unsafe extern "C" fn(usize) -> *mut c_void) {
    for size in [1usize << 20, 1 << 16, 1 << 12, 256, 128, 64, 24] {
        let mut consecutive_failures = 0u32;
        let mut attempts = 0u64;
        while consecutive_failures < 64 && attempts < 20_000_000 {
            attempts += 1;
            let p = std::hint::black_box(malloc(std::hint::black_box(size)));
            if p.is_null() {
                consecutive_failures += 1;
            } else {
                consecutive_failures = 0;
            }
        }
    }
}

fn address_space_limit() -> u64 {
    let statm = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
    let pages: u64 = statm
        .split_whitespace()
        .next()
        .expect("statm size field")
        .parse()
        .expect("statm size number");
    pages * 4096 + (8 << 20)
}

/// Runs `f` in a forked child whose heap has been exhausted, returning the
/// verdict `f` reported.
fn exhausted_child<F: FnOnce() -> c_int>(f: F) -> c_int {
    use std::io::Write;
    let limit = address_space_limit();
    // Resolved before the fork; `dlsym` itself allocates.
    let malloc_sym: libloading::Symbol<'static, unsafe extern "C" fn(usize) -> *mut c_void> =
        unsafe { sym(c_lib(), "malloc") };
    let malloc = *malloc_sym;
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };
    unsafe {
        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // A watchdog, so a regression fails the test loudly (SIGALRM) instead
            // of hanging the suite.
            libc::alarm(30);
            let rl = libc::rlimit {
                rlim_cur: limit,
                rlim_max: limit,
            };
            libc::setrlimit(libc::RLIMIT_AS, &rl);
            exhaust_heap(malloc);
            let verdict = f();
            libc::_exit(verdict);
        }
        let mut status: c_int = 0;
        assert!(libc::waitpid(pid, &mut status, 0) == pid);
        assert!(
            libc::WIFEXITED(status),
            "child died from signal {} instead of reporting a verdict",
            libc::WTERMSIG(status)
        );
        libc::WEXITSTATUS(status)
    }
}

#[test]
fn malloc_failure_paths() {
    let ints_a = ArrayPair::<c_int>::new();
    let items_a = ArrayPair::<ItemT>::new();
    let ints_l = ListPair::<c_int>::new();
    let items_l = ListPair::<ItemT>::new();

    // Warm up lazy symbol binding in the parent: the children cannot allocate, and
    // the dynamic linker's first call into a symbol may need memory.
    unsafe {
        let a = (ints_a.c.create)(1);
        (ints_a.c.destroy)(a);
        let a = (ints_a.rs.create)(1);
        (ints_a.rs.destroy)(a);
        let a = (items_a.c.create)(4);
        (items_a.c.destroy)(a);
        let a = (items_a.rs.create)(4);
        (items_a.rs.destroy)(a);
        let l = (ints_l.c.create)();
        (ints_l.c.destroy)(l);
        let l = (ints_l.rs.create)();
        (ints_l.rs.destroy)(l);
        let l = (items_l.c.create)();
        (items_l.c.destroy)(l);
        let l = (items_l.rs.create)();
        (items_l.rs.destroy)(l);
    }

    // Row 1 / 10 — the header allocation fails, so `create` returns NULL.
    let verdict = |p: *mut u8| {
        if p.is_null() {
            VERDICT_NULL
        } else {
            VERDICT_NON_NULL
        }
    };
    let cases: Vec<(&str, Box<dyn Fn() -> c_int>, Box<dyn Fn() -> c_int>)> = vec![
        (
            "array_int_create(1)",
            Box::new(move || unsafe { verdict((ints_a.c.create)(1) as *mut u8) }),
            Box::new(move || unsafe { verdict((ints_a.rs.create)(1) as *mut u8) }),
        ),
        (
            "array_item_t_create(4)",
            Box::new(move || unsafe { verdict((items_a.c.create)(4) as *mut u8) }),
            Box::new(move || unsafe { verdict((items_a.rs.create)(4) as *mut u8) }),
        ),
        (
            "list_int_create()",
            Box::new(move || unsafe { verdict((ints_l.c.create)() as *mut u8) }),
            Box::new(move || unsafe { verdict((ints_l.rs.create)() as *mut u8) }),
        ),
        (
            "list_item_t_create()",
            Box::new(move || unsafe { verdict((items_l.c.create)() as *mut u8) }),
            Box::new(move || unsafe { verdict((items_l.rs.create)() as *mut u8) }),
        ),
    ];
    for (label, c_case, rs_case) in cases {
        let c_verdict = exhausted_child(|| c_case());
        let rs_verdict = exhausted_child(|| rs_case());
        assert_eq!(
            c_verdict, VERDICT_NULL,
            "{label}: expected the C build to return NULL when malloc fails, got verdict {c_verdict}"
        );
        assert_eq!(
            c_verdict, rs_verdict,
            "{label}: C verdict {c_verdict} vs Rust verdict {rs_verdict}"
        );
    }

    // Rows 12 / 14 — the node allocation of append/prepend fails on a list that
    // was created *before* the heap ran out.
    let lists = ListPair::<ItemT>::new();
    let mut rng = Rng::new(0xDEAD_0012);
    let value = ItemT::rand(&mut rng);
    unsafe {
        let c_list = (lists.c.create)();
        let rs_list = (lists.rs.create)();
        assert!(!c_list.is_null() && !rs_list.is_null());
        for (label, is_append) in [("append", true), ("prepend", false)] {
            let code = |ret: c_int| match ret {
                -1 => VERDICT_MINUS_ONE,
                0 => VERDICT_ZERO,
                _ => VERDICT_OTHER,
            };
            let c_verdict = exhausted_child(|| {
                let ret = if is_append {
                    (lists.c.append)(c_list, value)
                } else {
                    (lists.c.prepend)(c_list, value)
                };
                code(ret)
            });
            let rs_verdict = exhausted_child(|| {
                let ret = if is_append {
                    (lists.rs.append)(rs_list, value)
                } else {
                    (lists.rs.prepend)(rs_list, value)
                };
                code(ret)
            });
            assert_eq!(
                c_verdict, VERDICT_MINUS_ONE,
                "list_item_t_{label}: expected -1 from the C build when the node malloc fails"
            );
            assert_eq!(
                c_verdict, rs_verdict,
                "list_item_t_{label}: C verdict {c_verdict} vs Rust verdict {rs_verdict}"
            );
        }
        // The parent's lists are untouched by the children.
        assert_eq!((lists.c.size)(c_list), 0);
        assert_eq!((lists.rs.size)(rs_list), 0);
        (lists.c.destroy)(c_list);
        (lists.rs.destroy)(rs_list);
    }
}

// ============================================================================
// Row 31 — fixed-size fields with no NUL byte
//
// `printf("%s", item.name)` and `strcmp(item.category, ...)` keep reading past
// the field. The part of that walk which stays inside the struct is fully
// determined by the bytes the caller passed, so it can be compared exactly; the
// structs below are built from raw bytes (padding included) with a NUL placed in
// a later field, so neither implementation ever reads past the struct.
// ============================================================================

/// Builds an `item_t` from its 120 raw bytes, so even the padding is defined.
fn item_from_bytes(bytes: &[u8; 120]) -> ItemT {
    unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const ItemT) }
}

fn order_from_bytes(bytes: &[u8; 80]) -> OrderT {
    unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const OrderT) }
}

#[test]
fn unterminated_fields_read_into_the_next_field() {
    let (c, rs) = inventory_apis();
    let arrays = ArrayPair::<ItemT>::new();
    let mut rng = Rng::new(0xDEAD_0031);

    for round in 0..200 {
        // item_t layout: id @0, name @4..68, category @68..100, pad @100..104,
        // price @104..112, quantity @112..116, pad @116..120.
        let mut bytes = [0u8; 120];
        // A fully non-NUL `name`, so `%s` runs into `category`.
        for b in bytes[4..68].iter_mut() {
            *b = b'A' + (rng.below(26) as u8);
        }
        // `category` gets a NUL somewhere inside it, which terminates both the
        // name walk and the category walk while staying inside the struct.
        let nul_at = 68 + 1 + rng.below(30);
        for i in 68..100 {
            bytes[i] = if i == nul_at {
                0
            } else {
                b'a' + (rng.below(26) as u8)
            };
        }
        // Defined padding and a finite price/quantity.
        for i in 100..120 {
            bytes[i] = (1 + rng.below(200)) as u8;
        }
        let item = item_from_bytes(&bytes);

        assert_same_output(
            &format!("print_item with an unterminated name (round {round})"),
            || unsafe { (c.print_item)(item) },
            || unsafe { (rs.print_item)(item) },
        );

        // `strcmp(item.category, category)` also walks past the field: a query
        // string longer than the visible part must compare against the bytes that
        // follow inside the struct.
        let visible = &bytes[68..nul_at];
        let mut query = visible.to_vec();
        if rng.below(2) == 0 {
            query.push(b'a' + (rng.below(26) as u8)); // one byte longer
        }
        let query = cstring(&query);
        unsafe {
            let ca = (arrays.c.create)(2);
            let ra = (arrays.rs.create)(2);
            (arrays.c.push)(ca, item);
            (arrays.rs.push)(ra, item);
            assert_same_output(
                &format!("find_items_by_category with an unterminated category (round {round})"),
                || (c.find_items_by_category)(ca, query.as_ptr() as *const c_char),
                || (rs.find_items_by_category)(ra, query.as_ptr() as *const c_char),
            );
            (arrays.c.destroy)(ca);
            (arrays.rs.destroy)(ra);
        }
    }

    for round in 0..200 {
        // order_t layout: customer_id @0, customer_name @4..68, pad @68..72,
        // total_amount @72..80.
        let mut bytes = [0u8; 80];
        for b in bytes[4..68].iter_mut() {
            *b = b'N' + (rng.below(10) as u8);
        }
        // The NUL lands in the padding that follows `customer_name`.
        let nul_at = 68 + rng.below(4);
        for i in 68..72 {
            bytes[i] = if i == nul_at { 0 } else { 0x41 };
        }
        for i in 72..80 {
            bytes[i] = (1 + rng.below(60)) as u8;
        }
        let order = order_from_bytes(&bytes);
        assert_same_output(
            &format!("print_order with an unterminated name (round {round})"),
            || unsafe { (c.print_order)(order) },
            || unsafe { (rs.print_order)(order) },
        );
    }
}
