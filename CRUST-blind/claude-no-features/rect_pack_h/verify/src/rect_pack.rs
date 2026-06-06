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

        // Sort rects by max side descending; tie-break by min side descending
        rects.sort_by(|a, b| {
            let max_a = a.w.max(a.h);
            let max_b = b.w.max(b.h);
            match max_b.cmp(&max_a) {
                Ordering::Equal => {
                    let min_a = a.w.min(a.h);
                    let min_b = b.w.min(b.h);
                    min_b.cmp(&min_a)
                }
                other => other,
            }
        });

        // Reset packing info for all rectangles
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
            let (all_fit, none_fit) =
                pack_bin_tree(rects, &mut next, &mut last, max_w, max_h, page);
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

/// Run a single packing pass over rects[next..=last] using a fresh tree.
/// Updates `next` and `last` in place to track progress across pages.
/// Returns (all_fit, none_fit).
fn pack_bin_tree(
    rects: &mut [Rect],
    next: &mut usize,
    last: &mut usize,
    max_w: i32,
    max_h: i32,
    page: i32,
) -> (bool, bool) {
    let mut all_fit = true;
    let mut none_fit = true;
    let mut contiguous = true;

    let next_in = *next;
    let last_in = *last;
    let mut current_last = last_in;

    let root_w = if rects[next_in].w <= max_w {
        rects[next_in].w
    } else {
        max_w
    };
    let root_h = if rects[next_in].h <= max_h {
        rects[next_in].h
    } else {
        max_h
    };
    let mut root = BinNode::new(0, 0, root_w, root_h);

    let mut i = next_in;
    while i <= last_in {
        if !rects[i].info.packed {
            let w = rects[i].w;
            let h = rects[i].h;

            // First try to fit into existing tree
            let placed = match root.find_mut(w, h) {
                Some(node) => {
                    let xy = (node.x, node.y);
                    node.split(w, h);
                    Some(xy)
                }
                None => {
                    // Need to grow the tree
                    let r_clone = rects[i].clone();
                    root.grow(&r_clone, max_w, max_h).map(|n| (n.x, n.y))
                }
            };

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
                current_last = i;
            }
        }

        if contiguous {
            *next = i + 1;
        }

        i += 1;
    }

    *last = current_last;

    (all_fit, none_fit)
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
    /// Mutable version of `find` used internally for splitting/placing.
    fn find_mut(&mut self, w: i32, h: i32) -> Option<&mut BinNode> {
        if self.used {
            // Determine if right subtree can satisfy the request before
            // borrowing it mutably (so we don't clobber the chance to try
            // the down subtree).
            let right_has = self
                .right
                .as_deref()
                .map(|r| r.find(w, h).is_some())
                .unwrap_or(false);
            if right_has {
                return self.right.as_deref_mut().and_then(|r| r.find_mut(w, h));
            }
            self.down.as_deref_mut().and_then(|d| d.find_mut(w, h))
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
        // Move current root contents into a new "old" node that becomes
        // the down child of the new root.
        let old_node = Box::new(BinNode {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            used: self.used,
            right: self.right.take(),
            down: self.down.take(),
        });
        let old_w = old_node.w;
        let old_h = old_node.h;

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w + rect.w;
        self.h = old_h;
        self.down = Some(old_node);
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

        let w = rect.w;
        let h = rect.h;
        if let Some(node) = self.find_mut(w, h) {
            node.split(w, h);
            Some(&*node)
        } else {
            None
        }
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let old_node = Box::new(BinNode {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            used: self.used,
            right: self.right.take(),
            down: self.down.take(),
        });
        let old_w = old_node.w;
        let old_h = old_node.h;

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w;
        self.h = old_h + rect.h;
        self.down = Some(Box::new(BinNode::new(0, old_h, old_w, rect.h)));
        self.right = Some(old_node);

        let w = rect.w;
        let h = rect.h;
        if let Some(node) = self.find_mut(w, h) {
            node.split(w, h);
            Some(&*node)
        } else {
            None
        }
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
