use wasm_bindgen_futures::*;

use super::js_bindings::*;

// TYPES
// ================================================================================================

// type SerializedInputNoteData = (String, Vec<u8>, String, String, String, String, Option<String>);

// type SerializedInputNoteParts = (Vec<u8>, String, String, Option<String>);

// ================================================================================================

// pub(crate) fn serialize_note(
//     note: &InputNoteRecord
// ) -> Result<SerializedInputNoteData, ()> {
//     let note_id = note.note_id().inner().to_string();
//     let note_assets = note.note().assets().to_bytes();
//     let (inclusion_proof, status) = match note.inclusion_proof() {
//         Some(proof) => {
//             // FIXME: This removal is to accomodate a problem with how the node constructs paths where
//             // they are constructed using note ID instead of authentication hash, so for now we remove the first
//             // node here.
//             //
//             // Note: once removed we can also stop creating a new `NoteInclusionProof`
//             //
//             // See: https://github.com/0xPolygonMiden/miden-node/blob/main/store/src/state.rs#L274
//             let mut path = proof.note_path().clone();
//             if path.len() > 0 {
//                 let _removed = path.remove(0);
//             }

//             let block_num = proof.origin().block_num;
//             let node_index = proof.origin().node_index.value();
//             let sub_hash = proof.sub_hash();
//             let note_root = proof.note_root();

//             let inclusion_proof = serde_json::to_string(&NoteInclusionProof::new(
//                 block_num, sub_hash, note_root, node_index, path,
//             )?)
//             .map_err(|err| ())?;

//             (Some(inclusion_proof), String::from("committed"))
//         },
//         None => (None, String::from("pending")),
//     };
//     let recipient = note.note().recipient().to_hex();

//     let sender_id = note.note().metadata().sender();
//     let tag = note.note().metadata().tag();
//     let metadata = serde_json::to_string(&NoteMetadata::new(sender_id, tag))
//         .map_err(|err| ())?;

//     let nullifier = note.note().nullifier().inner().to_string();
//     let script = note.note().script().to_bytes();
//     let inputs = note.note().inputs().to_bytes();
//     let serial_num = note.note().serial_num();
//     let details =
//         serde_json::to_string(&NoteRecordDetails::new(nullifier, script, inputs, serial_num))
//             .map_err(|err| ())?;

//     Ok((note_id, note_assets, recipient, status, metadata, details, inclusion_proof))
// }

// pub(super) async fn insert_input_note_tx(
//     note: &InputNoteRecord
// ) -> Result<(), ()> {
//     let (note_id, assets, recipient, status, metadata, details, inclusion_proof) =
//         serialize_note(note)?;

//     let result = JsFuture::from(idxdb_insert_input_note(
//         note_id,
//         assets,
//         recipient,
//         status,
//         metadata,
//         details,
//         inclusion_proof
//     )).await; 
//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }

// pub async fn insert_output_note_tx(
//     note: &InputNoteRecord
// ) -> Result<(), ()> {
//     let (note_id, assets, recipient, status, metadata, details, inclusion_proof) =
//         serialize_note(note)?;

//     let result = JsFuture::from(idxdb_insert_output_note(
//         note_id,
//         assets,
//         recipient,
//         status,
//         metadata,
//         details,
//         inclusion_proof
//     )).await; 
//     match result {
//         Ok(_) => Ok(()),
//         Err(_) => Err(()),
//     }
// }