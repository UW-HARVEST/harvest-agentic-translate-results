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

struct PackCtx {
    max_w: i32,
    max_h: i32,
    page: i32,
    next: usize,
    last: usize,
}

struct PackRes {
    all_fit: bool,
    none_fit: bool,
}

/// Rectangle packer using a binary tree algorithm
pub struct RectPacker;
impl RectPacker {
    /// Pack rectangles into a bin of the given maximum dimensions
    pub fn pack(max_w: i32, max_h: i32, paging: bool, rects: &mut [Rect]) -> bool {
        if rects.is_empty() {
            return true;
        }

        // Sort by max side descending, then min side descending
        rects.sort_by(|a, b| {
            let max_a = a.w.max(a.h);
            let max_b = b.w.max(b.h);
            let diff = max_b.cmp(&max_a);
            if diff != Ordering::Equal {
                return diff;
            }
            let min_a = a.w.min(a.h);
            let min_b = b.w.min(b.h);
            min_b.cmp(&min_a)
        });

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

        let mut all_packed = false;

        loop {
            let res = pack_bin_tree(&mut ctx, rects);
            let ok = res.all_fit;
            all_packed = all_packed || ok;

            if ok || !paging || res.none_fit {
                break;
            }
            ctx.page += 1;
        }

        all_packed
    }
}

fn pack_bin_tree(ctx: &mut PackCtx, rects: &mut [Rect]) -> PackRes {
    let mut res = PackRes {
        all_fit: true,
        none_fit: true,
    };

    let root_w = rects[ctx.next].w.min(ctx.max_w);
    let root_h = rects[ctx.next].h.min(ctx.max_h);

    let mut root = BinNode::new(0, 0, root_w, root_h);

    let mut contiguous = true;
    let mut last = ctx.last;

    for i in ctx.next..=ctx.last {
        if !rects[i].info.packed {
            let w = rects[i].w;
            let h = rects[i].h;

            // Try find, then split
            if let Some((x, y)) = root.find_and_split(w, h) {
                rects[i].info.x = x;
                rects[i].info.y = y;
                rects[i].info.packed = true;
                rects[i].info.page = ctx.page;
                res.none_fit = false;
            } else {
                // Try grow
                let rw = rects[i].w;
                let rh = rects[i].h;
                if let Some((x, y)) = root.grow_and_place(rw, rh, ctx.max_w, ctx.max_h) {
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
        }

        if contiguous {
            ctx.next = i + 1;
        }
    }

    ctx.last = last;
    res
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
    #[allow(dead_code)]
    fn find(&self, w: i32, h: i32) -> Option<&BinNode> {
        if self.used {
            let right = self.right.as_ref().unwrap().find(w, h);
            if right.is_some() {
                return right;
            }
            self.down.as_ref().unwrap().find(w, h)
        } else if w <= self.w && h <= self.h {
            Some(self)
        } else {
            None
        }
    }
    /// Split this node after placing a rectangle of the given dimensions
    fn split(&mut self, w: i32, h: i32) -> &Self {
        self.used = true;
        self.down = Some(Box::new(BinNode::new(self.x, self.y + h, self.w, self.h - h)));
        self.right = Some(Box::new(BinNode::new(self.x + w, self.y, self.w - w, h)));
        self
    }

    /// Find a fitting node and split it, returning (x, y) of placement
    fn find_and_split(&mut self, w: i32, h: i32) -> Option<(i32, i32)> {
        if self.used {
            if let Some(pos) = self.right.as_mut().unwrap().find_and_split(w, h) {
                return Some(pos);
            }
            return self.down.as_mut().unwrap().find_and_split(w, h);
        } else if w <= self.w && h <= self.h {
            let x = self.x;
            let y = self.y;
            self.split(w, h);
            Some((x, y))
        } else {
            None
        }
    }

    /// Grow the bin to the right to accommodate a rectangle
    #[allow(dead_code)]
    fn grow_right(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let mut old = BinNode::new(0, 0, 0, 0);
        std::mem::swap(&mut old, self);

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old.w + rect.w;
        self.h = old.h;
        self.right = Some(Box::new(BinNode::new(old.w, 0, rect.w, old.h)));
        self.down = Some(Box::new(old));

        let node = self.find(rect.w, rect.h);
        if node.is_some() {
            // We know it exists, now split via mutable path
            None // placeholder, handled by grow_and_place
        } else {
            None
        }
    }
    /// Grow the bin downward to accommodate a rectangle
    #[allow(dead_code)]
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let mut old = BinNode::new(0, 0, 0, 0);
        std::mem::swap(&mut old, self);

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old.w;
        self.h = old.h + rect.h;
        self.down = Some(Box::new(BinNode::new(0, old.h, old.w, rect.h)));
        self.right = Some(Box::new(old));

        let node = self.find(rect.w, rect.h);
        if node.is_some() {
            None // placeholder, handled by grow_and_place
        } else {
            None
        }
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    #[allow(dead_code)]
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

    /// Grow and then find+split, returning (x, y) of placement
    fn grow_and_place(&mut self, w: i32, h: i32, max_w: i32, max_h: i32) -> Option<(i32, i32)> {
        let can_grow_down = w <= self.w && (h + self.h) <= max_h;
        let can_grow_right = h <= self.h && (w + self.w) <= max_w;

        let should_grow_right = can_grow_right && self.h >= (self.w + w);
        let should_grow_down = can_grow_down && self.w >= (self.h + h);

        let do_right = if should_grow_right {
            true
        } else if should_grow_down {
            false
        } else if can_grow_right {
            true
        } else if can_grow_down {
            false
        } else {
            return None;
        };

        if do_right {
            self.do_grow_right(w, h);
        } else {
            self.do_grow_down(w, h);
        }

        self.find_and_split(w, h)
    }

    fn do_grow_right(&mut self, rw: i32, _rh: i32) {
        let mut old = BinNode::new(0, 0, 0, 0);
        std::mem::swap(&mut old, self);

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old.w + rw;
        self.h = old.h;
        self.right = Some(Box::new(BinNode::new(old.w, 0, rw, old.h)));
        self.down = Some(Box::new(old));
    }

    fn do_grow_down(&mut self, _rw: i32, rh: i32) {
        let mut old = BinNode::new(0, 0, 0, 0);
        std::mem::swap(&mut old, self);

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old.w;
        self.h = old.h + rh;
        self.down = Some(Box::new(BinNode::new(0, old.h, old.w, rh)));
        self.right = Some(Box::new(old));
    }
}
