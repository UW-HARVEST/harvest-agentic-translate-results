use rect_pack_h::rect_pack::{Rect, RectPacker};

/// Helper: find a rectangle by id (rectangles get re-ordered after pack call)
fn find_by_id(rects: &[Rect], id: i32) -> &Rect {
    rects.iter().find(|r| r.id == id).expect("rect with id not found")
}

#[test]
fn test_empty_rects() {
    // C: rects_size == 0 -> returns true
    let mut rects: Vec<Rect> = Vec::new();
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert!(ok, "empty rects should return true");
    assert_eq!(rects.len(), 0);
}

#[test]
fn test_empty_rects_with_paging() {
    let mut rects: Vec<Rect> = Vec::new();
    let ok = RectPacker::pack(10, 10, true, &mut rects);
    assert!(ok);
    assert_eq!(rects.len(), 0);
}

#[test]
fn test_single_rect_fits() {
    // From C runner: 100 100 0 1 / 0 5 5 -> RESULT 1, 0 0 0 1 0
    let mut rects = vec![Rect::new(0, 5, 5)];
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert!(ok);
    assert_eq!(rects.len(), 1);
    let r = find_by_id(&rects, 0);
    assert_eq!(r.id, 0);
    assert_eq!(r.w, 5);
    assert_eq!(r.h, 5);
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert!(r.info.packed);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_single_rect_exact_fit() {
    // From C runner: 10 10 0 1 / 0 10 10 -> RESULT 1, 0 0 0 1 0
    let mut rects = vec![Rect::new(0, 10, 10)];
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert!(ok);
    let r = find_by_id(&rects, 0);
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert!(r.info.packed);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_single_rect_too_big_no_paging() {
    // From C runner: 10 10 0 1 / 0 20 20 -> RESULT 0, 0 0 0 0 0
    let mut rects = vec![Rect::new(0, 20, 20)];
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert!(!ok);
    let r = find_by_id(&rects, 0);
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert!(!r.info.packed);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_single_rect_too_big_paging() {
    // From C runner: 10 10 1 1 / 0 20 20 -> RESULT 0, 0 0 0 0 0
    let mut rects = vec![Rect::new(0, 20, 20)];
    let ok = RectPacker::pack(10, 10, true, &mut rects);
    assert!(!ok);
    let r = find_by_id(&rects, 0);
    assert!(!r.info.packed);
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_one_rect_per_page_paging() {
    // C test: rect_pack_one_rect_per_page_paging
    // From C runner output for 10 10 1 4 with 6,7 / 8,7 / 8,9 / 10,9:
    // RESULT 1, in sorted order (descending max side): id 3,2,1,0 each at (0,0) page 0,1,2,3
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    let ok = RectPacker::pack(10, 10, true, &mut rects);
    assert!(ok);
    assert_eq!(rects.len(), 4);

    // Expected: largest first; sorted in descending max-side order so id=3 first.
    // C runner output: id 3,2,1,0 each at (0,0) on pages 0,1,2,3.
    let r3 = find_by_id(&rects, 3);
    assert_eq!(r3.info.x, 0);
    assert_eq!(r3.info.y, 0);
    assert!(r3.info.packed);
    assert_eq!(r3.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert_eq!(r2.info.x, 0);
    assert_eq!(r2.info.y, 0);
    assert!(r2.info.packed);
    assert_eq!(r2.info.page, 1);

    let r1 = find_by_id(&rects, 1);
    assert_eq!(r1.info.x, 0);
    assert_eq!(r1.info.y, 0);
    assert!(r1.info.packed);
    assert_eq!(r1.info.page, 2);

    let r0 = find_by_id(&rects, 0);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.page, 3);
}

#[test]
fn test_no_paging_partial_fit() {
    // C test: test_rect_pack_no_paging
    // 10 10 0 4 with 6,7 / 8,7 / 8,9 / 10,9
    // From C runner: RESULT 0
    //   3 0 0 1 0  (10x9 fits at 0,0)
    //   2 0 0 0 0
    //   1 0 0 0 0
    //   0 0 0 0 0
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert!(!ok);

    let r0 = find_by_id(&rects, 0);
    assert!(!r0.info.packed);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert_eq!(r0.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(!r1.info.packed);
    assert_eq!(r1.info.x, 0);
    assert_eq!(r1.info.y, 0);
    assert_eq!(r1.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert!(!r2.info.packed);
    assert_eq!(r2.info.x, 0);
    assert_eq!(r2.info.y, 0);
    assert_eq!(r2.info.page, 0);

    let r3 = find_by_id(&rects, 3);
    assert!(r3.info.packed);
    assert_eq!(r3.info.x, 0);
    assert_eq!(r3.info.y, 0);
    assert_eq!(r3.info.page, 0);
}

#[test]
fn test_fail_paging() {
    // C test: test_rect_pack_fail_paging
    // 1000 1000 1 2: rect 0=900,900 (fits), rect 1=1100,1100 (doesn't fit ever)
    // From C runner: RESULT 0
    //   1 0 0 0 0
    //   0 0 0 1 0
    let mut rects = vec![Rect::new(0, 900, 900), Rect::new(1, 1100, 1100)];
    let ok = RectPacker::pack(1000, 1000, true, &mut rects);
    assert!(!ok);
    assert_eq!(rects.len(), 2);

    let r0 = find_by_id(&rects, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert_eq!(r0.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(!r1.info.packed);
    assert_eq!(r1.info.x, 0);
    assert_eq!(r1.info.y, 0);
    assert_eq!(r1.info.page, 0);
}

#[test]
fn test_fill_100_1x1_in_10x10() {
    // C test: test_rect_pack_fill — 100 1x1 rects all fit on page 0
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, 1, 1)).collect();
    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert!(ok);
    assert_eq!(rects.len(), 100);

    for i in 0..100 {
        let r = find_by_id(&rects, i);
        assert!(r.info.packed, "rect {} not packed", i);
        assert_eq!(r.info.page, 0, "rect {} not on page 0", i);
    }
}

#[test]
fn test_uniform_paging_512_512_first_layout() {
    // C test: test_rect_pack_uniform_paging — 100 rects with w=h=i+1 in 512x512 paging
    // From C runner output, validate exact (x, y, page) for many rectangles.
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, i + 1, i + 1)).collect();
    let ok = RectPacker::pack(512, 512, true, &mut rects);
    assert!(ok);
    assert_eq!(rects.len(), 100);

    // Expected (id, x, y, packed, page)
    let expected: &[(i32, i32, i32, bool, i32)] = &[
        (99, 0, 0, true, 0),
        (98, 100, 0, true, 0),
        (97, 0, 100, true, 0),
        (96, 98, 100, true, 0),
        (95, 199, 0, true, 0),
        (94, 199, 96, true, 0),
        (93, 0, 198, true, 0),
        (92, 94, 198, true, 0),
        (91, 187, 198, true, 0),
        (90, 295, 0, true, 0),
        (89, 295, 91, true, 0),
        (88, 295, 181, true, 0),
        (87, 0, 292, true, 0),
        (86, 88, 292, true, 0),
        (85, 175, 292, true, 0),
        (84, 261, 292, true, 0),
        (83, 386, 0, true, 0),
        (82, 386, 84, true, 0),
        (81, 386, 167, true, 0),
        (80, 386, 249, true, 0),
        (79, 0, 380, true, 0),
        (78, 80, 380, true, 0),
        (77, 159, 380, true, 0),
        (76, 237, 380, true, 0),
        (75, 314, 380, true, 0),
        (74, 390, 380, true, 0),
        (73, 0, 0, true, 1),
        (72, 74, 0, true, 1),
        (71, 0, 74, true, 1),
        (70, 72, 74, true, 1),
        (69, 147, 0, true, 1),
        (68, 147, 70, true, 1),
        (67, 0, 146, true, 1),
        (66, 68, 146, true, 1),
        (65, 135, 146, true, 1),
        (64, 217, 0, true, 1),
        (63, 217, 65, true, 1),
        (62, 217, 129, true, 1),
        (61, 0, 214, true, 1),
        (60, 62, 214, true, 1),
        (59, 123, 214, true, 1),
        (58, 183, 214, true, 1),
        (57, 282, 0, true, 1),
        (56, 282, 58, true, 1),
        (55, 282, 115, true, 1),
        (54, 282, 171, true, 1),
        (53, 0, 276, true, 1),
        (52, 54, 276, true, 1),
        (51, 0, 460, true, 0),
        (50, 52, 460, true, 0),
        (49, 386, 330, true, 0),
        (48, 103, 460, true, 0),
        (47, 152, 460, true, 0),
        (46, 200, 460, true, 0),
        (45, 247, 460, true, 0),
        (44, 293, 460, true, 0),
        (43, 338, 460, true, 0),
        (42, 382, 460, true, 0),
        (41, 425, 460, true, 0),
        (40, 470, 0, true, 0),
        (39, 470, 41, true, 0),
        (38, 470, 81, true, 0),
        (37, 470, 120, true, 0),
        (36, 470, 158, true, 0),
        (35, 470, 195, true, 0),
        (34, 470, 231, true, 0),
        (33, 470, 266, true, 0),
        (32, 470, 300, true, 0),
        (31, 470, 333, true, 0),
        (30, 470, 365, true, 0),
        (29, 470, 396, true, 0),
        (28, 470, 426, true, 0),
        (27, 470, 455, true, 0),
        (26, 470, 483, true, 0),
        (25, 436, 330, true, 0),
        (24, 346, 292, true, 0),
        (23, 436, 356, true, 0),
        (22, 346, 317, true, 0),
        (21, 295, 270, true, 0),
        (20, 317, 270, true, 0),
        (19, 338, 270, true, 0),
        (18, 358, 270, true, 0),
        (17, 346, 340, true, 0),
        (16, 369, 317, true, 0),
        (15, 279, 198, true, 0),
        (14, 279, 214, true, 0),
        (13, 497, 483, true, 0),
        (12, 498, 455, true, 0),
        (11, 499, 426, true, 0),
        (10, 500, 396, true, 0),
        (9, 501, 365, true, 0),
        (8, 502, 333, true, 0),
        (7, 503, 300, true, 0),
        (6, 504, 266, true, 0),
        (5, 505, 231, true, 0),
        (4, 506, 195, true, 0),
        (3, 507, 158, true, 0),
        (2, 508, 120, true, 0),
        (1, 509, 81, true, 0),
        (0, 510, 41, true, 0),
    ];

    for &(id, ex_x, ex_y, ex_packed, ex_page) in expected {
        let r = find_by_id(&rects, id);
        assert_eq!(r.info.packed, ex_packed, "rect {} packed mismatch", id);
        assert_eq!(r.info.x, ex_x, "rect {} x mismatch (got {}, expected {})", id, r.info.x, ex_x);
        assert_eq!(r.info.y, ex_y, "rect {} y mismatch (got {}, expected {})", id, r.info.y, ex_y);
        assert_eq!(r.info.page, ex_page, "rect {} page mismatch", id);
    }
}

#[test]
fn test_no_paging_partial_5_50x50() {
    // From C runner: 100 100 0 5 / 5 rects of 50x50
    // 4 fit, last one fails
    // Output:
    //   0 0 0 1 0
    //   1 50 0 1 0
    //   2 0 50 1 0
    //   3 50 50 1 0
    //   4 0 0 0 0
    let mut rects: Vec<Rect> = (0..5).map(|i| Rect::new(i, 50, 50)).collect();
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert!(!ok);

    // The 4 first packed in id order (sort is stable for ties)
    let r0 = find_by_id(&rects, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert_eq!(r0.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(r1.info.packed);
    assert_eq!(r1.info.x, 50);
    assert_eq!(r1.info.y, 0);
    assert_eq!(r1.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert!(r2.info.packed);
    assert_eq!(r2.info.x, 0);
    assert_eq!(r2.info.y, 50);
    assert_eq!(r2.info.page, 0);

    let r3 = find_by_id(&rects, 3);
    assert!(r3.info.packed);
    assert_eq!(r3.info.x, 50);
    assert_eq!(r3.info.y, 50);
    assert_eq!(r3.info.page, 0);

    let r4 = find_by_id(&rects, 4);
    assert!(!r4.info.packed);
    assert_eq!(r4.info.x, 0);
    assert_eq!(r4.info.y, 0);
    assert_eq!(r4.info.page, 0);
}

#[test]
fn test_paging_5_50x50() {
    // From C runner: 100 100 1 5 / 5 rects of 50x50
    //   0,1,2,3 on page 0; 4 on page 1
    let mut rects: Vec<Rect> = (0..5).map(|i| Rect::new(i, 50, 50)).collect();
    let ok = RectPacker::pack(100, 100, true, &mut rects);
    assert!(ok);

    let r0 = find_by_id(&rects, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert_eq!(r0.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(r1.info.packed);
    assert_eq!(r1.info.x, 50);
    assert_eq!(r1.info.y, 0);
    assert_eq!(r1.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert!(r2.info.packed);
    assert_eq!(r2.info.x, 0);
    assert_eq!(r2.info.y, 50);
    assert_eq!(r2.info.page, 0);

    let r3 = find_by_id(&rects, 3);
    assert!(r3.info.packed);
    assert_eq!(r3.info.x, 50);
    assert_eq!(r3.info.y, 50);
    assert_eq!(r3.info.page, 0);

    let r4 = find_by_id(&rects, 4);
    assert!(r4.info.packed);
    assert_eq!(r4.info.x, 0);
    assert_eq!(r4.info.y, 0);
    assert_eq!(r4.info.page, 1);
}

#[test]
fn test_three_50x50_in_100x100() {
    // From C runner: 100 100 0 3 / 3 rects of 50x50
    // 0 0 0 1 0
    // 1 50 0 1 0
    // 2 0 50 1 0
    let mut rects: Vec<Rect> = (0..3).map(|i| Rect::new(i, 50, 50)).collect();
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert!(ok);

    let r0 = find_by_id(&rects, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert_eq!(r0.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(r1.info.packed);
    assert_eq!(r1.info.x, 50);
    assert_eq!(r1.info.y, 0);
    assert_eq!(r1.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert!(r2.info.packed);
    assert_eq!(r2.info.x, 0);
    assert_eq!(r2.info.y, 50);
    assert_eq!(r2.info.page, 0);
}

#[test]
fn test_four_50x50_in_100x100_exact() {
    // From C runner: 100 100 0 4 / 4 rects of 50x50
    let mut rects: Vec<Rect> = (0..4).map(|i| Rect::new(i, 50, 50)).collect();
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert!(ok);

    let r0 = find_by_id(&rects, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.x, 0);
    assert_eq!(r0.info.y, 0);
    assert_eq!(r0.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(r1.info.packed);
    assert_eq!(r1.info.x, 50);
    assert_eq!(r1.info.y, 0);
    assert_eq!(r1.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert!(r2.info.packed);
    assert_eq!(r2.info.x, 0);
    assert_eq!(r2.info.y, 50);
    assert_eq!(r2.info.page, 0);

    let r3 = find_by_id(&rects, 3);
    assert!(r3.info.packed);
    assert_eq!(r3.info.x, 50);
    assert_eq!(r3.info.y, 50);
    assert_eq!(r3.info.page, 0);
}

#[test]
fn test_small_sort_case() {
    // From C runner: 100 100 0 5 with mixed sizes
    // sorted by max-side desc: 3(30,20), 1(20,10), 4(15,15), 2(8,8), 0(5,5)
    // Output (in sorted order):
    //   3 0 0 1 0
    //   1 0 20 1 0
    //   4 30 0 1 0
    //   2 30 15 1 0
    //   0 38 15 1 0
    let mut rects = vec![
        Rect::new(0, 5, 5),
        Rect::new(1, 20, 10),
        Rect::new(2, 8, 8),
        Rect::new(3, 30, 20),
        Rect::new(4, 15, 15),
    ];
    let ok = RectPacker::pack(100, 100, false, &mut rects);
    assert!(ok);

    let r3 = find_by_id(&rects, 3);
    assert!(r3.info.packed);
    assert_eq!(r3.info.x, 0);
    assert_eq!(r3.info.y, 0);
    assert_eq!(r3.info.page, 0);

    let r1 = find_by_id(&rects, 1);
    assert!(r1.info.packed);
    assert_eq!(r1.info.x, 0);
    assert_eq!(r1.info.y, 20);
    assert_eq!(r1.info.page, 0);

    let r4 = find_by_id(&rects, 4);
    assert!(r4.info.packed);
    assert_eq!(r4.info.x, 30);
    assert_eq!(r4.info.y, 0);
    assert_eq!(r4.info.page, 0);

    let r2 = find_by_id(&rects, 2);
    assert!(r2.info.packed);
    assert_eq!(r2.info.x, 30);
    assert_eq!(r2.info.y, 15);
    assert_eq!(r2.info.page, 0);

    let r0 = find_by_id(&rects, 0);
    assert!(r0.info.packed);
    assert_eq!(r0.info.x, 38);
    assert_eq!(r0.info.y, 15);
    assert_eq!(r0.info.page, 0);
}

#[test]
fn test_mixed_size_rects_paging() {
    // From C runner with 20 deterministic rects in 100x100 with paging:
    //   100 100 1 20
    //   (0,21,13),(1,6,16),(2,11,9),(3,12,5),(4,19,15),(5,24,23),(6,13,14),(7,14,19),
    //   (8,7,8),(9,8,9),(10,12,16),(11,8,25),(12,19,12),(13,18,21),(14,11,24),
    //   (15,6,14),(16,22,18),(17,30,26),(18,25,6),(19,30,9)
    // RESULT 1
    let inputs: Vec<(i32, i32, i32)> = vec![
        (0, 21, 13),
        (1, 6, 16),
        (2, 11, 9),
        (3, 12, 5),
        (4, 19, 15),
        (5, 24, 23),
        (6, 13, 14),
        (7, 14, 19),
        (8, 7, 8),
        (9, 8, 9),
        (10, 12, 16),
        (11, 8, 25),
        (12, 19, 12),
        (13, 18, 21),
        (14, 11, 24),
        (15, 6, 14),
        (16, 22, 18),
        (17, 30, 26),
        (18, 25, 6),
        (19, 30, 9),
    ];
    let mut rects: Vec<Rect> = inputs
        .iter()
        .map(|(id, w, h)| Rect::new(*id, *w, *h))
        .collect();
    let ok = RectPacker::pack(100, 100, true, &mut rects);
    assert!(ok);
    assert_eq!(rects.len(), 20);

    // From C runner output (id, x, y, packed, page):
    let expected: &[(i32, i32, i32, bool, i32)] = &[
        (17, 0, 0, true, 0),
        (19, 30, 0, true, 0),
        (11, 0, 26, true, 0),
        (18, 30, 9, true, 0),
        (5, 8, 26, true, 0),
        (14, 60, 0, true, 0),
        (16, 32, 26, true, 0),
        (13, 71, 0, true, 0),
        (0, 0, 51, true, 0),
        (4, 0, 64, true, 0),
        (7, 71, 21, true, 0),
        (12, 21, 51, true, 0),
        (10, 0, 79, true, 0),
        (1, 60, 24, true, 0),
        (6, 19, 64, true, 0),
        (15, 54, 26, true, 0),
        (3, 71, 40, true, 0),
        (2, 60, 40, true, 0),
        (9, 30, 15, true, 0),
        (8, 38, 15, true, 0),
    ];

    for &(id, ex_x, ex_y, ex_packed, ex_page) in expected {
        let r = find_by_id(&rects, id);
        assert_eq!(r.info.packed, ex_packed, "rect {} packed mismatch", id);
        assert_eq!(r.info.x, ex_x, "rect {} x: got {}, expected {}", id, r.info.x, ex_x);
        assert_eq!(r.info.y, ex_y, "rect {} y: got {}, expected {}", id, r.info.y, ex_y);
        assert_eq!(r.info.page, ex_page, "rect {} page mismatch", id);
    }
}

#[test]
fn test_rect_constructor_default() {
    // Test the Rect::new constructor and default RectOutInfo
    let r = Rect::new(7, 100, 200);
    assert_eq!(r.id, 7);
    assert_eq!(r.w, 100);
    assert_eq!(r.h, 200);
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert!(!r.info.packed);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_pack_resets_existing_info() {
    // The C code resets all rect info before packing.
    // Verify that pre-existing info is reset.
    let mut rects = vec![Rect::new(0, 5, 5)];
    rects[0].info.packed = true;
    rects[0].info.x = 999;
    rects[0].info.y = 888;
    rects[0].info.page = 7;

    let ok = RectPacker::pack(10, 10, false, &mut rects);
    assert!(ok);
    let r = find_by_id(&rects, 0);
    // After packing in 10x10, the 5x5 rect is placed at (0,0) on page 0
    assert_eq!(r.info.x, 0);
    assert_eq!(r.info.y, 0);
    assert!(r.info.packed);
    assert_eq!(r.info.page, 0);
}

#[test]
fn test_packed_rects_within_bounds_and_no_overlap() {
    // Property test: when packing succeeds, all rects must be within bounds
    // and not overlap (within their page).
    let mut rects: Vec<Rect> = (1..=20).map(|i| Rect::new(i, i, i + 1)).collect();
    let max_w = 100;
    let max_h = 100;
    let ok = RectPacker::pack(max_w, max_h, true, &mut rects);
    assert!(ok);

    // All packed
    for r in &rects {
        assert!(r.info.packed, "rect {} not packed", r.id);
        assert!(
            r.info.x >= 0 && r.info.x + r.w <= max_w,
            "rect {} x out of bounds: x={}, w={}",
            r.id,
            r.info.x,
            r.w
        );
        assert!(
            r.info.y >= 0 && r.info.y + r.h <= max_h,
            "rect {} y out of bounds: y={}, h={}",
            r.id,
            r.info.y,
            r.h
        );
    }

    // No overlap within same page
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            if rects[i].info.page != rects[j].info.page {
                continue;
            }
            let a = &rects[i];
            let b = &rects[j];
            let no_overlap = a.info.x + a.w <= b.info.x
                || b.info.x + b.w <= a.info.x
                || a.info.y + a.h <= b.info.y
                || b.info.y + b.h <= a.info.y;
            assert!(
                no_overlap,
                "rects {} and {} overlap on page {}: a=({},{},{}x{}), b=({},{},{}x{})",
                a.id,
                b.id,
                a.info.page,
                a.info.x,
                a.info.y,
                a.w,
                a.h,
                b.info.x,
                b.info.y,
                b.w,
                b.h,
            );
        }
    }
}

fn main() {}
