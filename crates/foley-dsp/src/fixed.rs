//! A list of at most `N`, kept inline, for a bed's voices.

/// A list of at most `N` items, kept inline: pushing past `N` drops the new
/// one, and nothing is ever allocated. The slots past its length hold
/// defaults, so the items are a plain slice with no check per item.
#[derive(Debug)]
pub struct Fixed<T, const N: usize> {
    items: [T; N],
    len: usize,
}

impl<T: Default, const N: usize> Default for Fixed<T, N> {
    fn default() -> Self {
        Fixed::new()
    }
}

impl<T: Default, const N: usize> Fixed<T, N> {
    pub fn new() -> Self {
        Fixed {
            items: core::array::from_fn(|_| T::default()),
            len: 0,
        }
    }
}

impl<T, const N: usize> Fixed<T, N> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == N
    }

    /// Adds `item`, unless it's full; whether it did.
    pub fn push(&mut self, item: T) -> bool {
        if self.len == N {
            return false;
        }
        self.items[self.len] = item;
        self.len += 1;
        true
    }

    /// Keeps only the items `keep` says to, in order.
    pub fn retain(&mut self, mut keep: impl FnMut(&T) -> bool) {
        let mut kept = 0;
        for i in 0..self.len {
            if keep(&self.items[i]) {
                self.items.swap(kept, i);
                kept += 1;
            }
        }
        self.len = kept;
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items[..self.len].iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items[..self.len].iter_mut()
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a mut Fixed<T, N> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items[..self.len].iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_holds_at_most_n_and_keeps_order() {
        let mut list: Fixed<u32, 3> = Fixed::new();
        assert!(list.push(1) && list.push(2) && list.push(3));
        assert!(!list.push(4) && list.is_full());
        list.retain(|x| *x != 2);
        assert_eq!(list.iter().copied().collect::<std::vec::Vec<_>>(), [1, 3]);
        for x in &mut list {
            *x *= 10;
        }
        assert!(list.push(5));
        assert_eq!(
            list.iter().copied().collect::<std::vec::Vec<_>>(),
            [10, 30, 5]
        );
    }
}
