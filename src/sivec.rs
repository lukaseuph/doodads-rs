use itertools::Itertools;
use std::ops::{Deref, Index, IndexMut};

/// A stable-index vector.
///
/// `SiVec` supports indexing but not slicing because the data structure
/// cannot guarantee that the underlying [`Vec`] is in the same order.
/// As such, `SiVec` should be used where order of elements is irrelevant.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SiVec<T> {
    /// Stores the index of the data that the stable index points to.
    slot_to_pos: Vec<usize>,
    /// Maps indexes in `data` back to the corresponding `data_index`.
    pos_to_slot: Vec<usize>,
    /// The actual elements of the vector.
    data: Vec<T>,
}
impl<T> SiVec<T> {
    pub fn new() -> Self {
        Self {
            slot_to_pos: vec![],
            pos_to_slot: vec![],
            data: vec![],
        }
    }

    fn position(&self, index: usize) -> Option<usize> {
        let pos = *self.slot_to_pos.get(index)?;
        // `pos < len` rules out freed ids; `id[pos] == index` rules out
        // freed ids whose stale position has since been reused by another element.
        (pos < self.data.len() && self.pos_to_slot[pos] == index).then_some(pos)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.position(index).map(|p| &self.data[p])
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let pos = self.position(index)?;
        Some(&mut self.data[pos])
    }

    pub fn insert(&mut self, element: T) -> usize {
        if self.data.len() == self.pos_to_slot.len() {
            let previous_capacity = self.data.len();
            self.slot_to_pos.push(previous_capacity);
            self.pos_to_slot.push(previous_capacity);
            self.data.push(element);
            previous_capacity
        } else {
            let new_position = self.data.len();
            let reused_id = self.pos_to_slot[new_position];
            self.slot_to_pos[reused_id] = new_position;
            self.data.push(element);
            reused_id
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        let physical_pos = self.position(index)?;
        let last_index = self.data.len() - 1;
        self.data.swap(physical_pos, last_index);
        self.pos_to_slot.swap(physical_pos, last_index);
        // update data index
        self.slot_to_pos[self.pos_to_slot[physical_pos]] = physical_pos;
        self.slot_to_pos[self.pos_to_slot[last_index]] = last_index;
        // pop
        self.data.pop()
    }
}
macro_rules! impl_from_for_si_vec {
    ($( [$($generics:tt)*] $src:ty ),* $(,)?) => {
        $(
            impl<$($generics)*> From<$src> for SiVec<T> {
                fn from(value: $src) -> Self {
                    let data = Vec::from(value);
                    let pos_to_slot = (0..data.len()).collect_vec();
                    let slot_to_pos = pos_to_slot.clone();
                    Self { slot_to_pos, pos_to_slot, data }
                }
            }
        )+
    };
}
impl_from_for_si_vec!(
    [const N: usize, T] [T; N],
    [const N: usize, T: Clone] &[T; N],
    [T: Clone] &[T],
    [T] Vec<T>,
);
impl<T> FromIterator<T> for SiVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(Vec::from_iter(iter))
    }
}
impl<T> Index<usize> for SiVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index)
            .expect("index refers to a removed or invalid id")
    }
}
impl<T> IndexMut<usize> for SiVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index)
            .expect("index refers to a removed or invalid id")
    }
}
impl<T> Deref for SiVec<T> {
    type Target = [T];

    /// Returns the underlying data.
    /// Note that this slice is not necessarily in the order that the `SiVec` would suggest.
    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_array() {
        let a = SiVec::from([1, 2, 3]);
        let mut b = SiVec::<i32>::new();
        b.insert(1);
        b.insert(2);
        b.insert(3);
        assert_eq!(a, b);
    }

    #[test]
    fn from_array_reference() {
        let a = SiVec::from(&[1, 2, 3]);
        let mut b = SiVec::<i32>::new();
        b.insert(1);
        b.insert(2);
        b.insert(3);
        assert_eq!(a, b);
    }

    #[test]
    fn from_slice() {
        let a = SiVec::from([1, 2, 3].as_slice());
        let mut b = SiVec::<i32>::new();
        b.insert(1);
        b.insert(2);
        b.insert(3);
        assert_eq!(a, b);
    }

    #[test]
    fn from_vec() {
        let a = SiVec::from(vec![1, 2, 3]);
        let mut b = SiVec::<i32>::new();
        b.insert(1);
        b.insert(2);
        b.insert(3);
        assert_eq!(a, b);
    }

    #[test]
    fn verify_empty() {
        let siv = SiVec::<i32>::new();
        assert!(siv.is_empty());

        let mut siv = SiVec::from([1, 2, 3]);
        assert!(!siv.is_empty());
        siv.remove(0);
        siv.remove(1);
        siv.remove(2);
        assert!(siv.is_empty());
    }

    #[test]
    fn remove_middle_element() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(1);
        assert_eq!(siv.len(), 2);
        assert_eq!(*siv, [1, 3]);
        assert_eq!(siv[0], 1);
        assert_eq!(siv[2], 3);
    }

    #[test]
    fn remove_insert() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(1);
        assert_eq!(*siv, [1, 3]);
        siv.insert(4);
        assert_eq!(*siv, [1, 3, 4]);
        assert_eq!(siv[0], 1);
        assert_eq!(siv[1], 4);
        assert_eq!(siv[2], 3);
    }

    #[test]
    fn get_element() {
        let siv = SiVec::from([1, 2, 3]);
        assert_eq!(siv.get(0), Some(&1));
        assert_eq!(siv.get(1), Some(&2));
        assert_eq!(siv.get(2), Some(&3));
        assert_eq!(siv.get(3), None);
    }

    #[test]
    fn get_after_remove_uses_stable_ids() {
        let mut siv = SiVec::from([10, 20, 30]);
        siv.remove(0);
        // len is 2, but ids 1 and 2 are still live
        assert_eq!(siv.get(1), Some(&20));
        assert_eq!(siv.get(2), Some(&30));
    }

    #[test]
    fn get_removed_id_is_none() {
        let mut siv = SiVec::from([10, 20, 30]);
        siv.remove(1);
        assert_eq!(siv.get(1), None); // currently passes the bound check, then panics
    }

    #[test]
    fn get_mut_modifies_element() {
        let mut siv = SiVec::from([1, 2, 3]);
        *siv.get_mut(1).unwrap() = 20;
        assert_eq!(siv[1], 20);
    }

    #[test]
    fn get_mut_after_removal_uses_stable_ids() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(0);
        *siv.get_mut(2).unwrap() += 10;
        assert_eq!(siv[2], 13);
    }

    #[test]
    fn get_mut_invalid_ids_return_none() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(1);
        assert!(siv.get_mut(1).is_none()); // removed
        assert!(siv.get_mut(10).is_none()); // never issued
    }

    #[test]
    fn remove_invalid_ids_return_none() {
        let mut siv = SiVec::from([10, 20]);
        assert_eq!(siv.remove(5), None); // out of range
        assert_eq!(siv.remove(0), Some(10));
        assert_eq!(siv.remove(0), None); // already removed
        assert_eq!(siv[1], 20); // structure not corrupted
        assert_eq!(siv.len(), 1);
    }

    #[test]
    fn remove_from_empty_returns_none() {
        let mut siv = SiVec::<i32>::new();
        assert_eq!(siv.remove(0), None);
    }

    #[test]
    fn insert_returns_stable_ids() {
        let mut siv = SiVec::new();
        assert_eq!(siv.insert('a'), 0);
        assert_eq!(siv.insert('b'), 1);
        assert_eq!(siv.insert('c'), 2);
    }

    #[test]
    fn removed_id_is_reused() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(1);
        assert_eq!(siv.insert(4), 1);
        assert_eq!(siv[1], 4);
    }

    #[test]
    fn free_list_reuses_in_lifo_order() {
        let mut siv = SiVec::from([1, 2, 3, 4]);
        siv.remove(1);
        siv.remove(2);
        // Pin down whichever order you actually want; this documents it.
        let first = siv.insert(10);
        let second = siv.insert(20);
        assert_eq!((first, second), (2, 1));
        assert_eq!(siv[1], 20);
        assert_eq!(siv[2], 10);
    }

    #[test]
    fn remove_everything_then_refill() {
        let mut siv = SiVec::from([1, 2, 3]);
        for id in 0..3 {
            siv.remove(id);
        }
        assert!(siv.is_empty());
        let ids: Vec<_> = (0..3).map(|n| siv.insert(n)).collect();
        assert_eq!(siv.len(), 3);
        for (id, n) in ids.iter().zip(0..3) {
            assert_eq!(siv[*id], n);
        }
    }

    #[test]
    fn remove_last_element() {
        let mut siv = SiVec::from([1, 2, 3]);
        assert_eq!(siv.remove(2), Some(3)); // swap with itself
        assert_eq!(*siv, [1, 2]);
        assert_eq!(siv[0], 1);
        assert_eq!(siv[1], 2);
    }

    #[test]
    fn remove_first_element_moves_last_into_place() {
        let mut siv = SiVec::from([1, 2, 3]);
        assert_eq!(siv.remove(0), Some(1));
        assert_eq!(*siv, [3, 2]); // physical order changed
        assert_eq!(siv[1], 2);
        assert_eq!(siv[2], 3); // but stable ids didn't
    }

    #[test]
    fn remove_returns_the_removed_value() {
        let mut siv = SiVec::from([1, 2, 3]);
        assert_eq!(siv.remove(1), Some(2));
    }

    #[test]
    fn single_element_roundtrip() {
        let mut siv = SiVec::new();
        let id = siv.insert(7);
        assert_eq!(siv.remove(id), Some(7));
        assert!(siv.is_empty());
    }

    #[test]
    fn index_mut_after_removal() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(0);
        siv[2] += 10;
        assert_eq!(siv[2], 13);
        assert_eq!(siv[1], 2);
    }

    #[test]
    fn works_with_non_copy_types() {
        let mut siv = SiVec::from(vec!["a".to_string(), "b".to_string()]);
        let removed = siv.remove(0);
        assert_eq!(removed.as_deref(), Some("a"));
        assert_eq!(siv[1], "b");
    }

    #[test]
    fn from_iterator() {
        let siv: SiVec<i32> = (1..=3).collect();
        assert_eq!(siv, SiVec::from([1, 2, 3]));
    }

    #[test]
    fn default_is_empty() {
        assert!(SiVec::<i32>::default().is_empty());
    }

    #[test]
    fn matches_hashmap_model() {
        use std::collections::HashMap;

        // Tiny deterministic PRNG so the test needs no dependencies.
        let mut seed: u64 = 0x2545F4914F6CDD1D;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };

        let mut siv = SiVec::new();
        let mut model: HashMap<usize, u64> = HashMap::new();
        let mut max_id = 0;

        for step in 0..5000u64 {
            if next() % 3 != 0 {
                let id = siv.insert(step);
                assert!(model.insert(id, step).is_none(), "id {id} issued twice");
                max_id = max_id.max(id + 1);
            } else {
                let id = (next() % (max_id as u64 + 2)) as usize; // sometimes out of range
                assert_eq!(siv.remove(id), model.remove(&id));
            }

            assert_eq!(siv.len(), model.len());
            for id in 0..max_id + 2 {
                assert_eq!(
                    siv.get(id),
                    model.get(&id),
                    "mismatch at id {id}, step {step}"
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "removed or invalid id")]
    fn index_out_of_range_panics() {
        let siv = SiVec::from([1, 2, 3]);
        let _ = siv[10];
    }

    #[test]
    #[should_panic(expected = "removed or invalid id")]
    fn index_removed_id_panics() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(1);
        let _ = siv[1];
    }

    #[test]
    #[should_panic(expected = "removed or invalid id")]
    fn index_removed_id_panics_even_when_position_was_reused() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(0); // 3 moves into physical position 0
        let _ = siv[0]; // stale id 0 must not alias the moved element
    }

    #[test]
    #[should_panic(expected = "removed or invalid id")]
    fn index_mut_removed_id_panics() {
        let mut siv = SiVec::from([1, 2, 3]);
        siv.remove(1);
        siv[1] = 99;
    }

    #[test]
    #[should_panic(expected = "removed or invalid id")]
    fn index_empty_panics() {
        let siv = SiVec::<i32>::new();
        let _ = siv[0];
    }
}
