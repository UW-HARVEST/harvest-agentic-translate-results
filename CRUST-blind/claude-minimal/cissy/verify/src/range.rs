pub mod range {
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
            let mut new_elem = Box::new(RangeElement::new());
            new_elem.start = num;
            new_elem.rangetype = RangeType::Single;

            match start_of_list {
                None => Some(new_elem),
                Some(mut head) => {
                    // walk to end of list
                    let mut ptr: &mut RangeElement = &mut *head;
                    while ptr.next.is_some() {
                        ptr = ptr.next.as_mut().unwrap();
                    }
                    ptr.next = Some(new_elem);
                    Some(head)
                }
            }
        }

        pub fn add_start_end(
            start: u32,
            end: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            assert!(start > 0);
            assert!(end >= start);
            let mut new_elem = Box::new(RangeElement::new());
            new_elem.start = start;
            if start == end {
                new_elem.rangetype = RangeType::Single;
            } else {
                new_elem.rangetype = RangeType::StartEnd;
                new_elem.end = end;
            }

            match start_of_list {
                None => Some(new_elem),
                Some(mut head) => {
                    let mut ptr: &mut RangeElement = &mut *head;
                    while ptr.next.is_some() {
                        ptr = ptr.next.as_mut().unwrap();
                    }
                    ptr.next = Some(new_elem);
                    Some(head)
                }
            }
        }

        pub fn add_greater_equal(
            num: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            let mut new_elem = Box::new(RangeElement::new());
            new_elem.start = num;
            new_elem.rangetype = RangeType::GreaterEqual;

            match start_of_list {
                None => Some(new_elem),
                Some(mut head) => {
                    let mut ptr: &mut RangeElement = &mut *head;
                    while ptr.next.is_some() {
                        ptr = ptr.next.as_mut().unwrap();
                    }
                    ptr.next = Some(new_elem);
                    Some(head)
                }
            }
        }

        pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
            assert!(start_of_list.is_some());
            let mut current = start_of_list.as_deref();
            while let Some(node) = current {
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
                current = node.next.as_deref();
            }
            false
        }

        pub fn to_string<'a>(&'a self, buf: &'a mut String, bufsize: usize) -> &'a str {
            buf.clear();
            let formatted = match self.rangetype {
                RangeType::Single => format!("[{}]", self.start),
                RangeType::GreaterEqual => format!("[{}-]", self.start),
                RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
                RangeType::Empty => "[]".to_string(),
            };
            // truncate to bufsize-1 like snprintf
            let truncated: String = if bufsize == 0 {
                String::new()
            } else if formatted.len() >= bufsize {
                formatted.chars().take(bufsize - 1).collect()
            } else {
                formatted
            };
            buf.push_str(&truncated);
            buf.as_str()
        }

        pub fn list_to_string<'a>(
            buf: &'a mut String,
            bufsize: usize,
            start_of_list: &'a Option<Box<RangeElement>>,
        ) -> &'a str {
            buf.clear();
            let mut current = start_of_list.as_deref();
            while let Some(node) = current {
                if buf.len() >= bufsize {
                    break;
                }
                let mut tmp = String::new();
                node.to_string(&mut tmp, 256);
                let remaining = bufsize.saturating_sub(buf.len());
                if tmp.len() <= remaining {
                    buf.push_str(&tmp);
                } else {
                    let truncated: String = tmp.chars().take(remaining).collect();
                    buf.push_str(&truncated);
                }
                current = node.next.as_deref();
            }
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
                    if state & flag_dash != 0 {
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
                        if state & flag_dash != 0 {
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

            // finish up
            if numbuf.is_empty() {
                if state & flag_dash != 0 {
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
