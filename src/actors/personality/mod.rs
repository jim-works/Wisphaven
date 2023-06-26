use std::ops::{Deref, DerefMut};

use bevy::prelude::*;
use rand_distr::Normal;

pub struct PersonalityPlugin;

mod components;
pub use components::*;

impl Plugin for PersonalityPlugin {
    fn build(&self, _app: &mut App) {
        
    }
}

//treated as a normal distribution with mean value and variance
#[derive(Debug, Clone, Copy)]
pub struct FacetValue(Normal<f32>);

impl Deref for FacetValue {
    type Target = Normal<f32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FacetValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FacetValue {
    pub fn new(value: f32, std_dev: f32) -> Result<Self, rand_distr::NormalError> {
        let dist = Normal::new(value,std_dev)?;
        Ok(Self(dist))
    }
}
