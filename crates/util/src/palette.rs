use std::{collections::VecDeque, ops::Index};

//Assuming that most values in the palette will be small, so V gets cloned instead of referenced in the iterator
pub trait Palette<K, V, I>: Index<usize, Output = V>
where
    I: Iterator<Item = (K, V)>,
    K: Clone,
    V: Clone,
{
    fn index_key(&self, index: usize) -> K;
    fn get_key(&self, value: &V) -> Option<K>;
    fn get_value(&self, key: K) -> Option<&V>;
    fn set(&mut self, index: usize, val: V);
    fn palette_iter(&self) -> I;
}

pub struct PaletteIter<K, V> {
    pub data: VecDeque<(K, V)>,
}

impl<K, V> Iterator for PaletteIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.data.pop_front()
    }
}

pub trait PaletteMap<K, V, R> {
    fn get_value(&self, key: K) -> Option<&V>;
    fn get_entry_mut(&mut self, key: K) -> Option<&mut (K, V, R)>;
    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (K, V, R)>;
    fn get_key(&self, value: &V) -> Option<K>;
}

impl<K: Copy + PartialEq<K>, V: PartialEq<V>> PaletteMap<K, V, u16> for Vec<(K, V, u16)> {
    fn get_value(&self, key: K) -> Option<&V> {
        self.iter()
            .find(|(k, _, r)| *k == key && *r > 0)
            .map(|(_, v, _)| v)
    }

    fn get_key(&self, value: &V) -> Option<K> {
        self.iter()
            .find(|(_, v, r)| v == value && *r > 0)
            .map(|(k, _, _)| *k)
    }

    fn get_entry_mut(&mut self, key: K) -> Option<&mut (K, V, u16)> {
        self.iter_mut().find(|(k, _, r)| *k == key && *r > 0)
    }

    fn get_entry_mut_value(&mut self, val: &V) -> Option<&mut (K, V, u16)> {
        self.iter_mut().find(|(_, v, r)| val == v && *r > 0)
    }
}
