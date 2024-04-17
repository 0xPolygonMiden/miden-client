// type SerializedBlockHeaderData = (String, String, String, bool);

// pub fn serialize_block_header(
//     block_header: BlockHeader,
//     chain_mmr_peaks: Vec<Digest>,
//     has_client_notes: bool,
// ) -> Result<SerializedBlockHeaderData, ()> {
//     let block_num = block_header.block_num().to_string();
//     let header =
//         serde_json::to_string(&block_header).map_err(|err| ())?;
//     let chain_mmr_peaks =
//         serde_json::to_string(&chain_mmr_peaks).map_err(|err| ())?;

//     Ok((block_num, header, chain_mmr_peaks, has_client_notes))
// }