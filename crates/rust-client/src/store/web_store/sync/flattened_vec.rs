use alloc::vec::Vec;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct FlattenedU8Vec {
    data: Vec<u8>,
    lengths: Vec<u32>,
}

#[wasm_bindgen]
impl FlattenedU8Vec {
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    pub fn lengths(&self) -> Vec<u32> {
        self.lengths.clone()
    }

    pub fn num_inner_vecs(&self) -> u32 {
        self.lengths.len() as u32 // The number of inner Vec<u8> is the number of lengths
    }
}

pub fn flatten_nested_u8_vec(nested_vec: Vec<Vec<u8>>) -> FlattenedU8Vec {
    // Calculate the lengths of each inner Vec<u8> before flattening
    let lengths: Vec<u32> = nested_vec.iter().map(|v| v.len() as u32).collect();

    // Now you can flatten the Vec<Vec<u8>> into a single Vec<u8>
    let data: Vec<u8> = nested_vec.into_iter().flatten().collect();

    FlattenedU8Vec { data, lengths }
}
