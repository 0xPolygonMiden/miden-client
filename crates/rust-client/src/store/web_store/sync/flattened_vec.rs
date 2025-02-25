use alloc::vec::Vec;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct FlattenedU8Vec {
    data: Vec<u8>,
    lengths: Vec<usize>,
}

#[wasm_bindgen]
impl FlattenedU8Vec {
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn lengths(&self) -> Vec<usize> {
        self.lengths.clone()
    }

    pub fn num_inner_vecs(&self) -> usize {
        self.lengths.len() // The number of inner Vec<u8> is the number of lengths
    }
}

pub fn flatten_nested_u8_vec(nested_vec: Vec<Vec<u8>>) -> FlattenedU8Vec {
    // Calculate the lengths of each inner Vec<u8> before flattening
    let lengths: Vec<usize> = nested_vec.iter().map(Vec::len).collect();

    // Now you can flatten the Vec<Vec<u8>> into a single Vec<u8>
    let data: Vec<u8> = nested_vec.into_iter().flatten().collect();

    FlattenedU8Vec { data, lengths }
}
