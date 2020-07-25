use std::collections::HashSet;

pub trait Unique {
    fn unique(self) -> bool;
}

impl<T: IntoIterator<Item = V>, V: std::cmp::Eq + std::hash::Hash> Unique for T {
    fn unique(self) -> bool {
        let mut seen: HashSet<_> = HashSet::new();
        self.into_iter().all(move |i| seen.insert(i))
    }
}
