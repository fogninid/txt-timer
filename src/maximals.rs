use std::collections::VecDeque;

#[derive(Debug)]
pub struct Maximals<T>
where
    T: Ord,
{
    count: usize,
    data: VecDeque<T>,
}

impl<T: Ord> Maximals<T> {
    pub fn new(count: usize) -> Self {
        Maximals {
            count,
            data: VecDeque::with_capacity(count),
        }
    }

    pub fn insert(&mut self, element: T) -> Option<&mut T> {
        let idx = self.data.partition_point(|x| x > &element);
        if idx < self.count {
            if self.data.len() + 1 > self.count {
                self.data.pop_back();
            }
            self.data.insert(idx, element);
            Some(&mut self.data[idx])
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.data.iter()
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.data.clear();
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
        itertools::assert_equal(m.iter(), vec![].iter());

        m.insert(1);
        itertools::assert_equal(m.iter(), (vec![1]).iter());
        m.insert(2);
        m.insert(3);
        m.insert(7);
        itertools::assert_equal(m.iter(), vec![7, 3, 2, 1].iter());
        m.insert(9);
        itertools::assert_equal(m.iter(), vec![9, 7, 3, 2].iter());
        m.insert(3);
        itertools::assert_equal(m.iter(), vec![9, 7, 3, 3].iter());
        m.insert(8);
        itertools::assert_equal(m.iter(), vec![9, 8, 7, 3].iter());
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(8);
        m.insert(7);
        itertools::assert_equal(m.iter(), vec![9, 8, 8, 8].iter());

        for v in vec![1, 2, 7, 7, 8, 1, 2].into_iter().permutations(7) {
            m = Maximals::new(5);
            for e in v {
                m.insert(e);
            }
            itertools::assert_equal(m.iter(), vec![8, 7, 7, 2, 2].iter());
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
        itertools::assert_equal(m.iter(), vec![].iter());
        let t_1_2 = T { cmp: 1, data: 2 };
        let t_3_1 = T { cmp: 3, data: 1 };
        let t_1_3 = T { cmp: 1, data: 3 };
        m.insert(t_1_2);
        m.insert(t_3_1);
        m.insert(t_1_3);

        itertools::assert_equal(m.iter(), vec![t_3_1, t_1_3, t_1_2].iter());

        m.clear();
        m.insert(t_3_1);
        m.insert(t_1_3);
        m.insert(t_1_2);
        itertools::assert_equal(m.iter(), vec![t_3_1, t_1_3, t_1_2].iter());
    }
}
