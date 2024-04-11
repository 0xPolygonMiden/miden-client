use serde_wasm_bindgen::from_value;
use wasm_bindgen_futures::*;

use super::WebStore;
use crate::native_code::store::NativeNoteFilter;
use crate::web_client::notes::NoteFilter;

mod js_bindings;
use js_bindings::*;

mod models;
use models::*;

mod utils;
use utils::*;

impl WebStore {
    // pub(crate) async fn get_input_notes(
    //     &mut self,
    //     filter: NativeNoteFilter
    // ) -> Result<Vec<InputNoteRecord>, ()> {
    //     let filter_as_str = match filter {
    //         NativeNoteFilter::Pending => "Pending",
    //         NativeNoteFilter::Committed => "Committed",
    //         NativeNoteFilter::Consumed => "Consumed",
    //         NativeNoteFilter::All => "All", 
    //     };

    //     let promise = idxdb_get_input_notes(filter_as_str.to_string());
    //     let js_value = JsFuture::from(promise).await?;
    //     let input_notes_idxdb: Vec<InputNoteIdxdbObject> = from_value(js_value).unwrap();
  
    //     let native_input_notes: Result<Vec<InputNoteRecord>, ()> = input_notes_idxdb.into_iter().map(|note_idxdb| {

    //         let note_details: NoteRecordDetails =
    //             serde_json::from_str(&note_idxdb.details).map_err(|_err| ())?;
    //         let note_metadata: NoteMetadata =
    //             serde_json::from_str(&note_idxdb.metadata).map_err(|_err| ())?;

    //         let script = NoteScript::read_from_bytes(note_details.script())?;
    //         let inputs = NoteInputs::read_from_bytes(note_details.inputs())?;

    //         let serial_num = note_details.serial_num();
    //         let note_metadata = NoteMetadata::new(note_metadata.sender(), note_metadata.tag());
    //         let note_assets = NoteAssets::read_from_bytes(&note_idxdb.assets)?;
    //         let note = Note::from_parts(script, inputs, note_assets, *serial_num, note_metadata);

    //         let inclusion_proof = match note_idxdb.inclusion_proof {
    //             Some(note_inclusion_proof) => {
    //                 let note_inclusion_proof: NoteInclusionProof =
    //                     serde_json::from_str(&note_inclusion_proof)
    //                         .map_err(|err| ())?;

    //                 Some(note_inclusion_proof)
    //             },
    //             _ => None,
    //         };

    //         Ok(InputNoteRecord::new(note, inclusion_proof))
    //     }).collect(); // Collect into a Result<Vec<AccountId>, ()>
        
    //     return native_input_notes;
    // }

    // pub(crate) async fn get_input_note(
    //     &mut self,
    //     note_id: NoteId
    // ) -> Result<InputNoteRecord, ()> {
    //     let note_id_str = &note_id.inner().to_string();

    //     let promise = idxdb_get_input_note(note_id_str);
    //     let js_value = JsFuture::from(promise).await?;
    //     let input_note_idxdb: InputNoteIdxdbObject = from_value(js_value).unwrap();

    //     let note_details: NoteRecordDetails =
    //         serde_json::from_str(&input_note_idxdb.details).map_err(|_err| ())?;
    //     let note_metadata: NoteMetadata =
    //         serde_json::from_str(&input_note_idxdb.metadata).map_err(|_err| ())?;

    //     let script = NoteScript::read_from_bytes(note_details.script())?;
    //     let inputs = NoteInputs::read_from_bytes(note_details.inputs())?;

    //     let serial_num = note_details.serial_num();
    //     let note_metadata = NoteMetadata::new(note_metadata.sender(), note_metadata.tag());
    //     let note_assets = NoteAssets::read_from_bytes(&input_note_idxdb.assets)?;
    //     let note = Note::from_parts(script, inputs, note_assets, *serial_num, note_metadata);

    //     let inclusion_proof = match input_note_idxdb.inclusion_proof {
    //         Some(note_inclusion_proof) => {
    //             let note_inclusion_proof: NoteInclusionProof =
    //                 serde_json::from_str(&note_inclusion_proof)
    //                     .map_err(|err| ())?;

    //             Some(note_inclusion_proof)
    //         },
    //         _ => None,
    //     };

    //     Ok(InputNoteRecord::new(note, inclusion_proof))
    // }

    // pub(crate) async fn insert_input_note(
    //     &mut self,
    //     note: &InputNoteRecord
    // ) -> Result<(), ()> {
    //     insert_input_note_tx(note).await
    // }

    // pub(crate) async fn get_output_notes(
    //     &mut self,
    //     filter: NativeNoteFilter
    // ) -> Result<Vec<InputNoteRecord>, ()> {
    //     let filter_as_str = match filter {
    //         NativeNoteFilter::Pending => "Pending",
    //         NativeNoteFilter::Committed => "Committed",
    //         NativeNoteFilter::Consumed => "Consumed",
    //         NativeNoteFilter::All => "All", 
    //     };

    //     let promise = idxdb_get_output_notes(filter_as_str.to_string());
    //     let js_value = JsFuture::from(promise).await?;
    //     let output_notes_idxdb: Vec<OutputNoteIdxdbObject> = from_value(js_value).unwrap();
  
    //     let native_output_notes: Result<Vec<InputNoteRecord>, ()> = output_notes_idxdb.into_iter().map(|note_idxdb| {

    //         let note_details: NoteRecordDetails =
    //             serde_json::from_str(&note_idxdb.details).map_err(|_err| ())?;
    //         let note_metadata: NoteMetadata =
    //             serde_json::from_str(&note_idxdb.metadata).map_err(|_err| ())?;

    //         let script = NoteScript::read_from_bytes(note_details.script())?;
    //         let inputs = NoteInputs::read_from_bytes(note_details.inputs())?;

    //         let serial_num = note_details.serial_num();
    //         let note_metadata = NoteMetadata::new(note_metadata.sender(), note_metadata.tag());
    //         let note_assets = NoteAssets::read_from_bytes(&note_idxdb.assets)?;
    //         let note = Note::from_parts(script, inputs, note_assets, *serial_num, note_metadata);

    //         let inclusion_proof = match note_idxdb.inclusion_proof {
    //             Some(note_inclusion_proof) => {
    //                 let note_inclusion_proof: NoteInclusionProof =
    //                     serde_json::from_str(&note_inclusion_proof)
    //                         .map_err(|err| ())?;

    //                 Some(note_inclusion_proof)
    //             },
    //             _ => None,
    //         };

    //         Ok(InputNoteRecord::new(note, inclusion_proof))
    //     }).collect(); // Collect into a Result<Vec<AccountId>, ()>
        
    //     return native_output_notes;
    // }

    // pub(crate) async fn get_unspent_input_note_nullifiers(
    //     &mut self
    // ) -> Result<Vec<Nullifier>, ()>{
    //     let promise = idxdb_get_unspent_input_note_nullifiers();
    //     let js_value = JsFuture::from(promise).await?;
    //     let nullifiers_as_str: Vec<String> = from_value(js_value).unwrap();

    //     let nullifiers = nullifiers_as_str.into_iter().map(|s| {
    //         Digest::try_from(v).map(Nullifier::from).map_err(|err| ())
    //     }).collect::<Result<Vec<Nullifier>, _>>();

    //     return nullifiers
    // }
}