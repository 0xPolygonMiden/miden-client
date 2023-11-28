use super::Client;

pub fn insert_test_data(mut client: Client) {
    use mock::mock::{
        account::MockAccountType, notes::AssetPreservationStatus, transaction::mock_inputs,
    };

    // generate test data
    let (_, _, _, recorded_notes) = mock_inputs(
        MockAccountType::StandardExisting,
        AssetPreservationStatus::Preserved,
    );

    // insert notes into database
    for note in recorded_notes.into_iter() {
        client.insert_input_note(note).unwrap();
    }
}
