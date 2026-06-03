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

/// Internal context used during packing
struct PackCtx {
    max_w: i32,
    max_h: i32,
    page: i32,
    next: usize,
    last: usize,
}

/// Internal result of a single pack pass
struct PackRes {
    all_fit: bool,
    none_fit: bool,
}

/// Run a single pack pass over the range [ctx.next, ctx.last] in `rects`.
/// Updates `ctx.next` and `ctx.last` to reflect the unpacked range.
fn pack_bin_tree(ctx: &mut PackCtx, rects: &mut [Rect]) -> PackRes {
    let mut res = PackRes {
        all_fit: true,
        none_fit: true,
    };

    let first = ctx.next;
    let root_w = if rects[first].w <= ctx.max_w {
        rects[first].w
    } else {
        ctx.max_w
    };
    let root_h = if rects[first].h <= ctx.max_h {
        rects[first].h
    } else {
        ctx.max_h
    };
    let mut root = BinNode::new(0, 0, root_w, root_h);

    let mut contiguous = true;
    let mut last = ctx.last;

    for i in first..=ctx.last {
        if !rects[i].info.packed {
            let r_w = rects[i].w;
            let r_h = rects[i].h;

            let mut placed_pos: Option<(i32, i32)> = None;

            if let Some(node) = root.find_mut(r_w, r_h) {
                placed_pos = Some((node.x, node.y));
                node.split(r_w, r_h);
            }

            if placed_pos.is_none() {
                let r = rects[i].clone();
                if let Some(node) = root.grow(&r, ctx.max_w, ctx.max_h) {
                    placed_pos = Some((node.x, node.y));
                }
            }

            if let Some((x, y)) = placed_pos {
                rects[i].info.x = x;
                rects[i].info.y = y;
                rects[i].info.packed = true;
                rects[i].info.page = ctx.page;
                res.none_fit = false;
            } else {
                rects[i].info.packed = false;
                res.all_fit = false;
                contiguous = false;
                last = i;
            }
        }

        if contiguous {
            ctx.next = i + 1;
        }
    }

    ctx.last = last;
    res
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

        // Sort rectangles by max side descending, then by min side descending
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

        // Reset all rect output info
        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let mut ctx = PackCtx {
            max_w,
            max_h,
            page: 0,
            next: 0,
            last: rects.len() - 1,
        };

        let mut ok = false;
        let mut all_packed = false;

        while !ok {
            let res = pack_bin_tree(&mut ctx, rects);
            ok = res.all_fit;
            all_packed = all_packed || ok;

            if !paging || res.none_fit {
                break;
            }

            ctx.page += 1;
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
                if let Some(node) = right.find(w, h) {
                    return Some(node);
                }
            }
            self.down.as_deref().and_then(|d| d.find(w, h))
        } else if w <= self.w && h <= self.h {
            Some(self)
        } else {
            None
        }
    }

    /// Mutable version of `find`, used internally to allow splitting after finding.
    fn find_mut(&mut self, w: i32, h: i32) -> Option<&mut BinNode> {
        if !self.used {
            if w <= self.w && h <= self.h {
                return Some(self);
            }
            return None;
        }

        if let Some(right) = self.right.as_deref_mut() {
            if let Some(node) = right.find_mut(w, h) {
                return Some(node);
            }
        }
        if let Some(down) = self.down.as_deref_mut() {
            return down.find_mut(w, h);
        }
        None
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
        self.down = Some(Box::new(old));
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

        if let Some(node) = self.find_mut(rect.w, rect.h) {
            Some(node.split(rect.w, rect.h))
        } else {
            None
        }
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

        if let Some(node) = self.find_mut(rect.w, rect.h) {
            Some(node.split(rect.w, rect.h))
        } else {
            None
        }
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let can_grow_down = rect.w <= self.w && (rect.h + self.h) <= max_h;
        let can_grow_right = rect.h <= self.h && (rect.w + self.w) <= max_w;
        let should_grow_right = can_grow_right && self.h >= (self.w + rect.w);
        let should_grow_down = can_grow_down && self.w >= (self.h + rect.h);

        if should_grow_right {
            self.grow_right(rect, max_w, max_h)
        } else if should_grow_down {
            self.grow_down(rect, max_w, max_h)
        } else if can_grow_right {
            self.grow_right(rect, max_w, max_h)
        } else if can_grow_down {
            self.grow_down(rect, max_w, max_h)
        } else {
            None
        }
    }
}
