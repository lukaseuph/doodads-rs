use std::ops::{Deref, Index, IndexMut};
use itertools::Itertools;

/// A stable-index vector.
///
/// `SiVec` supports indexing but not slicing because the data structure cannot guarantee that the underlying [`Vec`] is in the same
/// order. As such, `SiVec` should be used where order of elements is irrelevant.
#[derive(Debug, Default, PartialEq, Clone)]
struct SiVec<T> {
    /// Stores the index of the data that the stable index points to.
    data_index: Vec<usize>,
    /// Maps indexes in `data` back to the corresponding `data_index`.
    id: Vec<usize>,
    /// The actual elements of the vector.
    data: Vec<T>,
}
impl<T> SiVec<T> {
    fn new() -> Self {
        Self {
            data_index: vec![],
            id: vec![],
            data: vec![],
        }
    }

    fn get(&self, index: usize) -> Option<&T> {
        if index < self.data.len() {
            Some(&self[index])
        } else {
            None
        }
    }

    fn insert(&mut self, element: T) -> usize {
        if self.data.len() == self.id.len() {
            let previous_capacity = self.data.len();
            self.data_index.push(previous_capacity);
            self.id.push(previous_capacity);
            self.data.push(element);
            previous_capacity
        } else {
            let new_position = self.data.len();
            let reused_id = self.id[new_position];
            self.data_index[reused_id] = new_position;
            self.data.push(element);
            reused_id
        }
    }

    fn remove(&mut self, index: usize) -> Option<T> {
        if self.is_empty() { return None; }
        let physical_pos = self.data_index[index];
        let last_index = self.data.len() - 1;
        self.data.swap(physical_pos, last_index);
        self.id.swap(physical_pos, last_index);
        // update data index
        self.data_index[self.id[physical_pos]] = physical_pos;
        self.data_index[self.id[last_index]] = last_index;
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
                    let id = (0..data.len()).collect_vec();
                    let data_index = id.clone();
                    Self { data_index, id, data }
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
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[self.data_index[index]]
    }
}
impl<T> IndexMut<usize> for SiVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[self.data_index[index]]
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
}