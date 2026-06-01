use rect_pack_h::rect_pack::{Rect, RectOutInfo, RectPacker};

#[test]
fn test_rect_new() {
    let r = Rect::new(7, 12, 34);
    assert_eq!(r.id, 7);
    assert_eq!(r.w, 12);
    assert_eq!(r.h, 34);
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert_eq!(r.info.packed, false);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_rect_out_info_default() {
    let info = RectOutInfo::default();
    assert_eq!(info.x, 0);
    assert_eq!(info.y, 0);
    assert_eq!(info.packed, false);
    assert_eq!(info.page, 0);
}

#[test]
fn test_pack_empty() {
    let mut rects: Vec<Rect> = Vec::new();
    let ok = RectPacker::pack(100, 100, true, &mut rects);
    assert_eq!(ok, true);
    assert!(rects.is_empty());
}

#[test]
fn test_pack_uniform_paging() {
    // From C: i+1 sized squares, 100 of them, 512x512, paging=true => ok=true
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, i + 1, i + 1)).collect();
    let ok = RectPacker::pack(512, 512, true, &mut rects);
    assert_eq!(ok, true);
    // After pack, every rect must be packed
    for r in &rects {
        assert_eq!(r.info.packed, true);
    }
    // Sort order: descending max-side, so first rect should be the 100x100 with id=99
    assert_eq!(rects[0].id, 99);
    assert_eq!(rects[0].w, 100);
    assert_eq!(rects[0].h, 100);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.page, 0);
    // Last item is id=0, w=1 h=1, page=0, x=510 y=41
    assert_eq!(rects[99].id, 0);
    assert_eq!(rects[99].w, 1);
    assert_eq!(rects[99].h, 1);
    assert_eq!(rects[99].info.x, 510);
    assert_eq!(rects[99].info.y, 41);
    assert_eq!(rects[99].info.page, 0);
}

#[test]
fn test_pack_one_per_page() {
    // From C test: 4 rects each requires its own page in a 10x10 area, paging=true
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    let ok = RectPacker::pack(10, 10, true, &mut rects);
    assert_eq!(ok, true);
    // After sort: id=3 is first (10x9), id=2 second (8x9), id=1 third (8x7), id=0 last (6x7)
    assert_eq!(rects[0].id, 3);
    assert_eq!(rects[0].info.packed, true);
    assert_eq!(rects[0].info.page, 0);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);

    assert_eq!(rects[1].id, 2);
    assert_eq!(rects[1].info.packed, true);
    assert_eq!(rects[1].info.page, 1);
    assert_eq!(rects[1].info.x, 0);
    assert_eq!(rects[1].info.y, 0);

    assert_eq!(rects[2].id, 1);
    assert_eq!(rects[2].info.packed, true);
    assert_eq!(rects[2].info.page, 2);
    assert_eq!(rects[2].info.x, 0);
    assert_eq!(rects[2].info.y, 0);

    assert_eq!(rects[3].id, 0);
    assert_eq!(rects[3].info.packed, true);
    assert_eq!(rects[3].info.page, 3);
    assert_eq!(rects[3].info.x, 0);
    assert_eq!(rects[3].info.y, 0);
}

#[test]
fn test_pack_fill() {
    // 100 1x1 rects in 10x10 with no paging => all fit
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, 1, 1)).collect();
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert_eq!(ok, true);
    for r in &rects {
        assert_eq!(r.info.packed, true);
        assert_eq!(r.info.page, 0);
    }
}

#[test]
fn test_pack_no_paging_partial() {
    // From C: 4 rects in 10x10, no paging => only the largest fits
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert_eq!(ok, false);
    // Sort order: id=3 first
    assert_eq!(rects[0].id, 3);
    assert_eq!(rects[0].info.packed, true);
    assert_eq!(rects[0].info.page, 0);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);

    assert_eq!(rects[1].id, 2);
    assert_eq!(rects[1].info.packed, false);

    assert_eq!(rects[2].id, 1);
    assert_eq!(rects[2].info.packed, false);

    assert_eq!(rects[3].id, 0);
    assert_eq!(rects[3].info.packed, false);
}

#[test]
fn test_pack_fail_paging_too_big() {
    // From C: rect 1100x1100 cannot fit in 1000x1000, even with paging.
    // 900x900 will pack but 1100x1100 never can => returns false
    let mut rects = vec![Rect::new(0, 900, 900), Rect::new(1, 1100, 1100)];
    let ok = RectPacker::pack(1000, 1000, true, &mut rects);
    assert_eq!(ok, false);
    // After sort: id=1 (1100x1100) first (won't pack), id=0 (900x900) second (packs)
    assert_eq!(rects[0].id, 1);
    assert_eq!(rects[0].info.packed, false);
    assert_eq!(rects[1].id, 0);
    assert_eq!(rects[1].info.packed, true);
    assert_eq!(rects[1].info.page, 0);
    assert_eq!(rects[1].info.x, 0);
    assert_eq!(rects[1].info.y, 0);
}

#[test]
fn test_pack_single_fits() {
    let mut rects = vec![Rect::new(42, 10, 20)];
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert_eq!(ok, true);
    assert_eq!(rects[0].id, 42);
    assert_eq!(rects[0].w, 10);
    assert_eq!(rects[0].h, 20);
    assert_eq!(rects[0].info.packed, true);
    assert_eq!(rects[0].info.page, 0);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
}

#[test]
fn test_pack_single_too_big_paging() {
    let mut rects = vec![Rect::new(42, 200, 200)];
    let ok = RectPacker::pack(100, 100, true, &mut rects);
    assert_eq!(ok, false);
    assert_eq!(rects[0].id, 42);
    assert_eq!(rects[0].info.packed, false);
    assert_eq!(rects[0].info.page, 0);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
}

#[test]
fn test_pack_single_too_big_no_paging() {
    let mut rects = vec![Rect::new(42, 200, 200)];
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert_eq!(ok, false);
    assert_eq!(rects[0].id, 42);
    assert_eq!(rects[0].info.packed, false);
}

#[test]
fn test_pack_two_rects_grow_right() {
    // Two 5x10 rects => grow right, second placed at x=5,y=0
    let mut rects = vec![Rect::new(0, 5, 10), Rect::new(1, 5, 10)];
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert_eq!(ok, true);
    // Stable sort preserves order when equal sizes
    assert_eq!(rects[0].id, 0);
    assert_eq!(rects[0].info.packed, true);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);

    assert_eq!(rects[1].id, 1);
    assert_eq!(rects[1].info.packed, true);
    assert_eq!(rects[1].info.x, 5);
    assert_eq!(rects[1].info.y, 0);
}

#[test]
fn test_pack_two_rects_grow_down() {
    // Two 10x5 rects => grow down, second at x=0,y=5
    let mut rects = vec![Rect::new(0, 10, 5), Rect::new(1, 10, 5)];
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert_eq!(ok, true);
    assert_eq!(rects[0].id, 0);
    assert_eq!(rects[0].info.packed, true);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);

    assert_eq!(rects[1].id, 1);
    assert_eq!(rects[1].info.packed, true);
    assert_eq!(rects[1].info.x, 0);
    assert_eq!(rects[1].info.y, 5);
}

#[test]
fn test_pack_simple_4_squares() {
    // Four 5x5 squares packed into 10x10
    // C output:
    //   i=0 id=0 x=0 y=0
    //   i=1 id=1 x=5 y=0
    //   i=2 id=2 x=0 y=5
    //   i=3 id=3 x=5 y=5
    let mut rects: Vec<Rect> = (0..4).map(|i| Rect::new(i, 5, 5)).collect();
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert_eq!(ok, true);

    assert_eq!(rects[0].id, 0);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.packed, true);
    assert_eq!(rects[0].info.page, 0);

    assert_eq!(rects[1].id, 1);
    assert_eq!(rects[1].info.x, 5);
    assert_eq!(rects[1].info.y, 0);
    assert_eq!(rects[1].info.packed, true);
    assert_eq!(rects[1].info.page, 0);

    assert_eq!(rects[2].id, 2);
    assert_eq!(rects[2].info.x, 0);
    assert_eq!(rects[2].info.y, 5);
    assert_eq!(rects[2].info.packed, true);
    assert_eq!(rects[2].info.page, 0);

    assert_eq!(rects[3].id, 3);
    assert_eq!(rects[3].info.x, 5);
    assert_eq!(rects[3].info.y, 5);
    assert_eq!(rects[3].info.packed, true);
    assert_eq!(rects[3].info.page, 0);
}

#[test]
fn test_pack_many_small_no_paging() {
    // 16 5x5 rects in 20x20, no paging => all fit
    let mut rects: Vec<Rect> = (0..16).map(|i| Rect::new(i, 5, 5)).collect();
    let ok = RectPacker::pack(20, 20, false, &mut rects);
    assert_eq!(ok, true);
    for r in &rects {
        assert_eq!(r.info.packed, true);
        assert_eq!(r.info.page, 0);
    }
}

#[test]
fn test_pack_loop_idempotent() {
    // From C test_rect_pack_loop: re-packing same rects repeatedly should still succeed.
    let widths_heights: [(i32, i32); 8] = [
        (255, 255),
        (255, 253),
        (253, 255),
        (251, 255),
        (255, 251),
        (249, 255),
        (255, 249),
        (247, 255),
    ];
    let mut rects: Vec<Rect> = widths_heights
        .iter()
        .enumerate()
        .map(|(i, &(w, h))| Rect::new(i as i32, w, h))
        .collect();
    for _ in 0..5 {
        let ok = RectPacker::pack(500, 500, true, &mut rects);
        assert_eq!(ok, true);
        for r in &rects {
            assert_eq!(r.info.packed, true);
        }
    }
}

#[test]
fn test_pack_resets_info_each_call() {
    // info.packed should be reset to false at start of each call.
    let mut rects = vec![Rect::new(0, 200, 200)];
    rects[0].info.packed = true;
    rects[0].info.x = 9999;
    rects[0].info.y = 9999;
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert_eq!(ok, false);
    assert_eq!(rects[0].info.packed, false);
    // x/y reset to 0 since packing failed for this single rect
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.page, 0);
}

#[test]
fn test_pack_sort_order_max_side() {
    // Rectangles get sorted by max(w,h) descending, then min(w,h) descending.
    // Inputs: (3,3), (5,1), (2,5), (5,5) => after sort by max-side desc:
    // max=5: (5,5), (5,1)+(2,5) tie at max=5. Tiebreak by min(w,h) desc.
    // (5,5) min=5, (5,1) min=1, (2,5) min=2.  Order: (5,5), (2,5), (5,1)
    // Then max=3: (3,3) last.
    let mut rects = vec![
        Rect::new(0, 3, 3),
        Rect::new(1, 5, 1),
        Rect::new(2, 2, 5),
        Rect::new(3, 5, 5),
    ];
    RectPacker::pack(100, 100, false, &mut rects);
    assert_eq!(rects[0].id, 3);
    assert_eq!(rects[1].id, 2);
    assert_eq!(rects[2].id, 1);
    assert_eq!(rects[3].id, 0);
}

fn main() {}
