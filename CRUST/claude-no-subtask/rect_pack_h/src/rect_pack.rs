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
        let n = rects.len();
        if n == 0 {
            return true;
        }

        // Sort by max side descending, then by min side descending
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

        // Reset packing info for all rectangles
        for r in rects.iter_mut() {
            r.info = RectOutInfo::default();
        }

        let mut next: usize = 0;
        let mut last: usize = n - 1;
        let mut page: i32 = 0;
        let mut all_packed = false;
        let mut ok = false;

        while !ok {
            let (all_fit, none_fit, new_next, new_last) =
                pack_bin_tree(rects, max_w, max_h, page, next, last);
            ok = all_fit;
            all_packed = all_packed || ok;
            next = new_next;
            last = new_last;

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

/// Direction used to navigate within the bin tree
#[derive(Clone, Copy)]
enum Direction {
    Right,
    Down,
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
        let old_w = self.w;
        let old_h = self.h;

        // Move the existing root contents into a new node ("old").
        let old = Box::new(BinNode {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            used: self.used,
            right: self.right.take(),
            down: self.down.take(),
        });

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w + rect.w;
        self.h = old_h;
        self.down = Some(old);
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

        let path = self.find_path(rect.w, rect.h)?;
        {
            let node = self.navigate_mut(&path);
            node.split(rect.w, rect.h);
        }
        Some(self.navigate(&path))
    }
    /// Grow the bin downward to accommodate a rectangle
    fn grow_down(&mut self, rect: &Rect, _max_w: i32, _max_h: i32) -> Option<&BinNode> {
        let old_w = self.w;
        let old_h = self.h;

        let old = Box::new(BinNode {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
            used: self.used,
            right: self.right.take(),
            down: self.down.take(),
        });

        self.used = true;
        self.x = 0;
        self.y = 0;
        self.w = old_w;
        self.h = old_h + rect.h;
        self.down = Some(Box::new(BinNode::new(0, old_h, old_w, rect.h)));
        self.right = Some(old);

        let path = self.find_path(rect.w, rect.h)?;
        {
            let node = self.navigate_mut(&path);
            node.split(rect.w, rect.h);
        }
        Some(self.navigate(&path))
    }
    /// Grow the bin in the optimal direction to fit a rectangle
    fn grow(&mut self, rect: &Rect, max_w: i32, max_h: i32) -> Option<&BinNode> {
        let can_grow_down = rect.w <= self.w && (rect.h + self.h) <= max_h;
        let can_grow_right = rect.h <= self.h && (rect.w + self.w) <= max_w;

        let should_grow_right = can_grow_right && self.h >= (self.w + rect.w);
        let should_grow_down = can_grow_down && self.w >= (self.h + rect.h);

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

impl BinNode {
    /// Find a path (sequence of right/down steps) to a node that fits the rectangle.
    fn find_path(&self, w: i32, h: i32) -> Option<Vec<Direction>> {
        let mut path = Vec::new();
        if self.find_path_rec(w, h, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    fn find_path_rec(&self, w: i32, h: i32, path: &mut Vec<Direction>) -> bool {
        if self.used {
            if let Some(right) = self.right.as_deref() {
                path.push(Direction::Right);
                if right.find_path_rec(w, h, path) {
                    return true;
                }
                path.pop();
            }
            if let Some(down) = self.down.as_deref() {
                path.push(Direction::Down);
                if down.find_path_rec(w, h, path) {
                    return true;
                }
                path.pop();
            }
            false
        } else if w <= self.w && h <= self.h {
            true
        } else {
            false
        }
    }

    /// Navigate down a path immutably and return the target node.
    fn navigate(&self, path: &[Direction]) -> &BinNode {
        let mut node = self;
        for d in path {
            node = match d {
                Direction::Right => node.right.as_deref().expect("invalid path"),
                Direction::Down => node.down.as_deref().expect("invalid path"),
            };
        }
        node
    }

    /// Navigate down a path mutably and return the target node.
    fn navigate_mut(&mut self, path: &[Direction]) -> &mut BinNode {
        let mut node = self;
        for d in path {
            node = match d {
                Direction::Right => node.right.as_deref_mut().expect("invalid path"),
                Direction::Down => node.down.as_deref_mut().expect("invalid path"),
            };
        }
        node
    }
}

/// Pack rectangles starting from `next_in` to `last_in` into a bin tree.
///
/// Returns `(all_fit, none_fit, new_next, new_last)`.
fn pack_bin_tree(
    rects: &mut [Rect],
    max_w: i32,
    max_h: i32,
    page: i32,
    next_in: usize,
    last_in: usize,
) -> (bool, bool, usize, usize) {
    let mut all_fit = true;
    let mut none_fit = true;

    let root_w = rects[next_in].w;
    let root_h = rects[next_in].h;

    let mut root = BinNode::new(
        0,
        0,
        if root_w <= max_w { root_w } else { max_w },
        if root_h <= max_h { root_h } else { max_h },
    );

    let mut contiguous = true;
    let mut last = last_in;
    let mut next = next_in;

    let mut i = next_in;
    while i <= last_in {
        if !rects[i].info.packed {
            let rw = rects[i].w;
            let rh = rects[i].h;

            if let Some(path) = root.find_path(rw, rh) {
                let node = root.navigate_mut(&path);
                let nx = node.x;
                let ny = node.y;
                node.split(rw, rh);
                rects[i].info.x = nx;
                rects[i].info.y = ny;
                rects[i].info.packed = true;
                rects[i].info.page = page;
                none_fit = false;
            } else {
                let rect_clone = rects[i].clone();
                let result = root.grow(&rect_clone, max_w, max_h);
                if let Some(node) = result {
                    let nx = node.x;
                    let ny = node.y;
                    rects[i].info.x = nx;
                    rects[i].info.y = ny;
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
        }

        if contiguous {
            next = i + 1;
        }

        if i == last_in {
            break;
        }
        i += 1;
    }

    (all_fit, none_fit, next, last)
}
