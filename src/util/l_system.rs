use std::marker::PhantomData;

use bevy::prelude::*;

//https://en.wikipedia.org/wiki/L-system

pub struct LSystem<Alphabet: Clone, P: Fn(&Alphabet, u32) -> Option<Vec<Alphabet>>> {
    producer: P,
    phantom: PhantomData<Alphabet>
}

impl<Alphabet: Clone, P: Fn(&Alphabet, u32) -> Option<Vec<Alphabet>>> LSystem<Alphabet, P> {
    pub fn apply_to(&self, sentence: &[Alphabet], seed: u32) -> Vec<Alphabet> {
        let _my_span = info_span!("l_structure_apply_to", name = "l_structure_apply_to").entered();
        let mut new_sentence = Vec::new(); 
        for (i, letter) in sentence.iter().enumerate() {
            if let Some(mut rhs) = (self.producer)(letter, seed+i as u32) {
                new_sentence.append(&mut rhs);
            } else {
                new_sentence.push(letter.clone());
            }
        }
        new_sentence
    }

    pub fn iterate(&self, sentence: &Vec<Alphabet>, iterations: u32, seed: u32) -> Vec<Alphabet> {
        let mut curr_sentence = sentence;
        let mut ret_sentence = Vec::new();
        for _ in 0..iterations {
            ret_sentence = self.apply_to(curr_sentence, seed);
            curr_sentence = &ret_sentence
        }
        ret_sentence
    }

    pub fn new(producer: P) -> Self {
        Self {
            producer,
            phantom: PhantomData
        }
    }
}

#[derive(Clone, Copy)]
pub enum TreeAlphabet {
    Move(f32),
    Replace(f32),
    Rotate(Quat),
    StartBranch,
    EndBranch
}