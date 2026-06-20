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
            Self {
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
            Self::append(
                start_of_list,
                RangeElement {
                    start: num,
                    end: u32::MAX,
                    rangetype: RangeType::Single,
                    next: None,
                },
            )
        }
        pub fn add_start_end(
            start: u32,
            end: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            assert!(start > 0);
            assert!(end >= start);
            let rangetype = if start == end {
                RangeType::Single
            } else {
                RangeType::StartEnd
            };
            Self::append(
                start_of_list,
                RangeElement {
                    start,
                    end,
                    rangetype,
                    next: None,
                },
            )
        }
        pub fn add_greater_equal(
            num: u32,
            start_of_list: Option<Box<RangeElement>>,
        ) -> Option<Box<RangeElement>> {
            Self::append(
                start_of_list,
                RangeElement {
                    start: num,
                    end: u32::MAX,
                    rangetype: RangeType::GreaterEqual,
                    next: None,
                },
            )
        }
        pub fn contains_num(num: u32, start_of_list: &Option<Box<RangeElement>>) -> bool {
            let mut ptr = start_of_list.as_deref();
            while let Some(element) = ptr {
                match element.rangetype {
                    RangeType::Single if element.start == num => return true,
                    RangeType::GreaterEqual if num >= element.start => return true,
                    RangeType::StartEnd if num >= element.start && num <= element.end => {
                        return true
                    }
                    _ => {}
                }
                ptr = element.next.as_deref();
            }
            false
        }
        pub fn to_string<'a>(&self, buf: &'a mut String, bufsize: usize) -> &'a str {
            let text = match self.rangetype {
                RangeType::Single => format!("[{}]", self.start),
                RangeType::GreaterEqual => format!("[{}-]", self.start),
                RangeType::StartEnd => format!("[{}-{}]", self.start, self.end),
                RangeType::Empty => "[]".to_string(),
            };
            buf.clear();
            let max_len = bufsize.saturating_sub(1);
            if max_len > 0 {
                let slice = if text.len() <= max_len {
                    text.as_str()
                } else {
                    &text[..max_len]
                };
                buf.push_str(slice);
            }
            buf.as_str()
        }
        pub fn list_to_string<'a>(
            buf: &'a mut String,
            bufsize: usize,
            start_of_list: &'a Option<Box<RangeElement>>,
        ) -> &'a str {
            buf.clear();
            let max_len = bufsize.saturating_sub(1);
            let mut ptr = start_of_list.as_deref();
            while let Some(element) = ptr {
                let text = match element.rangetype {
                    RangeType::Single => format!("[{}]", element.start),
                    RangeType::GreaterEqual => format!("[{}-]", element.start),
                    RangeType::StartEnd => format!("[{}-{}]", element.start, element.end),
                    RangeType::Empty => "[]".to_string(),
                };
                if buf.len() >= max_len {
                    break;
                }
                let remaining = max_len - buf.len();
                if text.len() <= remaining {
                    buf.push_str(&text);
                } else {
                    buf.push_str(&text[..remaining]);
                    break;
                }
                ptr = element.next.as_deref();
            }
            buf.as_str()
        }
        pub fn parse_int_ranges(text: &str) -> Option<Box<RangeElement>> {
            let mut rv = None;
            let mut state = 0_i32;
            let flag_dash = 0x01_i32;
            let mut numbuf = String::new();
            let mut start = 0_u32;

            for ch in text.chars() {
                if ch.is_ascii_digit() {
                    numbuf.push(ch);
                } else if ch == '-' {
                    if (state & flag_dash) != 0 {
                        panic!("error: too many dashes in range");
                    }
                    state |= flag_dash;
                    if numbuf.is_empty() {
                        panic!("error: range cannot start with '-'");
                    }
                    start = numbuf.parse::<u32>().unwrap_or(0);
                    if start == 0 {
                        panic!("error: start range <= 0");
                    }
                    numbuf.clear();
                } else if ch == ',' {
                    if numbuf.is_empty() {
                        if (state & flag_dash) != 0 {
                            rv = Self::add_greater_equal(start, rv);
                        }
                    } else if start > 0 {
                        let end = numbuf.parse::<u32>().unwrap_or(0);
                        if end == 0 || start > end {
                            panic!("error: illegal start/end ranges");
                        }
                        rv = Self::add_start_end(start, end, rv);
                    } else {
                        start = numbuf.parse::<u32>().unwrap_or(0);
                        rv = Self::add_single(start, rv);
                    }
                    numbuf.clear();
                    start = 0;
                    state = 0;
                }
            }

            if numbuf.is_empty() {
                if (state & flag_dash) != 0 {
                    rv = Self::add_greater_equal(start, rv);
                }
            } else if start > 0 {
                let end = numbuf.parse::<u32>().unwrap_or(0);
                if end == 0 || start > end {
                    panic!("error: illegal start/end range {}", text);
                }
                rv = Self::add_start_end(start, end, rv);
            } else {
                start = numbuf.parse::<u32>().unwrap_or(0);
                rv = Self::add_single(start, rv);
            }

            rv
        }

        fn append(
            start_of_list: Option<Box<RangeElement>>,
            element: RangeElement,
        ) -> Option<Box<RangeElement>> {
            match start_of_list {
                None => Some(Box::new(element)),
                Some(mut list) => {
                    let mut ptr = &mut list;
                    while let Some(ref mut next) = ptr.next {
                        ptr = next;
                    }
                    ptr.next = Some(Box::new(element));
                    Some(list)
                }
            }
        }
    }
}
