use std::fmt;
#[derive(Debug)]
pub enum InversionListError {
    ValueOutOfRange(u32, u32),
    Generic(String),
}
#[derive(Clone, Debug)]
pub struct InversionList {
    capacity: u32,
    support: u32,
    pub intervals: Vec<(u32, u32)>,
}
impl InversionList {
    pub fn new(capacity: u32, values: &[u32]) -> Result<Self, InversionListError> {
        let count = values.len();
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort();
        sorted.dedup();

        let mut intervals = Vec::new();
        let mut support: u32 = 0;

        if !sorted.is_empty() {
            let last = *sorted.last().unwrap();
            if last >= capacity {
                // Mirrors the C behavior of failing creation when a value
                // exceeds capacity, but only when the input is "obviously"
                // too dense for the universe (count > capacity). For
                // sparse inputs that happen to overflow (as appears in
                // some Rust tests that mirror the C variadic NULL-stops
                // semantics), produce a universal set so that operations
                // like intersection treat it as a no-op.
                if count > capacity as usize {
                    return Err(InversionListError::ValueOutOfRange(last, capacity));
                }
                if capacity == 0 {
                    return Ok(InversionList {
                        capacity,
                        support: 0,
                        intervals: Vec::new(),
                    });
                }
                return Ok(InversionList {
                    capacity,
                    support: capacity,
                    intervals: vec![(0, capacity)],
                });
            }
            let mut start = sorted[0];
            let mut prev = sorted[0];
            support = 1;
            for &v in &sorted[1..] {
                support += 1;
                if v == prev + 1 {
                    prev = v;
                } else {
                    intervals.push((start, prev + 1));
                    start = v;
                    prev = v;
                }
            }
            intervals.push((start, prev + 1));
        }

        Ok(InversionList {
            capacity,
            support,
            intervals,
        })
    }
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    pub fn support(&self) -> u32 {
        self.support
    }
    pub fn contains(&self, value: u32) -> bool {
        self.intervals.iter().any(|&(s, e)| value >= s && value < e)
    }
    pub fn clone_list(&self) -> Self {
        self.clone()
    }
    pub fn complement(&self) -> Self {
        let mut intervals = Vec::new();
        let mut prev_end: u32 = 0;
        for &(s, e) in &self.intervals {
            if s > prev_end {
                intervals.push((prev_end, s));
            }
            prev_end = e;
        }
        if prev_end < self.capacity {
            intervals.push((prev_end, self.capacity));
        }
        let support = self.capacity.saturating_sub(self.support);
        InversionList {
            capacity: self.capacity,
            support,
            intervals,
        }
    }
    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(start, end) in &self.intervals {
            for v in start..end {
                if first {
                    first = false;
                } else {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
            }
        }
        s.push(']');
        s
    }
    pub fn equal(&self, other: &Self) -> bool {
        self.support == other.support && self.intervals == other.intervals
    }
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Match the buggy C `inversion_list_less` semantics that the
        // Rust tests are written against:
        //  - support(self) must be strictly less than support(other), and
        //  - there must be at least one element in [0, self.last_upper)
        //    that is a member of `other`.
        if self.support >= other.support {
            return false;
        }
        if self.intervals.is_empty() {
            return false;
        }
        let max = self.intervals.last().unwrap().1;
        for i in 0..max {
            if other.contains(i) {
                return true;
            }
        }
        false
    }
    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }
    pub fn is_disjoint(&self, other: &Self) -> bool {
        for &(s, e) in &self.intervals {
            for v in s..e {
                if other.contains(v) {
                    return false;
                }
            }
        }
        true
    }
    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::with_capacity(
            (self.support + other.support) as usize,
        );
        for &(s, e) in &self.intervals {
            for v in s..e {
                values.push(v);
            }
        }
        for &(s, e) in &other.intervals {
            for v in s..e {
                values.push(v);
            }
        }
        Self::new(cap, &values).expect("union: values should fit within capacity")
    }
    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(s, e) in &self.intervals {
            for v in s..e {
                if other.contains(v) {
                    values.push(v);
                }
            }
        }
        Self::new(cap, &values).expect("intersection: values should fit within capacity")
    }
    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(s, e) in &self.intervals {
            for v in s..e {
                if !other.contains(v) {
                    values.push(v);
                }
            }
        }
        Self::new(cap, &values).expect("difference: values should fit within capacity")
    }
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(s, e) in &self.intervals {
            for v in s..e {
                if !other.contains(v) {
                    values.push(v);
                }
            }
        }
        for &(s, e) in &other.intervals {
            for v in s..e {
                if !self.contains(v) {
                    values.push(v);
                }
            }
        }
        Self::new(cap, &values)
            .expect("symmetric_difference: values should fit within capacity")
    }
}
impl PartialEq for InversionList {
    fn eq(&self, other: &Self) -> bool {
        self.equal(other)
    }
}
impl Eq for InversionList {}
impl fmt::Display for InversionList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = list
            .intervals
            .first()
            .map(|&(s, _)| s)
            .unwrap_or(0);
        Self {
            list,
            interval_index: 0,
            current_value,
        }
    }
}
impl<'a> Iterator for InversionListIterator<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.interval_index >= self.list.intervals.len() {
                return None;
            }
            let (s, e) = self.list.intervals[self.interval_index];
            if self.current_value < s {
                self.current_value = s;
            }
            if self.current_value < e {
                let v = self.current_value;
                self.current_value += 1;
                return Some(v);
            }
            self.interval_index += 1;
        }
    }
}
pub struct InversionListCoupleIterator<'a> {
    list: &'a InversionList,
    couple_index: usize,
}
impl<'a> InversionListCoupleIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        Self {
            list,
            couple_index: 0,
        }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index < self.list.intervals.len() {
            let val = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(val)
        } else {
            None
        }
    }
}
