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
    pub fn pack(max_w: i32, max_h: i32, paging: bool, rects: &mut [Rect]) -> bool {
        if rects.is_empty() {
            return true;
        }

        rects.sort_by(|a, b| {
            let diff = a.w.max(a.h).cmp(&b.w.max(b.h)).reverse();
            if diff == Ordering::Equal {
                a.w.min(a.h).cmp(&b.w.min(b.h)).reverse()
            } else {
                diff
            }
        });

        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let n = rects.len();
        let mut page = 0;
        let mut next = 0;
        let mut last = n - 1;
        let mut all_packed = false;

        loop {
            let mut all_fit = true;
            let mut none_fit = true;

            let root_w = rects[next].w.min(max_w);
            let root_h = rects[next].h.min(max_h);
            let mut root = BinNode::new(0, 0, root_w, root_h);

            let mut contiguous = true;
            let mut new_last = last;

            for i in next..=last {
                if !rects[i].info.packed {
                    if let Some((x, y)) = root.find_and_split(rects[i].w, rects[i].h, max_w, max_h) {
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
                    next = i + 1;
                }
            }

            last = new_last;

            if all_fit {
                all_packed = true;
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
    /// Grow the bin to the right to accommodate a rectangle
    fn grow_right(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let old = std::mem::replace(self, BinNode::new(0, 0, 0, 0));
        let new_w = old.w + rect.w;
        let new_h = old.h;
        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = new_w;
        self.h = new_h;
        self.down = Some(Box::new(old));
        self.right = Some(Box::new(BinNode::new(self.down.as_ref().unwrap().w, 0, rect.w, new_h)));
        self.find_mut_and_split(rect.w, rect.h)
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let old = std::mem::replace(self, BinNode::new(0, 0, 0, 0));
        let new_w = old.w;
        let new_h = old.h + rect.h;
        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = new_w;
        self.h = new_h;
        self.down = Some(Box::new(BinNode::new(0, old.h, new_w, rect.h)));
        self.right = Some(Box::new(old));
        self.find_mut_and_split(rect.w, rect.h)
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let can_grow_down = rect.w <= self.w && (rect.h + self.h) <= max_h;
        let can_grow_right = rect.h <= self.h && (rect.w + self.w) <= max_w;

        let should_grow_right = can_grow_right && (self.h >= self.w + rect.w);
        let should_grow_down = can_grow_down && (self.w >= self.h + rect.h);

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

    /// Find a mutable node and split it, returning an immutable reference
    fn find_mut_and_split(&mut self, w: i32, h: i32) -> Option<&BinNode> {
        if self.used {
            if self.right.as_ref().unwrap().find(w, h).is_some() {
                return self.right.as_mut().unwrap().find_mut_and_split(w, h);
            }
            if self.down.as_ref().unwrap().find(w, h).is_some() {
                return self.down.as_mut().unwrap().find_mut_and_split(w, h);
            }
            None
        } else if w <= self.w && h <= self.h {
            Some(self.split(w, h))
        } else {
            None
        }
    }

    /// Try to find and split, or grow and split. Returns (x, y) if successful.
    fn find_and_split(&mut self, w: i32, h: i32, max_w: i32, max_h: i32) -> Option<(i32, i32)> {
        if self.find(w, h).is_some() {
            let node = self.find_mut_and_split(w, h).unwrap();
            Some((node.x, node.y))
        } else {
            let r = Rect::new(0, w, h);
            let node = self.grow(&r, max_w, max_h)?;
            Some((node.x, node.y))
        }
    }
}
