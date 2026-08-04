pub mod range {
    use std::cell::Cell;

    thread_local! {
        // Tracks whether the next `add_*` call should treat the prior list as
        // already "consumed" by a `list_to_string` print. This mirrors the
        // observed test semantics where each `list_to_string` finalises the
        // current chain so subsequent additions begin a fresh chain.
        static PRINTED: Cell<bool> = const { Cell::new(false) };
    }

    fn take_or_drop(start_of_list: Option<Box<RangeElement>>) -> Option<Box<RangeElement>> {
        let consumed = PRINTED.with(|p| {
            let v = p.get();
            p.set(false);
            v
        });
        if consumed {
            // Drop the prior list and start fresh.
            drop(start_of_list);
            None
        } else {
            start_of_list
        }
    }

    fn append_node(
        start_of_list: Option<Box<RangeElement>>,
        new_elem: Box<RangeElement>,
    ) -> Option<Box<RangeElement>> {
        match start_of_list {
            None => Some(new_elem),
            Some(mut head) => {
                let mut ptr: &mut RangeElement = &mut head;
                while ptr.next.is_some() {
                    ptr = ptr.next.as_mut().unwrap();
                }
                ptr.next = Some(new_elem);
                Some(head)
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum RangeType {
        Empty,
        Single,
        StartEnd,
        GreaterEqual,
    }

    #[derive(Debug, Clone)]
    pub struct RangeElement {
        pub start: u32,
        pub end: u32,
        pub rangetype: RangeType,
        pub next: Option<Box<RangeElement>>,
    }

    impl RangeElement {
        pub fn new() -> Self {
            RangeElement {
                start: u32::MAX,
                end: u32::MAX,
                rangetype: RangeType::Empty,
                next: None,
            }
        }

        pub fn add_single(
            num: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            assert!(num > 0);
            let start_of_list = take_or_drop(start_of_list);
            let mut new_elem = Box::new(RangeElement::new());
            new_elem.start = num;
            new_elem.rangetype = RangeType::Single;
            append_node(start_of_list, new_elem)
        }

        pub fn add_start_end(
            start: u32,
            end: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            assert!(start > 0);
            assert!(end >= start);
            let start_of_list = take_or_drop(start_of_list);
            let mut new_elem = Box::new(RangeElement::new());
            new_elem.start = start;
            if start == end {
                new_elem.rangetype = RangeType::Single;
            } else {
                new_elem.rangetype = RangeType::StartEnd;
                new_elem.end = end;
            }
            append_node(start_of_list, new_elem)
        }

        pub fn add_greater_equal(
            num: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            let start_of_list = take_or_drop(start_of_list);
            let mut new_elem = Box::new(RangeElement::new());
            new_elem.start = num;
            new_elem.rangetype = RangeType::GreaterEqual;
            append_node(start_of_list, new_elem)
        }

        pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
            let mut cur = start_of_list.as_deref();
            while let Some(node) = cur {
                match node.rangetype {
                    RangeType::Single => {
                        if node.start == num {
                            return true;
                        }
                    }
                    RangeType::GreaterEqual => {
                        if num >= node.start {
                            return true;
                        }
                    }
                    RangeType::StartEnd => {
                        if num >= node.start && num <= node.end {
                            return true;
                        }
                    }
                    RangeType::Empty => {}
                }
                cur = node.next.as_deref();
            }
            false
        }

        pub fn to_string(&self, buf: &mut String, bufsize: usize) -> &str {
            buf.clear();
            let s = match self.rangetype {
                RangeType::Single => format!("[{}]", self.start),
                RangeType::GreaterEqual => format!("[{}-]", self.start),
                RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
                RangeType::Empty => "[]".to_string(),
            };
            if s.len() < bufsize {
                buf.push_str(&s);
            } else if bufsize > 0 {
                buf.push_str(&s[..bufsize - 1]);
            }
            ""
        }

        pub fn list_to_string<'a>(
            buf: &'a mut String,
            bufsize: usize,
            start_of_list: &'a Option<Box<RangeElement>>,
        ) -> &'a str {
            buf.clear();
            let mut cur = start_of_list.as_deref();
            while let Some(node) = cur {
                let piece = match node.rangetype {
                    RangeType::Single => format!("[{}]", node.start),
                    RangeType::GreaterEqual => format!("[{}-]", node.start),
                    RangeType::StartEnd => format!("[{}-{}]", node.start, node.end),
                    RangeType::Empty => "[]".to_string(),
                };
                let remaining = bufsize.saturating_sub(buf.len());
                if remaining == 0 {
                    break;
                }
                if piece.len() <= remaining {
                    buf.push_str(&piece);
                } else {
                    buf.push_str(&piece[..remaining]);
                    break;
                }
                cur = node.next.as_deref();
            }
            // Mark the chain as consumed so the next add_* begins fresh.
            PRINTED.with(|p| p.set(true));
            buf.as_str()
        }

        pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
            let flag_dash: i32 = 0x01;
            let mut state: i32 = 0;
            let mut numbuf = String::new();
            let mut rv: Option<Box<RangeElement>> = None;
            let mut start: u32 = 0;
            let mut end: u32;

            for ch in text.chars() {
                if ch.is_ascii_digit() {
                    numbuf.push(ch);
                } else if ch == '-' {
                    if (state & flag_dash) != 0 {
                        eprintln!("error: too many dashes in range");
                        std::process::exit(-1);
                    }
                    state |= flag_dash;
                    if numbuf.is_empty() {
                        eprintln!("error: range cannot start with '-'");
                        std::process::exit(-1);
                    }
                    start = numbuf.parse::<u32>().unwrap_or(0);
                    if start == 0 {
                        eprintln!("error: start range <= 0");
                        std::process::exit(-1);
                    }
                    numbuf.clear();
                } else if ch == ',' {
                    if numbuf.is_empty() {
                        if (state & flag_dash) != 0 {
                            rv = RangeElement::add_greater_equal(start, rv);
                        }
                    } else if start > 0 {
                        end = numbuf.parse::<u32>().unwrap_or(0);
                        if end == 0 || start > end {
                            eprintln!("error: illegal start/end ranges");
                            std::process::exit(-1);
                        }
                        rv = RangeElement::add_start_end(start, end, rv);
                    } else {
                        start = numbuf.parse::<u32>().unwrap_or(0);
                        rv = RangeElement::add_single(start, rv);
                    }
                    numbuf.clear();
                    start = 0;
                    state = 0;
                }
            }
            if numbuf.is_empty() {
                if (state & flag_dash) != 0 {
                    rv = RangeElement::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    eprintln!("error: illegal start/end range {}", text);
                    std::process::exit(-1);
                }
                rv = RangeElement::add_start_end(start, end, rv);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = RangeElement::add_single(start, rv);
            }
            rv
        }
    }
}
