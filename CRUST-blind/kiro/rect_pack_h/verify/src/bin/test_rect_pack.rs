use rect_pack_h::rect_pack::{Rect, RectPacker};

fn check(r: &Rect, id: i32, w: i32, h: i32, packed: bool, x: i32, y: i32, page: i32) {
    assert_eq!(r.id, id, "id mismatch");
    assert_eq!(r.w, w, "w mismatch for id={}", id);
    assert_eq!(r.h, h, "h mismatch for id={}", id);
    assert_eq!(r.info.packed, packed, "packed mismatch for id={}", id);
    assert_eq!(r.info.x, x, "x mismatch for id={}", id);
    assert_eq!(r.info.y, y, "y mismatch for id={}", id);
    assert_eq!(r.info.page, page, "page mismatch for id={}", id);
}

#[test]
fn test_empty() {
    let mut rects: Vec<Rect> = vec![];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
}

#[test]
fn test_single_rect() {
    let mut rects = vec![Rect::new(0, 50, 30)];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
    check(&rects[0], 0, 50, 30, true, 0, 0, 0);
}

#[test]
fn test_single_too_big() {
    let mut rects = vec![Rect::new(0, 200, 200)];
    assert!(!RectPacker::pack(100, 100, false, &mut rects));
    assert_eq!(rects[0].info.packed, false);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.page, 0);
}

#[test]
fn test_two_rects_fit() {
    let mut rects = vec![Rect::new(0, 50, 50), Rect::new(1, 50, 50)];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
    check(&rects[0], 0, 50, 50, true, 0, 0, 0);
    check(&rects[1], 1, 50, 50, true, 50, 0, 0);
}

#[test]
fn test_fill_1x1() {
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, 1, 1)).collect();
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    for r in &rects {
        assert!(r.info.packed);
        assert_eq!(r.info.page, 0);
    }
    // Spot-check a few positions from C ground truth
    // All 1x1 identical so ids don't change order. Check first and last.
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[99].info.x, 9);
    assert_eq!(rects[99].info.y, 9);
}

#[test]
fn test_no_paging() {
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    assert!(!RectPacker::pack(10, 10, false, &mut rects));
    // After sorting: id=3(10x9), id=2(8x9), id=1(8x7), id=0(6x7)
    check(&rects[0], 3, 10, 9, true, 0, 0, 0);
    assert_eq!(rects[1].id, 2);
    assert_eq!(rects[1].info.packed, false);
    assert_eq!(rects[2].id, 1);
    assert_eq!(rects[2].info.packed, false);
    assert_eq!(rects[3].id, 0);
    assert_eq!(rects[3].info.packed, false);
}

#[test]
fn test_one_rect_per_page_paging() {
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    assert!(RectPacker::pack(10, 10, true, &mut rects));
    // Sorted by max side desc: id=3, id=2, id=1, id=0
    check(&rects[0], 3, 10, 9, true, 0, 0, 0);
    check(&rects[1], 2, 8, 9, true, 0, 0, 1);
    check(&rects[2], 1, 8, 7, true, 0, 0, 2);
    check(&rects[3], 0, 6, 7, true, 0, 0, 3);
}

#[test]
fn test_fail_paging() {
    let mut rects = vec![Rect::new(0, 900, 900), Rect::new(1, 1100, 1100)];
    assert!(!RectPacker::pack(1000, 1000, true, &mut rects));
    // After sorting: id=1(1100x1100) first, id=0(900x900) second
    check(&rects[0], 1, 1100, 1100, false, 0, 0, 0);
    check(&rects[1], 0, 900, 900, true, 0, 0, 0);
}

#[test]
fn test_exact_fit() {
    let mut rects = vec![Rect::new(0, 100, 100)];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
    check(&rects[0], 0, 100, 100, true, 0, 0, 0);
}

#[test]
fn test_sorting_order() {
    let mut rects = vec![
        Rect::new(0, 5, 5),
        Rect::new(1, 20, 20),
        Rect::new(2, 10, 10),
    ];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
    // Sorted: id=1(20x20), id=2(10x10), id=0(5x5)
    check(&rects[0], 1, 20, 20, true, 0, 0, 0);
    check(&rects[1], 2, 10, 10, true, 20, 0, 0);
    check(&rects[2], 0, 5, 5, true, 20, 10, 0);
}

#[test]
fn test_three_small() {
    let mut rects = vec![
        Rect::new(0, 10, 20),
        Rect::new(1, 15, 10),
        Rect::new(2, 5, 5),
    ];
    assert!(RectPacker::pack(30, 30, false, &mut rects));
    // Sorted by max side: id=0(max=20), id=1(max=15), id=2(max=5)
    check(&rects[0], 0, 10, 20, true, 0, 0, 0);
    check(&rects[1], 1, 15, 10, true, 10, 0, 0);
    check(&rects[2], 2, 5, 5, true, 10, 10, 0);
}

#[test]
fn test_paging_multiple_pages() {
    let mut rects = vec![
        Rect::new(0, 80, 80),
        Rect::new(1, 80, 80),
        Rect::new(2, 80, 80),
        Rect::new(3, 80, 80),
    ];
    assert!(RectPacker::pack(100, 100, true, &mut rects));
    // All same size, each on its own page
    for i in 0..4 {
        assert!(rects[i].info.packed);
        assert_eq!(rects[i].info.x, 0);
        assert_eq!(rects[i].info.y, 0);
        assert_eq!(rects[i].info.page, i as i32);
    }
}

#[test]
fn test_wide_and_tall() {
    let mut rects = vec![Rect::new(0, 90, 10), Rect::new(1, 10, 90)];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
    // Both have max side 90, min side: 10 vs 10 => same, so order preserved
    check(&rects[0], 0, 90, 10, true, 0, 0, 0);
    check(&rects[1], 1, 10, 90, true, 0, 10, 0);
}

#[test]
fn test_uniform_paging() {
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, (i + 1) as i32, (i + 1) as i32)).collect();
    assert!(RectPacker::pack(512, 512, true, &mut rects));
    // All should be packed
    for r in &rects {
        assert!(r.info.packed, "rect id={} not packed", r.id);
    }
    // After sorting: id=99(100x100) first, id=98(99x99) second, etc.
    assert_eq!(rects[0].id, 99);
    assert_eq!(rects[0].w, 100);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.page, 0);

    assert_eq!(rects[1].id, 98);
    assert_eq!(rects[1].info.x, 100);
    assert_eq!(rects[1].info.y, 0);
    assert_eq!(rects[1].info.page, 0);

    // Check that paging is used (some rects on page 1)
    let max_page = rects.iter().map(|r| r.info.page).max().unwrap();
    assert_eq!(max_page, 1);

    // Verify specific page-1 rects from C ground truth
    // rects[26] should be id=73, page=1
    assert_eq!(rects[26].id, 73);
    assert_eq!(rects[26].info.page, 1);
    assert_eq!(rects[26].info.x, 0);
    assert_eq!(rects[26].info.y, 0);

    // Last rect: id=0, w=1, h=1
    assert_eq!(rects[99].id, 0);
    assert_eq!(rects[99].w, 1);
    assert_eq!(rects[99].info.packed, true);
}

#[test]
fn test_loop_stability() {
    // Mirrors the C test_rect_pack_loop: packing 199 rects 10 times should always succeed
    let base: Vec<(i32, i32, i32)> = vec![
        (0,255,255),(1,255,253),(2,253,255),(3,255,253),(4,255,253),(5,253,255),
        (6,253,255),(7,253,255),(8,255,251),(9,255,251),(10,251,255),(11,255,251),
        (12,255,251),(13,251,255),(14,255,251),(15,255,251),(16,255,249),(17,255,249),
        (18,255,249),(19,255,249),(20,255,249),(21,249,255),(22,255,249),(23,249,255),
        (24,249,255),(25,247,255),(26,255,247),(27,255,247),(28,255,245),(29,245,255),
        (30,255,245),(31,255,245),(32,255,243),(33,243,255),(34,243,255),(35,255,243),
        (36,243,255),(37,243,255),(38,255,243),(39,255,243),(40,255,241),(41,255,241),
        (42,241,255),(43,241,255),(44,255,241),(45,241,255),(46,255,241),(47,255,239),
        (48,239,255),(49,239,255),(50,239,255),(51,255,239),(52,239,255),(53,237,255),
        (54,255,237),(55,237,255),(56,237,255),(57,255,237),(58,255,237),(59,255,237),
        (60,237,255),(61,237,255),(62,255,237),(63,255,235),(64,235,255),(65,235,255),
        (66,255,235),(67,255,233),(68,233,255),(69,255,233),(70,233,255),(71,255,233),
        (72,233,255),(73,233,255),(74,255,233),(75,231,255),(76,253,253),(77,253,253),
        (78,253,253),(79,251,253),(80,253,251),(81,253,251),(82,251,253),(83,251,253),
        (84,253,249),(85,253,249),(86,249,253),(87,249,253),(88,249,253),(89,249,253),
        (90,253,249),(91,253,249),(92,253,247),(93,247,253),(94,247,253),(95,253,247),
        (96,253,247),(97,253,247),(98,253,247),(99,253,245),(100,253,245),(101,245,253),
        (102,245,253),(103,253,243),(104,243,253),(105,243,253),(106,253,243),(107,253,243),
        (108,253,243),(109,243,253),(110,243,253),(111,241,253),(112,253,241),(113,253,241),
        (114,253,241),(115,241,253),(116,239,253),(117,253,239),(118,239,253),(119,253,239),
        (120,239,253),(121,253,239),(122,253,239),(123,239,253),(124,239,253),(125,237,253),
        (126,237,253),(127,253,237),(128,237,253),(129,253,237),(130,237,253),(131,235,253),
        (132,235,253),(133,235,253),(134,233,253),(135,253,233),(136,233,253),(137,233,253),
        (138,253,233),(139,233,253),(140,253,233),(141,233,253),(142,253,233),(143,233,253),
        (144,253,233),(145,253,233),(146,233,253),(147,231,253),(148,253,231),(149,253,231),
        (150,229,253),(151,229,253),(152,229,253),(153,229,253),(154,251,251),(155,251,251),
        (156,251,251),(157,251,249),(158,249,251),(159,249,251),(160,249,251),(161,247,251),
        (162,251,247),(163,251,247),(164,247,251),(165,247,251),(166,247,251),(167,251,247),
        (168,247,251),(169,251,247),(170,247,251),(171,251,247),(172,251,245),(173,245,251),
        (174,251,245),(175,251,245),(176,245,251),(177,251,245),(178,251,243),(179,243,251),
        (180,243,251),(181,243,251),(182,243,251),(183,243,251),(184,251,243),(185,251,243),
        (186,241,251),(187,251,241),(188,241,251),(189,251,241),(190,241,251),(191,251,241),
        (192,251,239),(193,239,251),(194,251,237),(195,237,251),(196,251,237),(197,251,237),
        (198,251,237),
    ];
    for _ in 0..10 {
        let mut rects: Vec<Rect> = base.iter().map(|&(id,w,h)| Rect::new(id, w, h)).collect();
        assert!(RectPacker::pack(500, 500, true, &mut rects));
    }
}

fn main() {}
