use std::cmp::Ordering;
/// Output information for a packed rectangle
#[derive(Debug, Clone, Copy)]
pub struct RectOutInfo {
    /// X coordinate in the packed layout
    pub x: i32,
    /// Y coordinate in the packed layout
    pub y: i32,
    /// Whether the rectangle was successfully packed
    pub packed: bool,
    /// The page number where the rectangle was packed (for multi-page packing)
    pub page: i32,
}
impl Default for RectOutInfo {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            packed: false,
            page: 0,
        }
    }
}
/// A rectangle to be packed
#[derive(Debug, Clone)]
pub struct Rect {
    /// Unique identifier for the rectangle
    pub id: i32,
    /// Width of the rectangle
    pub w: i32,
    /// Height of the rectangle
    pub h: i32,
    /// Output information after packing
    pub info: RectOutInfo,
}
impl Rect {
    /// Create a new rectangle with the given dimensions
    pub fn new(id: i32, width: i32, height: i32) -> Self {
        Self {
            id,
            w: width,
            h: height,
            info: RectOutInfo::default(),
        }
    }
}
/// Rectangle packer using a binary tree algorithm
pub struct RectPacker;
impl RectPacker {
    /// Pack rectangles into a bin of the given maximum dimensions
    ///
    /// # Arguments
    ///
    /// * `max_w` - Maximum width of the packing area
    /// * `max_h` - Maximum height of the packing area
    /// * `paging` - Whether to allow multiple pages for packing
    /// * `rects` - Mutable slice of rectangles to pack
    ///
    /// # Returns
    ///
    /// `true` if all rectangles were successfully packed, `false` otherwise
    pub fn pack(max_w: i32, max_h: i32, paging: bool, rects: &mut [Rect]) -> bool {
        if rects.is_empty() {
            return true;
        }

        // Sort by max side descending; ties broken by min side descending.
        rects.sort_by(|a, b| {
            let max_a = a.w.max(a.h);
            let max_b = b.w.max(b.h);
            match max_b.cmp(&max_a) {
                Ordering::Equal => {
                    let min_a = a.w.min(a.h);
                    let min_b = b.w.min(b.h);
                    min_b.cmp(&min_a)
                }
                ord => ord,
            }
        });

        // Reset packing info for every rect.
        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let mut next: usize = 0;
        let mut last: usize = rects.len() - 1;
        let mut page: i32 = 0;
        let mut all_packed = false;

        loop {
            let (all_fit, none_fit, new_next, new_last) =
                pack_bin_tree_impl(max_w, max_h, page, rects, next, last);
            next = new_next;
            last = new_last;

            if all_fit {
                all_packed = true;
            }

            if all_fit || !paging || none_fit {
                break;
            }

            page += 1;
        }

        all_packed
    }
}
/// A node in the binary tree used for rectangle packing
struct BinNode {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    used: bool,
    right: Option<Box<BinNode>>,
    down: Option<Box<BinNode>>,
}
impl BinNode {
    /// Create a new bin node with the given dimensions
    fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
            used: false,
            right: None,
            down: None,
        }
    }
    /// Find a node in the tree that can fit a rectangle of the given dimensions
    fn find(&self, w: i32, h: i32) -> Option<&BinNode> {
        if self.used {
            if let Some(right) = self.right.as_ref() {
                if let Some(found) = right.find(w, h) {
                    return Some(found);
                }
            }
            self.down.as_ref().and_then(|d| d.find(w, h))
        } else if w <= self.w && h <= self.h {
            Some(self)
        } else {
            None
        }
    }
    /// Split this node after placing a rectangle of the given dimensions
    fn split(&mut self, w: i32, h: i32) -> &Self {
        self.used = true;
        self.down = Some(Box::new(BinNode::new(
            self.x,
            self.y + h,
            self.w,
            self.h - h,
        )));
        self.right = Some(Box::new(BinNode::new(
            self.x + w,
            self.y,
            self.w - w,
            h,
        )));
        self
    }
    /// Grow the bin to the right to accommodate a rectangle
    fn grow_right(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let old = std::mem::replace(self, BinNode::new(0, 0, 0, 0));
        let old_w = old.w;
        let old_h = old.h;
        self.x = 0;
        self.y = 0;
        self.w = old_w + rect.w;
        self.h = old_h;
        self.used = true;
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));
        self.down = Some(Box::new(old));

        find_and_split_ref(self, rect.w, rect.h)
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let old = std::mem::replace(self, BinNode::new(0, 0, 0, 0));
        let old_w = old.w;
        let old_h = old.h;
        self.x = 0;
        self.y = 0;
        self.w = old_w;
        self.h = old_h + rect.h;
        self.used = true;
        self.down = Some(Box::new(BinNode::new(0, old_h, old_w, rect.h)));
        self.right = Some(Box::new(old));

        find_and_split_ref(self, rect.w, rect.h)
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let can_grow_down = (rect.w <= self.w) && ((rect.h + self.h) <= max_h);
        let can_grow_right = (rect.h <= self.h) && ((rect.w + self.w) <= max_w);

        let should_grow_right = can_grow_right && (self.h >= (self.w + rect.w));
        let should_grow_down = can_grow_down && (self.w >= (self.h + rect.h));

        if should_grow_right {
            return self.grow_right(rect, max_w, max_h);
        } else if should_grow_down {
            return self.grow_down(rect, max_w, max_h);
        }

        if can_grow_right {
            self.grow_right(rect, max_w, max_h)
        } else if can_grow_down {
            self.grow_down(rect, max_w, max_h)
        } else {
            None
        }
    }
}

/// Internal helper: recursively traverse the bin tree, locate an unused node
/// that can fit the given dimensions, split it, and return the placement
/// coordinates.
fn find_and_split(node: &mut BinNode, w: i32, h: i32) -> Option<(i32, i32)> {
    if node.used {
        if let Some(right) = node.right.as_mut() {
            let result = find_and_split(right, w, h);
            if result.is_some() {
                return result;
            }
        }
        if let Some(down) = node.down.as_mut() {
            return find_and_split(down, w, h);
        }
        None
    } else if w <= node.w && h <= node.h {
        let pos = (node.x, node.y);
        node.split(w, h);
        Some(pos)
    } else {
        None
    }
}

/// Internal helper used by the public `grow_right`/`grow_down`/`grow`
/// methods. Like `find_and_split`, but returns a borrow of the node that
/// was split so callers can read its position fields.
fn find_and_split_ref<'a>(node: &'a mut BinNode, w: i32, h: i32) -> Option<&'a BinNode> {
    if node.used {
        let go_right = node
            .right
            .as_ref()
            .map_or(false, |r| r.find(w, h).is_some());
        if go_right {
            let right = node.right.as_mut().unwrap();
            return find_and_split_ref(right, w, h);
        }
        let go_down = node
            .down
            .as_ref()
            .map_or(false, |d| d.find(w, h).is_some());
        if go_down {
            let down = node.down.as_mut().unwrap();
            return find_and_split_ref(down, w, h);
        }
        None
    } else if w <= node.w && h <= node.h {
        node.split(w, h);
        Some(&*node)
    } else {
        None
    }
}

/// Internal helper that decides whether to grow right or down (mirrors the
/// C `grow_bin_tree`) and returns the placement coordinates.
fn grow_bin_tree(
    root: &mut BinNode,
    rect: &Rect,
    max_w: i32,
    max_h: i32,
) -> Option<(i32, i32)> {
    let can_grow_down = (rect.w <= root.w) && ((rect.h + root.h) <= max_h);
    let can_grow_right = (rect.h <= root.h) && ((rect.w + root.w) <= max_w);

    let should_grow_right = can_grow_right && (root.h >= (root.w + rect.w));
    let should_grow_down = can_grow_down && (root.w >= (root.h + rect.h));

    if should_grow_right {
        return grow_right_impl(root, rect);
    } else if should_grow_down {
        return grow_down_impl(root, rect);
    }

    if can_grow_right {
        grow_right_impl(root, rect)
    } else if can_grow_down {
        grow_down_impl(root, rect)
    } else {
        None
    }
}

fn grow_right_impl(root: &mut BinNode, rect: &Rect) -> Option<(i32, i32)> {
    let old = std::mem::replace(root, BinNode::new(0, 0, 0, 0));
    let old_w = old.w;
    let old_h = old.h;
    root.x = 0;
    root.y = 0;
    root.w = old_w + rect.w;
    root.h = old_h;
    root.used = true;
    root.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));
    root.down = Some(Box::new(old));

    find_and_split(root, rect.w, rect.h)
}

fn grow_down_impl(root: &mut BinNode, rect: &Rect) -> Option<(i32, i32)> {
    let old = std::mem::replace(root, BinNode::new(0, 0, 0, 0));
    let old_w = old.w;
    let old_h = old.h;
    root.x = 0;
    root.y = 0;
    root.w = old_w;
    root.h = old_h + rect.h;
    root.used = true;
    root.down = Some(Box::new(BinNode::new(0, old_h, old_w, rect.h)));
    root.right = Some(Box::new(old));

    find_and_split(root, rect.w, rect.h)
}

/// Mirror of the C `pack_bin_tree`: pack as many rectangles as possible on
/// a single page. Returns `(all_fit, none_fit, new_next, new_last)` where
/// `new_next` is the first non-contiguously-packed index and `new_last`
/// is the highest index that failed to pack on this page.
fn pack_bin_tree_impl(
    max_w: i32,
    max_h: i32,
    page: i32,
    rects: &mut [Rect],
    initial_next: usize,
    initial_last: usize,
) -> (bool, bool, usize, usize) {
    let mut all_fit = true;
    let mut none_fit = true;

    let root_w = rects[initial_next].w.min(max_w);
    let root_h = rects[initial_next].h.min(max_h);

    let mut root = BinNode::new(0, 0, root_w, root_h);
    let mut contiguous = true;

    let mut next = initial_next;
    let mut last = initial_last;
    let original_last = initial_last;

    let mut i = initial_next;
    while i <= original_last && i < rects.len() {
        if !rects[i].info.packed {
            let pos = match find_and_split(&mut root, rects[i].w, rects[i].h) {
                Some(p) => Some(p),
                None => {
                    let rect_clone = rects[i].clone();
                    grow_bin_tree(&mut root, &rect_clone, max_w, max_h)
                }
            };

            if let Some((x, y)) = pos {
                rects[i].info.x = x;
                rects[i].info.y = y;
                rects[i].info.packed = true;
                rects[i].info.page = page;
                none_fit = false;
            } else {
                rects[i].info.packed = false;
                all_fit = false;
                contiguous = false;
                last = i;
            }
        }

        if contiguous {
            next = i + 1;
        }

        i += 1;
    }

    (all_fit, none_fit, next, last)
}
