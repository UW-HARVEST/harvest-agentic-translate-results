use rect_pack_h::rect_pack::{Rect, RectPacker};

#[test]
fn test_empty() {
    let mut rects: Vec<Rect> = vec![];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
}

#[test]
fn test_single_rect() {
    let mut rects = vec![Rect::new(0, 5, 5)];
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    assert!(rects[0].info.packed);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.page, 0);
}

#[test]
fn test_exact_fit() {
    let mut rects = vec![Rect::new(0, 10, 10)];
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    assert!(rects[0].info.packed);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
}

#[test]
fn test_too_big_no_paging() {
    let mut rects = vec![Rect::new(0, 20, 20)];
    assert!(!RectPacker::pack(10, 10, false, &mut rects));
    assert!(!rects[0].info.packed);
}

#[test]
fn test_too_big_with_paging() {
    let mut rects = vec![Rect::new(0, 20, 20)];
    assert!(!RectPacker::pack(10, 10, true, &mut rects));
    assert!(!rects[0].info.packed);
}

#[test]
fn test_two_rects_fit() {
    let mut rects = vec![Rect::new(0, 5, 10), Rect::new(1, 5, 10)];
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    assert!(rects[0].info.packed);
    assert!(rects[1].info.packed);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[1].info.x, 5);
    assert_eq!(rects[1].info.y, 0);
}

#[test]
fn test_no_paging() {
    // C ground truth: after sort by max side desc, order is id=3(10x9), id=2(8x9), id=1(8x7), id=0(6x7)
    // Only first rect fits in 10x10, rest don't. Returns false.
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    assert!(!RectPacker::pack(10, 10, false, &mut rects));
    // After sorting: rects[0] is the biggest (id=3), only it is packed
    assert!(rects[0].info.packed);
    assert!(!rects[1].info.packed);
    assert!(!rects[2].info.packed);
    assert!(!rects[3].info.packed);
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
    // After sorting each rect gets its own page
    for r in &rects {
        assert!(r.info.packed);
    }
    // C output: pages 0,1,2,3 in sorted order
    assert_eq!(rects[0].info.page, 0);
    assert_eq!(rects[1].info.page, 1);
    assert_eq!(rects[2].info.page, 2);
    assert_eq!(rects[3].info.page, 3);
}

#[test]
fn test_fill() {
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, 1, 1)).collect();
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    for r in &rects {
        assert!(r.info.packed);
        assert_eq!(r.info.page, 0);
    }
}

#[test]
fn test_fail_paging() {
    // C ground truth: after sort, 1100x1100 comes first (bigger), then 900x900
    // 1100x1100 doesn't fit in 1000x1000 even with paging -> not packed
    // 900x900 fits -> packed on page 0 (but since first rect failed, none_fit triggers break)
    // Wait - C output shows: id=1(1100x1100) packed=0, id=0(900x900) packed=1
    // So after sort: [id=1 1100x1100, id=0 900x900]
    // First rect 1100x1100 can't fit -> root is clamped to 1000x1000, still 1100>1000
    // But 900x900 fits in the 1000x1000 root. So none_fit is false, all_fit is false.
    // With paging, it continues to next page. But 1100x1100 still can't fit on any page.
    // Eventually none_fit=true on a page with only 1100x1100 left -> breaks.
    // Result: false (not all packed)
    let mut rects = vec![Rect::new(0, 900, 900), Rect::new(1, 1100, 1100)];
    assert!(!RectPacker::pack(1000, 1000, true, &mut rects));
    // After sort: rects[0] is 1100x1100 (id=1), rects[1] is 900x900 (id=0)
    assert!(!rects[0].info.packed); // 1100x1100 too big
    assert!(rects[1].info.packed);  // 900x900 fits
}

#[test]
fn test_uniform_paging() {
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, i + 1, i + 1)).collect();
    assert!(RectPacker::pack(512, 512, true, &mut rects));
    // All rects should be packed
    for r in &rects {
        assert!(r.info.packed);
    }
}

#[test]
fn test_sort_order() {
    // C ground truth: sorted by max(w,h) desc, then min(w,h) desc
    // Input: id=0(2x3), id=1(5x1), id=2(4x4), id=3(1x5)
    // max sides: 3, 5, 4, 5 -> sorted: id=1(5,1), id=3(1,5), id=2(4,4), id=0(2,3)
    // id=1 and id=3 both have max=5, min side: 1 vs 1 -> equal, stable order from qsort
    // C output: id=1 first, id=3 second, id=2 third, id=0 fourth
    let mut rects = vec![
        Rect::new(0, 2, 3),
        Rect::new(1, 5, 1),
        Rect::new(2, 4, 4),
        Rect::new(3, 1, 5),
    ];
    assert!(RectPacker::pack(100, 100, false, &mut rects));
    // All should be packed
    for r in &rects {
        assert!(r.info.packed);
    }
    // Verify sort order by checking ids
    assert_eq!(rects[0].id, 1); // max=5, min=1
    assert_eq!(rects[1].id, 3); // max=5, min=1
    assert_eq!(rects[2].id, 2); // max=4, min=4
    assert_eq!(rects[3].id, 0); // max=3, min=2
}

#[test]
fn test_loop() {
    let mut rects = vec![
        Rect::new(0, 255, 255), Rect::new(1, 255, 253), Rect::new(2, 253, 255),
        Rect::new(3, 255, 253), Rect::new(4, 255, 253), Rect::new(5, 253, 255),
        Rect::new(6, 253, 255), Rect::new(7, 253, 255), Rect::new(8, 255, 251),
        Rect::new(9, 255, 251), Rect::new(10, 251, 255), Rect::new(11, 255, 251),
        Rect::new(12, 255, 251), Rect::new(13, 251, 255), Rect::new(14, 255, 251),
        Rect::new(15, 255, 251), Rect::new(16, 255, 249), Rect::new(17, 255, 249),
        Rect::new(18, 255, 249), Rect::new(19, 255, 249), Rect::new(20, 255, 249),
        Rect::new(21, 249, 255), Rect::new(22, 255, 249), Rect::new(23, 249, 255),
        Rect::new(24, 249, 255), Rect::new(25, 247, 255), Rect::new(26, 255, 247),
        Rect::new(27, 255, 247), Rect::new(28, 255, 245), Rect::new(29, 245, 255),
        Rect::new(30, 255, 245), Rect::new(31, 255, 245), Rect::new(32, 255, 243),
        Rect::new(33, 243, 255), Rect::new(34, 243, 255), Rect::new(35, 255, 243),
        Rect::new(36, 243, 255), Rect::new(37, 243, 255), Rect::new(38, 255, 243),
        Rect::new(39, 255, 243), Rect::new(40, 255, 241), Rect::new(41, 255, 241),
        Rect::new(42, 241, 255), Rect::new(43, 241, 255), Rect::new(44, 255, 241),
        Rect::new(45, 241, 255), Rect::new(46, 255, 241), Rect::new(47, 255, 239),
        Rect::new(48, 239, 255), Rect::new(49, 239, 255), Rect::new(50, 239, 255),
        Rect::new(51, 255, 239), Rect::new(52, 239, 255), Rect::new(53, 237, 255),
        Rect::new(54, 255, 237), Rect::new(55, 237, 255), Rect::new(56, 237, 255),
        Rect::new(57, 255, 237), Rect::new(58, 255, 237), Rect::new(59, 255, 237),
        Rect::new(60, 237, 255), Rect::new(61, 237, 255), Rect::new(62, 255, 237),
        Rect::new(63, 255, 235), Rect::new(64, 235, 255), Rect::new(65, 235, 255),
        Rect::new(66, 255, 235), Rect::new(67, 255, 233), Rect::new(68, 233, 255),
        Rect::new(69, 255, 233), Rect::new(70, 233, 255), Rect::new(71, 255, 233),
        Rect::new(72, 233, 255), Rect::new(73, 233, 255), Rect::new(74, 255, 233),
        Rect::new(75, 231, 255), Rect::new(76, 253, 253), Rect::new(77, 253, 253),
        Rect::new(78, 253, 253), Rect::new(79, 251, 253), Rect::new(80, 253, 251),
        Rect::new(81, 253, 251), Rect::new(82, 251, 253), Rect::new(83, 251, 253),
        Rect::new(84, 253, 249), Rect::new(85, 253, 249), Rect::new(86, 249, 253),
        Rect::new(87, 249, 253), Rect::new(88, 249, 253), Rect::new(89, 249, 253),
        Rect::new(90, 253, 249), Rect::new(91, 253, 249), Rect::new(92, 253, 247),
        Rect::new(93, 247, 253), Rect::new(94, 247, 253), Rect::new(95, 253, 247),
        Rect::new(96, 253, 247), Rect::new(97, 253, 247), Rect::new(98, 253, 247),
        Rect::new(99, 253, 245), Rect::new(100, 253, 245), Rect::new(101, 245, 253),
        Rect::new(102, 245, 253), Rect::new(103, 253, 243), Rect::new(104, 243, 253),
        Rect::new(105, 243, 253), Rect::new(106, 253, 243), Rect::new(107, 253, 243),
        Rect::new(108, 253, 243), Rect::new(109, 243, 253), Rect::new(110, 243, 253),
        Rect::new(111, 241, 253), Rect::new(112, 253, 241), Rect::new(113, 253, 241),
        Rect::new(114, 253, 241), Rect::new(115, 241, 253), Rect::new(116, 239, 253),
        Rect::new(117, 253, 239), Rect::new(118, 239, 253), Rect::new(119, 253, 239),
        Rect::new(120, 239, 253), Rect::new(121, 253, 239), Rect::new(122, 253, 239),
        Rect::new(123, 239, 253), Rect::new(124, 239, 253), Rect::new(125, 237, 253),
        Rect::new(126, 237, 253), Rect::new(127, 253, 237), Rect::new(128, 237, 253),
        Rect::new(129, 253, 237), Rect::new(130, 237, 253), Rect::new(131, 235, 253),
        Rect::new(132, 235, 253), Rect::new(133, 235, 253), Rect::new(134, 233, 253),
        Rect::new(135, 253, 233), Rect::new(136, 233, 253), Rect::new(137, 233, 253),
        Rect::new(138, 253, 233), Rect::new(139, 233, 253), Rect::new(140, 253, 233),
        Rect::new(141, 233, 253), Rect::new(142, 253, 233), Rect::new(143, 233, 253),
        Rect::new(144, 253, 233), Rect::new(145, 253, 233), Rect::new(146, 233, 253),
        Rect::new(147, 231, 253), Rect::new(148, 253, 231), Rect::new(149, 253, 231),
        Rect::new(150, 229, 253), Rect::new(151, 229, 253), Rect::new(152, 229, 253),
        Rect::new(153, 229, 253), Rect::new(154, 251, 251), Rect::new(155, 251, 251),
        Rect::new(156, 251, 251), Rect::new(157, 251, 249), Rect::new(158, 249, 251),
        Rect::new(159, 249, 251), Rect::new(160, 249, 251), Rect::new(161, 247, 251),
        Rect::new(162, 251, 247), Rect::new(163, 251, 247), Rect::new(164, 247, 251),
        Rect::new(165, 247, 251), Rect::new(166, 247, 251), Rect::new(167, 251, 247),
        Rect::new(168, 247, 251), Rect::new(169, 251, 247), Rect::new(170, 247, 251),
        Rect::new(171, 251, 247), Rect::new(172, 251, 245), Rect::new(173, 245, 251),
        Rect::new(174, 251, 245), Rect::new(175, 251, 245), Rect::new(176, 245, 251),
        Rect::new(177, 251, 245), Rect::new(178, 251, 243), Rect::new(179, 243, 251),
        Rect::new(180, 243, 251), Rect::new(181, 243, 251), Rect::new(182, 243, 251),
        Rect::new(183, 243, 251), Rect::new(184, 251, 243), Rect::new(185, 251, 243),
        Rect::new(186, 241, 251), Rect::new(187, 251, 241), Rect::new(188, 241, 251),
        Rect::new(189, 251, 241), Rect::new(190, 241, 251), Rect::new(191, 251, 241),
        Rect::new(192, 251, 239), Rect::new(193, 239, 251), Rect::new(194, 251, 237),
        Rect::new(195, 237, 251), Rect::new(196, 251, 237), Rect::new(197, 251, 237),
        Rect::new(198, 251, 237),
    ];
    for _ in 0..10 {
        assert!(RectPacker::pack(500, 500, true, &mut rects));
    }
    // After 10 iterations, all rects should still be packed
    for r in &rects {
        assert!(r.info.packed);
    }
}

#[test]
fn test_fill_exact_positions() {
    // Verify specific positions from C ground truth for 1x1 rects in 10x10
    let mut rects: Vec<Rect> = (0..100).map(|i| Rect::new(i, 1, 1)).collect();
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    // C output: id=0 at (0,0), id=1 at (1,0), id=2 at (0,1), id=3 at (1,1), id=4 at (2,0)
    assert_eq!((rects[0].info.x, rects[0].info.y), (0, 0));
    assert_eq!((rects[1].info.x, rects[1].info.y), (1, 0));
    assert_eq!((rects[2].info.x, rects[2].info.y), (0, 1));
    assert_eq!((rects[3].info.x, rects[3].info.y), (1, 1));
    assert_eq!((rects[4].info.x, rects[4].info.y), (2, 0));
    // Last rect id=99 at (9,9)
    assert_eq!((rects[99].info.x, rects[99].info.y), (9, 9));
}

#[test]
fn test_no_paging_sort_and_positions() {
    // Verify the sort reorders rects and only first fits
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    assert!(!RectPacker::pack(10, 10, false, &mut rects));
    // After sort: id=3(10x9) is first
    assert_eq!(rects[0].id, 3);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert!(rects[0].info.packed);
    // id=2(8x9) second
    assert_eq!(rects[1].id, 2);
    assert!(!rects[1].info.packed);
    // id=1(8x7) third
    assert_eq!(rects[2].id, 1);
    assert!(!rects[2].info.packed);
    // id=0(6x7) fourth
    assert_eq!(rects[3].id, 0);
    assert!(!rects[3].info.packed);
}

#[test]
fn test_fail_paging_sort_order() {
    let mut rects = vec![Rect::new(0, 900, 900), Rect::new(1, 1100, 1100)];
    assert!(!RectPacker::pack(1000, 1000, true, &mut rects));
    // After sort: 1100x1100 (id=1) first, 900x900 (id=0) second
    assert_eq!(rects[0].id, 1);
    assert_eq!(rects[1].id, 0);
    assert!(!rects[0].info.packed);
    assert!(rects[1].info.packed);
    assert_eq!(rects[1].info.page, 0);
}

#[test]
fn test_one_rect_per_page_positions() {
    let mut rects = vec![
        Rect::new(0, 6, 7),
        Rect::new(1, 8, 7),
        Rect::new(2, 8, 9),
        Rect::new(3, 10, 9),
    ];
    assert!(RectPacker::pack(10, 10, true, &mut rects));
    // Each rect starts at (0,0) on its own page
    for r in &rects {
        assert_eq!(r.info.x, 0);
        assert_eq!(r.info.y, 0);
    }
}

#[test]
fn test_info_reset_between_calls() {
    // Calling pack should reset info fields
    let mut rects = vec![Rect::new(0, 5, 5)];
    assert!(RectPacker::pack(10, 10, false, &mut rects));
    assert!(rects[0].info.packed);
    // Pack again with impossible size
    rects[0].w = 20;
    rects[0].h = 20;
    assert!(!RectPacker::pack(10, 10, false, &mut rects));
    assert!(!rects[0].info.packed);
    assert_eq!(rects[0].info.x, 0);
    assert_eq!(rects[0].info.y, 0);
    assert_eq!(rects[0].info.page, 0);
}

fn main() {}
