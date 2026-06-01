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

        // Sort rectangles by max_side desc, with min_side desc as a tiebreaker
        // (mirrors the C compare_rect_r_max_side)
        rects.sort_by(|a, b| {
            let max_a = a.w.max(a.h);
            let max_b = b.w.max(b.h);
            let diff = max_b - max_a;
            if diff == 0 {
                let min_a = a.w.min(a.h);
                let min_b = b.w.min(b.h);
                if min_b > min_a {
                    Ordering::Greater
                } else if min_b < min_a {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            } else if diff > 0 {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        });

        // Reset all info to defaults
        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let n = rects.len();
        let mut next: usize = 0;
        let mut last: usize = n - 1;
        let mut page: i32 = 0;
        let mut all_packed = false;
        let mut ok = false;

        while !ok {
            let res = pack_bin_tree(rects, &mut next, &mut last, max_w, max_h, page);
            ok = res.all_fit;
            all_packed = all_packed || ok;

            if !paging || res.none_fit {
                break;
            }

            page += 1;
        }

        all_packed
    }
}

/// Result of attempting to pack rectangles into a single page.
struct PackRes {
    all_fit: bool,
    none_fit: bool,
}

/// Pack rectangles into a single page using the bin-tree algorithm.
///
/// Mirrors the C `pack_bin_tree` function exactly.
fn pack_bin_tree(
    rects: &mut [Rect],
    next: &mut usize,
    last: &mut usize,
    max_w: i32,
    max_h: i32,
    page: i32,
) -> PackRes {
    let mut res = PackRes {
        all_fit: true,
        none_fit: true,
    };

    let start = *next;
    let end = *last;

    let root_w = if rects[start].w <= max_w {
        rects[start].w
    } else {
        max_w
    };
    let root_h = if rects[start].h <= max_h {
        rects[start].h
    } else {
        max_h
    };

    let mut root = BinNode::new(0, 0, root_w, root_h);

    let mut contiguous = true;
    let mut new_last = end;

    let mut i = start;
    while i <= end {
        if !rects[i].info.packed {
            let rw = rects[i].w;
            let rh = rects[i].h;

            // Try to find a node big enough to fit and split it
            let placed = match find_and_split(&mut root, rw, rh) {
                Some(pos) => Some(pos),
                None => grow_root(&mut root, rw, rh, max_w, max_h),
            };

            if let Some((px, py)) = placed {
                rects[i].info.x = px;
                rects[i].info.y = py;
                rects[i].info.packed = true;
                rects[i].info.page = page;
                res.none_fit = false;
            } else {
                rects[i].info.packed = false;
                res.all_fit = false;
                contiguous = false;
                new_last = i;
            }
        }

        if contiguous {
            *next = i + 1;
        }
        i += 1;
    }

    *last = new_last;
    res
}

/// Find a free node in the bin tree large enough for `(w, h)`, then split it.
/// Returns the (x, y) coordinate of the placed rectangle, or None if no fit.
fn find_and_split(node: &mut BinNode, w: i32, h: i32) -> Option<(i32, i32)> {
    if node.used {
        if let Some(r) = node.right.as_deref_mut() {
            if let Some(pos) = find_and_split(r, w, h) {
                return Some(pos);
            }
        }
        if let Some(d) = node.down.as_deref_mut() {
            if let Some(pos) = find_and_split(d, w, h) {
                return Some(pos);
            }
        }
        None
    } else if w <= node.w && h <= node.h {
        let pos = (node.x, node.y);
        node.used = true;
        node.down = Some(Box::new(BinNode::new(
            node.x,
            node.y + h,
            node.w,
            node.h - h,
        )));
        node.right = Some(Box::new(BinNode::new(
            node.x + w,
            node.y,
            node.w - w,
            h,
        )));
        Some(pos)
    } else {
        None
    }
}

/// Grow the root rightward to fit a rectangle of `(rect_w, rect_h)`.
/// Returns the (x, y) coordinate where the rectangle was placed.
fn grow_right_root(root: &mut BinNode, rect_w: i32, rect_h: i32) -> Option<(i32, i32)> {
    let old = std::mem::replace(root, BinNode::new(0, 0, 0, 0));
    let old_w = old.w;
    let old_h = old.h;
    *root = BinNode {
        x: 0,
        y: 0,
        w: old_w + rect_w,
        h: old_h,
        used: true,
        right: Some(Box::new(BinNode::new(old_w, 0, rect_w, old_h))),
        down: Some(Box::new(old)),
    };
    find_and_split(root, rect_w, rect_h)
}

/// Grow the root downward to fit a rectangle of `(rect_w, rect_h)`.
fn grow_down_root(root: &mut BinNode, rect_w: i32, rect_h: i32) -> Option<(i32, i32)> {
    let old = std::mem::replace(root, BinNode::new(0, 0, 0, 0));
    let old_w = old.w;
    let old_h = old.h;
    *root = BinNode {
        x: 0,
        y: 0,
        w: old_w,
        h: old_h + rect_h,
        used: true,
        right: Some(Box::new(old)),
        down: Some(Box::new(BinNode::new(0, old_h, old_w, rect_h))),
    };
    find_and_split(root, rect_w, rect_h)
}

/// Grow the root in the optimal direction to fit a rectangle.
fn grow_root(
    root: &mut BinNode,
    rect_w: i32,
    rect_h: i32,
    max_w: i32,
    max_h: i32,
) -> Option<(i32, i32)> {
    let can_grow_down = rect_w <= root.w && (rect_h + root.h) <= max_h;
    let can_grow_right = rect_h <= root.h && (rect_w + root.w) <= max_w;

    let should_grow_right = can_grow_right && (root.h >= (root.w + rect_w));
    let should_grow_down = can_grow_down && (root.w >= (root.h + rect_h));

    if should_grow_right {
        grow_right_root(root, rect_w, rect_h)
    } else if should_grow_down {
        grow_down_root(root, rect_w, rect_h)
    } else if can_grow_right {
        grow_right_root(root, rect_w, rect_h)
    } else if can_grow_down {
        grow_down_root(root, rect_w, rect_h)
    } else {
        None
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
            if let Some(r) = self.right.as_deref() {
                if let Some(found) = r.find(w, h) {
                    return Some(found);
                }
            }
            if let Some(d) = self.down.as_deref() {
                return d.find(w, h);
            }
            None
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
        let pos = grow_right_root(self, rect.w, rect.h)?;
        self.find_used_at(pos.0, pos.1)
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let pos = grow_down_root(self, rect.w, rect.h)?;
        self.find_used_at(pos.0, pos.1)
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let pos = grow_root(self, rect.w, rect.h, max_w, max_h)?;
        self.find_used_at(pos.0, pos.1)
    }

    /// Recursively look up a `used` node at the given absolute coordinates.
    fn find_used_at(&self, x: i32, y: i32) -> Option<&BinNode> {
        if self.used && self.x == x && self.y == y {
            return Some(self);
        }
        if let Some(r) = self.right.as_deref() {
            if let Some(n) = r.find_used_at(x, y) {
                return Some(n);
            }
        }
        if let Some(d) = self.down.as_deref() {
            if let Some(n) = d.find_used_at(x, y) {
                return Some(n);
            }
        }
        None
    }
}
