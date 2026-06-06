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
        Rect {
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

        // Sort nodes by max side (descending), tie-break by min side (descending)
        rects.sort_by(|a, b| {
            let a_max = a.w.max(a.h);
            let b_max = b.w.max(b.h);
            let cmp = b_max.cmp(&a_max);
            if cmp != Ordering::Equal {
                cmp
            } else {
                let a_min = a.w.min(a.h);
                let b_min = b.w.min(b.h);
                b_min.cmp(&a_min)
            }
        });

        // Reset info on every rectangle
        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let n = rects.len();
        let mut next: usize = 0;
        let mut last: usize = n - 1;
        let mut page: i32 = 0;
        let mut ok = false;
        let mut all_packed = false;

        while !ok {
            let (all_fit, none_fit, new_next, new_last) =
                pack_bin_tree(rects, next, last, max_w, max_h, page);
            next = new_next;
            last = new_last;
            ok = all_fit;
            all_packed = all_packed || ok;

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
            if let Some(right) = self.right.as_deref() {
                if let Some(found) = right.find(w, h) {
                    return Some(found);
                }
            }
            if let Some(down) = self.down.as_deref() {
                return down.find(w, h);
            }
            None
        } else if w <= self.w && h <= self.h {
            Some(self)
        } else {
            None
        }
    }
    /// Find a mutable node in the tree that can fit a rectangle of the given dimensions
    fn find_mut(&mut self, w: i32, h: i32) -> Option<&mut BinNode> {
        if self.used {
            if let Some(right) = self.right.as_deref_mut() {
                if let Some(found) = right.find_mut(w, h) {
                    return Some(found);
                }
            }
            if let Some(down) = self.down.as_deref_mut() {
                return down.find_mut(w, h);
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
        let old = BinNode {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            used: self.used,
            right: self.right.take(),
            down: self.down.take(),
        };
        let old_w = old.w;
        let old_h = old.h;

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w + rect.w;
        self.h = old_h;
        self.down = Some(Box::new(old));
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

        let r_w = rect.w;
        let r_h = rect.h;
        if let Some(node) = self.find_mut(r_w, r_h) {
            node.split(r_w, r_h);
            Some(&*node)
        } else {
            None
        }
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let old = BinNode {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            used: self.used,
            right: self.right.take(),
            down: self.down.take(),
        };
        let old_w = old.w;
        let old_h = old.h;

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w;
        self.h = old_h + rect.h;
        self.down = Some(Box::new(BinNode::new(0, old_h, old_w, rect.h)));
        self.right = Some(Box::new(old));

        let r_w = rect.w;
        let r_h = rect.h;
        if let Some(node) = self.find_mut(r_w, r_h) {
            node.split(r_w, r_h);
            Some(&*node)
        } else {
            None
        }
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let r_w = rect.w;
        let r_h = rect.h;
        let can_grow_down = (r_w <= self.w) && ((r_h + self.h) <= max_h);
        let can_grow_right = (r_h <= self.h) && ((r_w + self.w) <= max_w);
        let should_grow_right = can_grow_right && (self.h >= (self.w + r_w));
        let should_grow_down = can_grow_down && (self.w >= (self.h + r_h));

        if should_grow_right {
            return self.grow_right(rect, max_w, max_h);
        }
        if should_grow_down {
            return self.grow_down(rect, max_w, max_h);
        }
        if can_grow_right {
            return self.grow_right(rect, max_w, max_h);
        }
        if can_grow_down {
            return self.grow_down(rect, max_w, max_h);
        }
        None
    }
}

/// Run a single packing pass for a single page.
/// Returns (all_fit, none_fit, new_next, new_last).
fn pack_bin_tree(
    rects: &mut [Rect],
    next: usize,
    last: usize,
    max_w: i32,
    max_h: i32,
    page: i32,
) -> (bool, bool, usize, usize) {
    let mut all_fit = true;
    let mut none_fit = true;
    let mut contiguous = true;
    let mut new_next = next;
    let mut new_last = last;

    let root_w = rects[next].w.min(max_w);
    let root_h = rects[next].h.min(max_h);
    let mut root = BinNode::new(0, 0, root_w, root_h);

    for i in next..=last {
        if !rects[i].info.packed {
            let r_w = rects[i].w;
            let r_h = rects[i].h;

            // Try to find an existing free node first
            let placement = if let Some(node) = root.find_mut(r_w, r_h) {
                let xy = (node.x, node.y);
                node.split(r_w, r_h);
                Some(xy)
            } else {
                // Need to clone to avoid simultaneous mutable borrow with rects[i]
                let rect_copy = Rect {
                    id: rects[i].id,
                    w: r_w,
                    h: r_h,
                    info: rects[i].info,
                };
                root.grow(&rect_copy, max_w, max_h).map(|n| (n.x, n.y))
            };

            if let Some((x, y)) = placement {
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
            new_next = i + 1;
        }
    }

    (all_fit, none_fit, new_next, new_last)
}
