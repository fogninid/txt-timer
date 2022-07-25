use std::cmp::Ordering;

#[derive(Debug)]
pub struct Maximals<T>
where
    T: Ord,
{
    count: usize,
    data: Vec<T>,
}

impl<T: Ord> Maximals<T> {
    pub fn new(count: usize) -> Self {
        Maximals {
            count,
            data: Vec::with_capacity(count),
        }
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn insert(&mut self, element: T) {
        let len = self.data.len();
        let pos = self.bisect(0, len, &element);

        if pos == len && len == self.count {
            return;
        }

        if len == self.count {
            self.data.pop();
        }
        self.data.insert(pos, element)
    }

    pub fn data(&self) -> &[T] {
        self.data.as_slice()
    }

    fn bisect(&self, begin: usize, end: usize, element: &T) -> usize {
        if begin == end {
            return begin;
        }

        let first = self.data.get(begin).unwrap();

        if element.gt(first) {
            return begin;
        }

        let last = self.data.get(end - 1).unwrap();
        if element.lt(last) {
            return end;
        }

        let pivot_pos = begin + (end - 1 - begin) / 2;
        if pivot_pos == begin {
            return end - 1;
        }

        let pivot = self.data.get(pivot_pos).unwrap();

        match element.cmp(pivot) {
            Ordering::Less => self.bisect(pivot_pos, end, element),
            Ordering::Equal => pivot_pos,
            Ordering::Greater => self.bisect(begin, pivot_pos, element),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::maximals::Maximals;
    use itertools::Itertools;
    use std::cmp::Ordering;

    #[test]
    fn maximals_int() {
        let mut m: Maximals<u8> = Maximals::new(4);
        assert_eq!(m.data(), vec![]);

        m.insert(1);
        assert_eq!(m.data(), vec![1]);
        m.insert(2);
        m.insert(3);
        m.insert(7);
        assert_eq!(m.data(), vec![7, 3, 2, 1]);
        m.insert(9);
        assert_eq!(m.data(), vec![9, 7, 3, 2]);
        m.insert(3);
        assert_eq!(m.data(), vec![9, 7, 3, 3]);
        m.insert(8);
        assert_eq!(m.data(), vec![9, 8, 7, 3]);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(7);
        assert_eq!(m.data(), vec![9, 8, 8, 8]);

        for v in vec![1, 2, 7, 7, 8, 1, 2].into_iter().permutations(7) {
            m = Maximals::new(5);
            for e in v {
                m.insert(e);
            }
            assert_eq!(m.data(), vec![8, 7, 7, 2, 2]);
        }
    }

    #[derive(Eq, PartialOrd, PartialEq, Debug, Clone, Copy)]
    struct T {
        cmp: u8,
        data: u8,
    }

    impl Ord for T {
        fn cmp(&self, other: &Self) -> Ordering {
            self.cmp.cmp(&other.cmp)
        }
    }

    #[test]
    fn maximals_struct() {
        let mut m: Maximals<T> = Maximals::new(4);
        assert_eq!(m.data(), vec![]);
        let t_1_2 = T { cmp: 1, data: 2 };
        let t_3_1 = T { cmp: 3, data: 1 };
        let t_1_3 = T { cmp: 1, data: 3 };
        m.insert(t_1_2);
        m.insert(t_3_1);
        m.insert(t_1_3);

        assert_eq!(m.data(), vec![t_3_1, t_1_3, t_1_2]);

        m.clear();
        m.insert(t_3_1);
        m.insert(t_1_3);
        m.insert(t_1_2);
        assert_eq!(m.data(), vec![t_3_1, t_1_3, t_1_2]);
    }
}
