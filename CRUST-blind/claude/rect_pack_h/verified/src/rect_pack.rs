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

        // Sort rectangles by max side (descending), then min side (descending)
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

        // Reset all packing info before starting
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
            if let Some(ref right) = self.right {
                if let Some(found) = right.find(w, h) {
                    return Some(found);
                }
            }
            if let Some(ref down) = self.down {
                return down.find(w, h);
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
        self.w = old_w + rect.w;
        self.h = old_h;
        self.down = Some(old);
        self.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

        let pos = find_and_split_pos(self, rect.w, rect.h);
        match pos {
            Some((x, y)) => find_split_node(self, x, y),
            None => None,
        }
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

        let pos = find_and_split_pos(self, rect.w, rect.h);
        match pos {
            Some((x, y)) => find_split_node(self, x, y),
            None => None,
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

/// Helper that finds an unused node in the tree fitting (w, h), splits it,
/// and returns the (x, y) position where the rectangle was placed.
fn find_and_split_pos(node: &mut BinNode, w: i32, h: i32) -> Option<(i32, i32)> {
    if node.used {
        if let Some(ref mut right) = node.right {
            if let Some(pos) = find_and_split_pos(right, w, h) {
                return Some(pos);
            }
        }
        if let Some(ref mut down) = node.down {
            return find_and_split_pos(down, w, h);
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

/// Helper to locate a node in the tree at the given (x, y) that's been used
/// (i.e., has been split).
fn find_split_node(node: &BinNode, x: i32, y: i32) -> Option<&BinNode> {
    if node.used && node.x == x && node.y == y && node.right.is_some() && node.down.is_some() {
        return Some(node);
    }
    if let Some(ref r) = node.right {
        if let Some(found) = find_split_node(r, x, y) {
            return Some(found);
        }
    }
    if let Some(ref d) = node.down {
        return find_split_node(d, x, y);
    }
    None
}

/// Try to grow the bin tree to accommodate a rectangle, returning the
/// position where the rectangle was placed (if successful).
fn grow_bin_tree_pos(
    root: &mut BinNode,
    rect: &Rect,
    max_w: i32,
    max_h: i32,
) -> Option<(i32, i32)> {
    let can_grow_down = rect.w <= root.w && (rect.h + root.h) <= max_h;
    let can_grow_right = rect.h <= root.h && (rect.w + root.w) <= max_w;

    let should_grow_right = can_grow_right && root.h >= (root.w + rect.w);
    let should_grow_down = can_grow_down && root.w >= (root.h + rect.h);

    if should_grow_right {
        return grow_right_pos(root, rect);
    } else if should_grow_down {
        return grow_down_pos(root, rect);
    }

    if can_grow_right {
        return grow_right_pos(root, rect);
    } else if can_grow_down {
        return grow_down_pos(root, rect);
    }

    None
}

fn grow_right_pos(root: &mut BinNode, rect: &Rect) -> Option<(i32, i32)> {
    let old_w = root.w;
    let old_h = root.h;
    let old = Box::new(BinNode {
        x: root.x,
        y: root.y,
        w: root.w,
        h: root.h,
        used: root.used,
        right: root.right.take(),
        down: root.down.take(),
    });
    root.used = true;
    root.x = 0;
    root.y = 0;
    root.w = old_w + rect.w;
    root.h = old_h;
    root.down = Some(old);
    root.right = Some(Box::new(BinNode::new(old_w, 0, rect.w, old_h)));

    find_and_split_pos(root, rect.w, rect.h)
}

fn grow_down_pos(root: &mut BinNode, rect: &Rect) -> Option<(i32, i32)> {
    let old_w = root.w;
    let old_h = root.h;
    let old = Box::new(BinNode {
        x: root.x,
        y: root.y,
        w: root.w,
        h: root.h,
        used: root.used,
        right: root.right.take(),
        down: root.down.take(),
    });
    root.used = true;
    root.x = 0;
    root.y = 0;
    root.w = old_w;
    root.h = old_h + rect.h;
    root.down = Some(Box::new(BinNode::new(0, old_h, old_w, rect.h)));
    root.right = Some(old);

    find_and_split_pos(root, rect.w, rect.h)
}

/// Pack a contiguous range of rectangles into a single bin tree (one page).
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

    let start = *next;
    let end = *last;

    if start > end || start >= rects.len() {
        return (all_fit, none_fit);
    }

    let root_w = rects[start].w;
    let root_h = rects[start].h;

    let mut root = BinNode::new(
        0,
        0,
        if root_w <= max_w { root_w } else { max_w },
        if root_h <= max_h { root_h } else { max_h },
    );

    let mut contiguous = true;
    let mut new_last = end;

    let mut i = start;
    loop {
        if !rects[i].info.packed {
            let pos = find_and_split_pos(&mut root, rects[i].w, rects[i].h);
            if let Some((x, y)) = pos {
                rects[i].info.x = x;
                rects[i].info.y = y;
                rects[i].info.packed = true;
                rects[i].info.page = page;
                none_fit = false;
            } else {
                let rect_copy = rects[i].clone();
                let pos2 = grow_bin_tree_pos(&mut root, &rect_copy, max_w, max_h);
                if let Some((x, y)) = pos2 {
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
        }

        if contiguous {
            *next = i + 1;
        }

        if i == end {
            break;
        }
        i += 1;
    }

    *last = new_last;
    (all_fit, none_fit)
}
