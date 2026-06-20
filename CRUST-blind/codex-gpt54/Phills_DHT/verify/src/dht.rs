pub mod dht {
    pub struct HASHTABLE<T> {
        pub hash_table: Vec<Option<T>>, 
        pub lower_bound: i32, 
        pub higher_bound: i32,
    }
    impl <T> HASHTABLE<T> {
        pub fn dht_init() -> HASHTABLE<T> {
            HASHTABLE {
                hash_table: Vec::new(),
                lower_bound: 0,
                higher_bound: 0,
            }
        }
        pub fn dht_is_initialised(&self) -> bool {
            !self.hash_table.is_empty()
        }
        pub fn dht_init_table(&mut self, new_lower_bound: u32, new_upper_bound: u32, migrate: bool) -> bool {
            assert!(new_upper_bound > new_lower_bound);

            let new_hash_table_size = (new_upper_bound - new_lower_bound) as usize;
            let mut new_hash_table = Vec::new();

            if new_hash_table.try_reserve_exact(new_hash_table_size).is_err() {
                return false;
            }

            new_hash_table.resize_with(new_hash_table_size, || None);

            if self.dht_is_initialised() {
                let old_lower_bound = self.lower_bound as u32;

                for (i, item) in self.hash_table.iter_mut().enumerate() {
                    if let Some(value) = item.take() {
                        let absolute_position = i as u32 + old_lower_bound;

                        if migrate
                            && absolute_position >= new_lower_bound
                            && absolute_position < new_upper_bound
                        {
                            new_hash_table[(absolute_position - new_lower_bound) as usize] = Some(value);
                        }
                    }
                }
            }

            self.hash_table = new_hash_table;
            self.lower_bound = new_lower_bound as i32;
            self.higher_bound = new_upper_bound as i32;
            true
        }
        pub fn dht_get_lower_bound(&self) -> u32 {
            assert!(self.dht_is_initialised());
            self.lower_bound as u32
        }
        pub fn dht_get_upper_bound(&self) -> u32 {
            assert!(self.dht_is_initialised());
            self.higher_bound as u32
        }
        pub fn dht_get_size(&self) -> u32 {
            assert!(self.dht_is_initialised());
            (self.higher_bound - self.lower_bound) as u32
        }
        pub fn dht_read(&self, location: u32) -> &Option<T> {
            assert!(self.dht_is_initialised());
            assert!(location >= self.lower_bound as u32);
            assert!(location < self.higher_bound as u32);

            &self.hash_table[(location - self.lower_bound as u32) as usize]
        }
        pub fn dht_write(&mut self, location: u32, data: T) {
            assert!(self.dht_is_initialised());
            assert!(location >= self.lower_bound as u32);
            assert!(location < self.higher_bound as u32);

            self.hash_table[(location - self.lower_bound as u32) as usize] = Some(data);
        }
    }
}
