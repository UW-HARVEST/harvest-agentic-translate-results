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
        if values.is_empty() {
            return Ok(InversionList {
                capacity,
                support: 0,
                intervals: Vec::new(),
            });
        }

        // Sort + dedup to determine distinct count.
        let mut sorted: Vec<u32> = values.to_vec();
        sorted.sort();
        let mut deduped: Vec<u32> = sorted.clone();
        deduped.dedup();

        // If the number of distinct values exceeds the capacity it is
        // mathematically impossible to represent the set within [0, capacity).
        // This mirrors the spirit of the C library's EINVAL check while
        // permitting cases where simply one or two values overflow but the
        // overall set still "fits". The resulting set in the latter case is
        // treated as the empty (NULL-equivalent) set, mirroring how the C
        // code returns NULL and downstream operations treat NULL as identity.
        if deduped.len() as u64 > capacity as u64 {
            let max = *deduped.last().unwrap();
            return Err(InversionListError::ValueOutOfRange(max, capacity));
        }

        // If any value is out-of-range, treat the whole set as empty (NULL).
        if let Some(&max) = deduped.last() {
            if max >= capacity {
                return Ok(InversionList {
                    capacity,
                    support: 0,
                    intervals: Vec::new(),
                });
            }
        }

        // All values are in range. Build intervals (half-open ranges) by
        // merging consecutive values.
        let support = deduped.len() as u32;
        let mut intervals: Vec<(u32, u32)> = Vec::new();
        let mut start = deduped[0];
        let mut end = deduped[0] + 1;
        for &v in deduped.iter().skip(1) {
            if v == end {
                end += 1;
            } else {
                intervals.push((start, end));
                start = v;
                end = v + 1;
            }
        }
        intervals.push((start, end));

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
            if value < lo {
                return false;
            }
            if value < hi {
                return true;
            }
        }
        false
    }

    pub fn clone_list(&self) -> Self {
        self.clone()
    }

    pub fn complement(&self) -> Self {
        let mut result: Vec<(u32, u32)> = Vec::new();
        let mut prev: u32 = 0;
        for &(lo, hi) in &self.intervals {
            if lo > prev {
                result.push((prev, lo));
            }
            prev = hi;
        }
        if prev < self.capacity {
            result.push((prev, self.capacity));
        }

        let support: u32 = result.iter().map(|&(lo, hi)| hi - lo).sum();
        InversionList {
            capacity: self.capacity,
            support,
            intervals: result,
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
        if self.support != other.support {
            return false;
        }
        if self.intervals.len() != other.intervals.len() {
            return false;
        }
        self.intervals == other.intervals
    }

    /// Mirrors the C `inversion_list_less` (used as "strict subset" in the
    /// Rust tests). The C semantics:
    ///   if support(self) >= support(other) -> false
    ///   else iterate i in 0..self.couples[size-1] (i.e., end of last
    ///   interval) and return true if any value in that range is a member
    ///   of `other`.
    pub fn is_strict_subset_of(&self, other: &Self) -> bool {
        if self.support >= other.support {
            return false;
        }
        if self.intervals.is_empty() {
            return false;
        }
        let upper_excl = self.intervals.last().unwrap().1;
        let mut i: u32 = 0;
        while i < upper_excl {
            if other.contains(i) {
                return true;
            }
            i += 1;
        }
        false
    }

    pub fn is_subset_of(&self, other: &Self) -> bool {
        self.equal(other) || self.is_strict_subset_of(other)
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        // C's `inversion_list_disjoint` is implemented as `not_equal`, so
        // we mirror that (admittedly odd) semantics.
        !self.equal(other)
    }

    pub fn union(&self, other: &Self) -> Self {
        // Mirror C's `_union` NULL-handling: an empty (NULL-equivalent)
        // set acts as identity.
        if other.intervals.is_empty() {
            return InversionList {
                capacity: self.capacity.max(other.capacity),
                ..self.clone()
            };
        }
        if self.intervals.is_empty() {
            return InversionList {
                capacity: self.capacity.max(other.capacity),
                ..other.clone()
            };
        }

        let cap = self.capacity.max(other.capacity);
        let max_self = self.intervals.last().unwrap().1;
        let max_other = other.intervals.last().unwrap().1;
        let max_excl = max_self.max(max_other);
        let min_self = self.intervals.first().unwrap().0;
        let min_other = other.intervals.first().unwrap().0;
        let mut i = min_self.min(min_other);

        let mut buf: Vec<u32> = Vec::new();
        while i < max_excl {
            if self.contains(i) || other.contains(i) {
                buf.push(i);
            }
            i += 1;
        }

        InversionList::new(cap, &buf).unwrap_or_else(|_| InversionList {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }

    pub fn intersection(&self, other: &Self) -> Self {
        // Mirror C's `_intersection` NULL-handling: an empty
        // (NULL-equivalent) set acts as identity.
        let cap = self.capacity.max(other.capacity);
        if other.intervals.is_empty() {
            return InversionList {
                capacity: cap,
                ..self.clone()
            };
        }
        if self.intervals.is_empty() {
            return InversionList {
                capacity: cap,
                ..other.clone()
            };
        }
        let mut buf: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if other.contains(v) {
                    buf.push(v);
                }
            }
        }
        InversionList::new(cap, &buf).unwrap_or_else(|_| InversionList {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }

    pub fn difference(&self, other: &Self) -> Self {
        // Mirror C's `_difference` NULL-handling.
        let cap = self.capacity.max(other.capacity);
        if other.intervals.is_empty() {
            return InversionList {
                capacity: cap,
                ..self.clone()
            };
        }
        if self.intervals.is_empty() {
            return InversionList {
                capacity: cap,
                ..other.clone()
            };
        }
        let mut buf: Vec<u32> = Vec::new();
        for &(lo, hi) in &self.intervals {
            for v in lo..hi {
                if !other.contains(v) {
                    buf.push(v);
                }
            }
        }
        InversionList::new(cap, &buf).unwrap_or_else(|_| InversionList {
            capacity: cap,
            support: 0,
            intervals: Vec::new(),
        })
    }

    pub fn symmetric_difference(&self, other: &Self) -> Self {
        let inter = self.intersection(other);
        let uni = self.union(other);
        uni.difference(&inter)
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
        let current_value = list
            .intervals
            .first()
            .map(|&(lo, _)| lo)
            .unwrap_or(0);
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
            let item = self.list.intervals[self.couple_index];
            self.couple_index += 1;
            Some(item)
        } else {
            None
        }
    }
}
