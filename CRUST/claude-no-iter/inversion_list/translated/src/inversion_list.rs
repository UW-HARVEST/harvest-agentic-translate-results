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
        // Sort and deduplicate the values
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort_unstable();
        sorted.dedup();

        // Empty input: empty set
        if sorted.is_empty() {
            return Ok(InversionList {
                capacity,
                support: 0,
                intervals: Vec::new(),
            });
        }

        // Validate that all values are within capacity (max < capacity)
        let max_val = *sorted.last().unwrap();
        if max_val >= capacity {
            return Err(InversionListError::ValueOutOfRange(max_val, capacity));
        }

        // Build intervals by grouping consecutive integers
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut start = sorted[0];
        let mut end = start + 1; // half-open: [start, end)
        for &v in sorted.iter().skip(1) {
            if v == end {
                end = v + 1;
            } else {
                intervals.push((start, end));
                start = v;
                end = v + 1;
            }
        }
        intervals.push((start, end));

        let support = sorted.len() as u32;
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
        for &(lo, hi) in &self.intervals {
            if value >= lo && value < hi {
                return true;
            }
            if value < lo {
                return false;
            }
        }
        false
    }

    pub fn clone_list(&self) -> Self {
        self.clone()
    }

    pub fn complement(&self) -> Self {
        let mut new_intervals: Vec<(u32, u32)> = Vec::new();
        let mut prev: u32 = 0;
        for &(lo, hi) in &self.intervals {
            if prev < lo {
                new_intervals.push((prev, lo));
            }
            prev = hi;
        }
        if prev < self.capacity {
            new_intervals.push((prev, self.capacity));
        }
        let support = self.capacity - self.support;
        InversionList {
            capacity: self.capacity,
            support,
            intervals: new_intervals,
        }
    }

    pub fn to_str(&self) -> String {
        let mut s = String::from("[");
        let mut first = true;
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
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
        // Mirrors the C `inversion_list_less` semantics:
        //   1) self.support must be strictly less than other.support
        //   2) at least one element of `other` must lie in [0, last_upper_bound_of_self)
        if self.support >= other.support {
            return false;
        }
        if self.intervals.is_empty() {
            return false;
        }
        let upper = self.intervals.last().unwrap().1;
        for i in 0..upper {
            if other.contains(i) {
                return true;
            }
        }
        false
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        // Mirrors the C `inversion_list_less_equal`: equal OR less.
        self.equal(other) || self.is_strict_subset_of(other)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        // Two interval lists are disjoint when no interval pair overlaps
        let mut i = 0;
        let mut j = 0;
        while i < self.intervals.len() && j < other.intervals.len() {
            let (a_lo, a_hi) = self.intervals[i];
            let (b_lo, b_hi) = other.intervals[j];
            // Half-open overlap if a_lo < b_hi && b_lo < a_hi
            if a_lo < b_hi && b_lo < a_hi {
                return false;
            }
            if a_hi <= b_lo {
                i += 1;
            } else {
                j += 1;
            }
        }
        true
    }

    pub fn union(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                values.push(v);
            }
        }
        for &(lo, hi) in &other.intervals {
            for v in lo..hi {
                values.push(v);
            }
        }
        InversionList::new(cap, &values).expect("union: values out of range")
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if other.contains(v) {
                    values.push(v);
                }
            }
        }
        InversionList::new(cap, &values).expect("intersection: values out of range")
    }

    pub fn difference(&self, other: &Self) -> Self {
        let cap = self.capacity.max(other.capacity);
        let mut values: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if !other.contains(v) {
                    values.push(v);
                }
            }
        }
        InversionList::new(cap, &values).expect("difference: values out of range")
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let union = self.union(other);
        let intersection = self.intersection(other);
        union.difference(&intersection)
    }
}
impl PartialEq for InversionList {
    fn eq(&self, other: &Self) -> bool {
        self.capacity == other.capacity
            && self.support == other.support
            && self.intervals == other.intervals
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
        let current_value = if list.intervals.is_empty() {
            0
        } else {
            list.intervals[0].0
        };
        InversionListIterator {
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
            let (lo, hi) = self.list.intervals[self.interval_index];
            if self.current_value < lo {
                self.current_value = lo;
            }
            if self.current_value < hi {
                let v = self.current_value;
                self.current_value += 1;
                return Some(v);
            } else {
                self.interval_index += 1;
                if self.interval_index < self.list.intervals.len() {
                    self.current_value = self.list.intervals[self.interval_index].0;
                }
            }
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
        InversionListCoupleIterator {
            list,
            couple_index: 0,
        }
    }
}
impl<'a> Iterator for InversionListCoupleIterator<'a> {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<Self::Item> {
        if self.couple_index < self.list.intervals.len() {
            let pair = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(pair)
        } else {
            None
        }
    }
}
