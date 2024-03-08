use pyo3::prelude::*;

// Internal implementations
mod classes;
mod internal;

#[pymodule]
#[pyo3(name = "_cachebox")]
fn _cachebox(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "aWolverP")?;
    m.add_class::<classes::BaseCacheImpl>()?;
    m.add_class::<classes::Cache>()?;
    m.add_class::<classes::FIFOCache>()?;
    m.add_class::<classes::LFUCache>()?;
    m.add_class::<classes::LRUCache>()?;
    m.add_class::<classes::RRCache>()?;
    m.add_class::<classes::TTLCache>()?;
    m.add_class::<classes::VTTLCache>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BinaryHeap;
    use std::time::Instant;

    use rand::seq::SliceRandom;

    #[test]
    fn test_() {
        let n: usize = 10_000_000;
        let mut a_elements_in_random_order: Vec<usize> = Vec::from_iter(0..n);
        a_elements_in_random_order.shuffle(&mut rand::thread_rng());

        let heap: BinaryHeap<usize> = BinaryHeap::from(a_elements_in_random_order.clone());
        let mut a_sorted_by_sort_unstable = a_elements_in_random_order.clone();

        let now = Instant::now();
        a_sorted_by_sort_unstable.sort_unstable();
        let runtime_sort_unstable = now.elapsed();

        let now = Instant::now();
        let a_sorted_by_heap = heap.into_sorted_vec();
        let runtime_sorted_by_heap = now.elapsed();
        
        assert!(a_sorted_by_sort_unstable == a_sorted_by_heap);
        println!("runtime_sort_unstable={:?}", runtime_sort_unstable);
        println!("runtime_sorted_by_heap={:?}", runtime_sorted_by_heap);
    }
}
