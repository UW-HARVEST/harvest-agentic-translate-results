use Megalania::max_heap::MaxHeap;

fn main() {}

fn initialize_random_data(data: &mut [u32], _seed: u64) {
    use rand::Rng;
    let mut rng = rand::rng();
    for i in 0..data.len() {
        data[i] = i as u32;
    }
    for i in 0..data.len() {
        let j = rng.random_range(0..=i);
        data.swap(i, j);
    }
}

#[test]
fn max_heap_sort_test() {
    let heap_size = 10;
    let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(|a, b| {
        (a as i32) - (b as i32)
    });
    let mut heap = MaxHeap::new(heap_size, comparator);

    let mut data = vec![0u32; heap_size];
    initialize_random_data(&mut data, 666);

    for i in 0..heap_size {
        assert!(heap.insert(data[i]), "Could not insert into heap!");
    }

    assert_eq!(heap.count(), heap_size, "Heap wrong size!");

    for i in 0..heap_size {
        let expected_value = (heap_size - i - 1) as u32;
        let value = heap.maximum().expect("Could not get heap maximum!");
        assert_eq!(value, expected_value, "Maximum value in heap unexpected!");
        assert!(heap.remove_maximum(), "Could not pop heap!");
    }

    assert_eq!(heap.count(), 0, "Heap wrong size!");
}

#[test]
fn max_heap_top_k_test() {
    let heap_size = 10;
    let backing_store: Vec<u32> = vec![0; heap_size];
    let backing_store_ptr = Box::into_raw(Box::new(backing_store));

    let comparator: Box<dyn Fn(u32, u32) -> i32> = Box::new(move |a, b| {
        let bs = unsafe { &*backing_store_ptr };
        (bs[a as usize] as i32) - (bs[b as usize] as i32)
    });
    let mut heap = MaxHeap::new(heap_size, comparator);

    let data_size = 100;
    let mut data = vec![0u32; data_size];
    initialize_random_data(&mut data, 666);

    let mut data_count = 0usize;

    for i in 0..data_size {
        let bs = unsafe { &mut *backing_store_ptr };
        if data_count < heap_size {
            let pos = data_count;
            data_count += 1;
            bs[pos] = data[i];
            heap.insert(pos as u32);
            continue;
        }
        let maximum = heap.maximum().expect("Could not get heap maximum!");
        if data[i] < bs[maximum as usize] {
            bs[maximum as usize] = data[i];
            heap.update_maximum();
        }
    }

    for i in 0..heap_size {
        let expected_value = (heap_size - i - 1) as u32;
        let value = heap.maximum().expect("Could not get heap maximum!");
        let bs = unsafe { &*backing_store_ptr };
        assert!(
            (value as usize) < heap_size,
            "Maximum value exceeds backing_store data size!"
        );
        assert_eq!(
            bs[value as usize], expected_value,
            "Maximum value in heap unexpected!"
        );
        assert!(heap.remove_maximum(), "Could not pop heap!");
    }

    // Clean up
    unsafe { drop(Box::from_raw(backing_store_ptr)); }
}
