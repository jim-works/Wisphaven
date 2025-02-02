use super::components::*;
use bevy::{prelude::*, utils::HashMap};

#[derive(Resource, Default)]
pub struct SavedToLoadedIdMap<T: Into<Id> + Clone + From<Id> + std::hash::Hash + Eq + PartialEq> {
    pub map: HashMap<T, T>,
    pub max_key_id: u32,
}

impl<T: Into<Id> + From<Id> + Clone + std::hash::Hash + Eq + PartialEq> SavedToLoadedIdMap<T> {
    pub fn insert(&mut self, key: T, val: T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => {}
            Id::Basic(id) | Id::Dynamic(id) => self.max_key_id = self.max_key_id.max(id),
        }
        self.map.insert(key, val)
    }
    pub fn get(&self, key: &T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => Some(T::from(Id::Empty)),
            _ => self.map.get(key).cloned(),
        }
    }
}

#[derive(Resource, Default)]
pub struct LoadedToSavedIdMap<T: Into<Id> + Clone + From<Id> + std::hash::Hash + Eq + PartialEq> {
    pub map: HashMap<T, T>,
    pub max_key_id: u32,
}

impl<T: Into<Id> + From<Id> + std::hash::Hash + Clone + Eq + PartialEq> LoadedToSavedIdMap<T> {
    pub fn insert(&mut self, key: T, val: T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => {}
            Id::Basic(id) | Id::Dynamic(id) => self.max_key_id = self.max_key_id.max(id),
        }
        self.map.insert(key, val)
    }
    pub fn get(&self, key: &T) -> Option<T> {
        match key.clone().into() {
            Id::Empty => Some(T::from(Id::Empty)),
            _ => self.map.get(key).cloned(),
        }
    }
}
