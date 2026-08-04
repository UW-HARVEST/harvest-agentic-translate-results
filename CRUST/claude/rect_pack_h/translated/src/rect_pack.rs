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

        // Sort by max side descending, then by min side descending (matches qsort comparator)
        rects.sort_by(|a, b| {
            let a_max = a.w.max(a.h);
            let b_max = b.w.max(b.h);
            match b_max.cmp(&a_max) {
                Ordering::Equal => {
                    let a_min = a.w.min(a.h);
                    let b_min = b.w.min(b.h);
                    b_min.cmp(&a_min)
                }
                ord => ord,
            }
        });

        // Reset packing info
        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let mut next: usize = 0;
        let mut last: usize = rects.len() - 1;
        let mut page: i32 = 0;
        let mut all_packed = false;

        loop {
            let (all_fit, none_fit) =
                pack_bin_tree(rects, max_w, max_h, page, &mut next, &mut last);
            all_packed = all_packed || all_fit;

            if all_fit {
                break;
            }

            if !paging || none_fit {
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
        let old_x = self.x;
        let old_y = self.y;
        let old_w = self.w;
        let old_h = self.h;
        let old_used = self.used;
        let old_right = self.right.take();
        let old_down = self.down.take();

        let old = BinNode {
            x: old_x,
            y: old_y,
            w: old_w,
            h: old_h,
            used: old_used,
            right: old_right,
            down: old_down,
        };

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w + rect.w;
        self.h = old_h;
        self.down = Some(Box::new(old));
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

        find_and_split_ref(self, rect.w, rect.h)
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let old_x = self.x;
        let old_y = self.y;
        let old_w = self.w;
        let old_h = self.h;
        let old_used = self.used;
        let old_right = self.right.take();
        let old_down = self.down.take();

        let old = BinNode {
            x: old_x,
            y: old_y,
            w: old_w,
            h: old_h,
            used: old_used,
            right: old_right,
            down: old_down,
        };

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w;
        self.h = old_h + rect.h;
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
            return self.grow_right(rect, max_w, max_h);
        } else if can_grow_down {
            return self.grow_down(rect, max_w, max_h);
        }

        None
    }
}

/// Returns true iff the subtree rooted at `node` contains an unused node that
/// can fit a rectangle of the given dimensions.
fn has_fit(node: &BinNode, w: i32, h: i32) -> bool {
    if node.used {
        if let Some(r) = node.right.as_deref() {
            if has_fit(r, w, h) {
                return true;
            }
        }
        if let Some(d) = node.down.as_deref() {
            return has_fit(d, w, h);
        }
        false
    } else {
        w <= node.w && h <= node.h
    }
}

/// Mutable variant of `find` used for in-place mutation. Mirrors the C
/// `find_bin_tree` traversal: when used, try the `right` subtree first then
/// `down`. Uses an immutable pre-check to avoid borrow-checker issues with
/// mutable conditional returns.
fn find_mut<'a>(node: &'a mut BinNode, w: i32, h: i32) -> Option<&'a mut BinNode> {
    if !node.used {
        if w <= node.w && h <= node.h {
            return Some(node);
        }
        return None;
    }

    let go_right = node
        .right
        .as_deref()
        .map(|r| has_fit(r, w, h))
        .unwrap_or(false);

    if go_right {
        let r = node.right.as_deref_mut().expect("right exists when go_right");
        return find_mut(r, w, h);
    }
    if let Some(d) = node.down.as_deref_mut() {
        return find_mut(d, w, h);
    }
    None
}

/// Splits the given node in place (matches `split_bin_tree`).
fn split_in_place(node: &mut BinNode, w: i32, h: i32) {
    let nx = node.x;
    let ny = node.y;
    let nw = node.w;
    let nh = node.h;
    node.used = true;
    node.down = Some(Box::new(BinNode::new(nx, ny + h, nw, nh - h)));
    node.right = Some(Box::new(BinNode::new(nx + w, ny, nw - w, h)));
}

/// Finds a node that fits, splits it in place, and returns its (x, y).
fn find_and_split(root: &mut BinNode, w: i32, h: i32) -> Option<(i32, i32)> {
    let node = find_mut(root, w, h)?;
    let x = node.x;
    let y = node.y;
    split_in_place(node, w, h);
    Some((x, y))
}

/// Same as `find_and_split` but returns an immutable reference to the
/// (now-used) split node. Used by the `BinNode::grow_*` trait methods.
fn find_and_split_ref(root: &mut BinNode, w: i32, h: i32) -> Option<&BinNode> {
    let node = find_mut(root, w, h)?;
    split_in_place(node, w, h);
    Some(&*node)
}

/// Grows the root to the right. Mirrors C's `grow_right`.
fn grow_right_root(root: &mut BinNode, rw: i32, rh: i32) -> Option<(i32, i32)> {
    let old_x = root.x;
    let old_y = root.y;
    let old_w = root.w;
    let old_h = root.h;
    let old_used = root.used;
    let old_right = root.right.take();
    let old_down = root.down.take();

    let old = BinNode {
        x: old_x,
        y: old_y,
        w: old_w,
        h: old_h,
        used: old_used,
        right: old_right,
        down: old_down,
    };

    root.used = true;
    root.x = 0;
    root.y = 0;
    root.w = old_w + rw;
    root.h = old_h;
    root.down = Some(Box::new(old));
    root.right = Some(Box::new(BinNode::new(old_w, 0, rw, old_h)));

    find_and_split(root, rw, rh)
}

/// Grows the root downward. Mirrors C's `grow_down`.
fn grow_down_root(root: &mut BinNode, rw: i32, rh: i32) -> Option<(i32, i32)> {
    let old_x = root.x;
    let old_y = root.y;
    let old_w = root.w;
    let old_h = root.h;
    let old_used = root.used;
    let old_right = root.right.take();
    let old_down = root.down.take();

    let old = BinNode {
        x: old_x,
        y: old_y,
        w: old_w,
        h: old_h,
        used: old_used,
        right: old_right,
        down: old_down,
    };

    root.used = true;
    root.x = 0;
    root.y = 0;
    root.w = old_w;
    root.h = old_h + rh;
    root.down = Some(Box::new(BinNode::new(0, old_h, old_w, rh)));
    root.right = Some(Box::new(old));

    find_and_split(root, rw, rh)
}

/// Mirrors C's `grow_bin_tree`.
fn grow_bin_tree(
    root: &mut BinNode,
    rw: i32,
    rh: i32,
    max_w: i32,
    max_h: i32,
) -> Option<(i32, i32)> {
    let can_grow_down = (rw <= root.w) && ((rh + root.h) <= max_h);
    let can_grow_right = (rh <= root.h) && ((rw + root.w) <= max_w);

    let should_grow_right = can_grow_right && (root.h >= (root.w + rw));
    let should_grow_down = can_grow_down && (root.w >= (root.h + rh));

    if should_grow_right {
        return grow_right_root(root, rw, rh);
    } else if should_grow_down {
        return grow_down_root(root, rw, rh);
    }

    if can_grow_right {
        return grow_right_root(root, rw, rh);
    } else if can_grow_down {
        return grow_down_root(root, rw, rh);
    }

    None
}

/// Mirrors C's `pack_bin_tree`. Returns (all_fit, none_fit).
fn pack_bin_tree(
    rects: &mut [Rect],
    max_w: i32,
    max_h: i32,
    page: i32,
    next: &mut usize,
    last: &mut usize,
) -> (bool, bool) {
    let mut all_fit = true;
    let mut none_fit = true;

    let next_idx = *next;
    let last_idx = *last;

    if next_idx > last_idx {
        // Nothing to do; treat as success.
        return (true, true);
    }

    let root_w = rects[next_idx].w.min(max_w);
    let root_h = rects[next_idx].h.min(max_h);

    let mut root = BinNode::new(0, 0, root_w, root_h);

    let mut contiguous = true;
    let mut new_last = last_idx;

    for i in next_idx..=last_idx {
        if !rects[i].info.packed {
            let rw = rects[i].w;
            let rh = rects[i].h;

            let mut placed: Option<(i32, i32)> = find_and_split(&mut root, rw, rh);

            if placed.is_none() {
                placed = grow_bin_tree(&mut root, rw, rh, max_w, max_h);
            }

            if let Some((x, y)) = placed {
                rects[i].info.x = x;
                rects[i].info.y = y;
                rects[i].info.packed = true;
                rects[i].info.page = page;
                none_fit = false;
            } else {
                rects[i].info.packed = false;
                all_fit = false;
                contiguous = false;
                new_last = i;
            }
        }

        if contiguous {
            *next = i + 1;
        }
    }

    *last = new_last;

    (all_fit, none_fit)
}
