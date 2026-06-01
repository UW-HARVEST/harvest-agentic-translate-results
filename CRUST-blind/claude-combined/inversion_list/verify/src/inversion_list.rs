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
        let mut buffer: Vec<u32> = values.to_vec();
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut support: u32 = 0;

        if !buffer.is_empty() {
            buffer.sort();
            let last_val = *buffer.last().unwrap();
            if last_val >= capacity {
                return Err(InversionListError::ValueOutOfRange(last_val, capacity));
            }
            support = 1;
            let first = buffer[0];
            intervals.push((first, first + 1));
            for i in 1..buffer.len() {
                if buffer[i] != buffer[i - 1] {
                    support += 1;
                    let last_idx = intervals.len() - 1;
                    if buffer[i] == intervals[last_idx].1 {
                        intervals[last_idx].1 += 1;
                    } else {
                        intervals.push((buffer[i], buffer[i] + 1));
                    }
                }
            }
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
        for &(start, end) in &self.intervals {
            if value >= start && value < end {
                return true;
            }
        }
        false
    }

    pub fn clone_list(&self) -> Self {
        self.clone()
    }

    pub fn complement(&self) -> Self {
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev: u32 = 0;
        for &(start, end) in &self.intervals {
            if start > prev {
                intervals.push((prev, start));
            }
            prev = end;
        }
        if prev < self.capacity {
            intervals.push((prev, self.capacity));
        }
        let support = self.capacity - self.support;
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
                if !first {
                    s.push_str(", ");
                }
                s.push_str(&v.to_string());
                first = false;
            }
        }
        s.push(']');
        s
    }

    pub fn equal(&self, other: &Self) -> bool {
        if self.support != other.support {
            return false;
        }
        if self.intervals.len() != other.intervals.len() {
            return false;
        }
        // Compare flat couple array as the C code does (in_list)
        for i in 0..self.intervals.len() {
            if self.intervals[i] != other.intervals[i] {
                return false;
            }
        }
        true
    }

    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        // Matches C inversion_list_less:
        //   if support(self) >= support(other) -> false
        //   else iterate i from 0 to self.last_interval_end-1
        //   return true if any i is a member of `other`
        if self.support >= other.support {
            return false;
        }
        if self.intervals.is_empty() {
            return false;
        }
        let max = self.intervals.last().unwrap().1;
        let mut i: u32 = 0;
        while i < max {
            if other.contains(i) {
                return true;
            }
            i += 1;
        }
        false
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        // Matches C inversion_list_less_equal: equal || less
        self.equal(other) || self.is_strict_subset_of(other)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Matches C inversion_list_disjoint which is implemented as !equal
        !self.equal(other)
    }

    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
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
        Self::new(cap, &values).expect("union: values must be < capacity")
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
        Self::new(cap, &values).expect("intersection: values must be < capacity")
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
        Self::new(cap, &values).expect("difference: values must be < capacity")
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        // (self ∪ other) \ (self ∩ other), as in C
        let u = self.union(other);
        let i = self.intersection(other);
        u.difference(&i)
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
        f.write_str(&self.to_str())
    }
}
pub struct InversionListIterator<'a> {
    list: &'a InversionList,
    interval_index: usize,
    current_value: u32,
}
impl<'a> InversionListIterator<'a> {
    pub fn new(list: &'a InversionList) -> Self {
        let current_value = if let Some(&(start, _)) = list.intervals.first() {
            start
        } else {
            0
        };
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
        while self.interval_index < self.list.intervals.len() {
            let (start, end) = self.list.intervals[self.interval_index];
            if self.current_value < start {
                self.current_value = start;
            }
            if self.current_value < end {
                let v = self.current_value;
                self.current_value += 1;
                return Some(v);
            }
            self.interval_index += 1;
        }
        None
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
            let c = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(c)
        } else {
            None
        }
    }
}
