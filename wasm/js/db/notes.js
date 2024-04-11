import { 
    inputNotes,
    outputNotes
} from './schema.js';

export async function getInputNotes() {
    try {
        let notes;

        // Fetch the records based on the filter
        if (filter === 'All') {
            notes = await inputNotes.toArray();
        } else {
            notes = await inputNotes.where('status').equals(filter.toLowerCase()).toArray();
        }

        // Process each note to convert 'blobField' from Blob to Uint8Array
        const processedNotes = await Promise.all(notes.map(async note => {
            const assetsArrayBuffer = await note.assets.arrayBuffer();
            note.assets = new Uint8Array(assetsArrayBuffer);

            return note;
        }));

        return processedNotes;
    } catch {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

export async function getInputNote() {
    try {
        const note = await inputNotes.get(noteId);
        const assetsArrayBuffer = await note.assets.arrayBuffer();
        note.assets = new Uint8Array(assetsArrayBuffer);

        return note
    } catch {
        console.error("Failed to get input note: ", err);
        throw err;
    }
    
}

export async function insertInputNote(
    noteId,
    assets,
    recipient,
    status,
    metadata,
    details,
    inclusion_proof
) {
    try {
        let assetsBlob = new Blob([assets]);

        // Prepare the data object to insert
        const data = {
            noteId: noteId,
            assets: assetsBlob,
            recipient: recipient,
            status: status,
            metadata: JSON.stringify(metadata),
            details: JSON.stringify(details),
            inclusion_proof: JSON.stringify(inclusion_proof),
        };

        // Perform the insert using Dexie
        await inputNotes.add(data);
        return `Successfully inserted note: ${noteId}`;
    } catch (error) {
        console.error(`Error inserting note: ${noteId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function insertOutputNote(
    noteId,
    assets,
    recipient,
    status,
    metadata,
    details,
    inclusion_proof
) {
    try {
        let assetsBlob = new Blob([assets]);

        // Prepare the data object to insert
        const data = {
            noteId: noteId,
            assets: assetsBlob,
            recipient: recipient,
            status: status,
            metadata: JSON.stringify(metadata),
            details: JSON.stringify(details),
            inclusion_proof: JSON.stringify(inclusion_proof),
        };

        // Perform the insert using Dexie
        await outputNotes.add(data);
        return `Successfully inserted note: ${noteId}`;
    } catch (error) {
        console.error(`Error inserting note: ${noteId}:`, error);
        throw error; // Rethrow the error to handle it further up the call chain if needed
    }
}

export async function getUnspentInputNoteNullifiers() {
    try {
        const notes = await db.InputNotes.where('status').equals('committed').toArray();
        const nullifiers = notes.map(note => JSON.parse(note.details).nullifier);

        return nullifiers;
    } catch (err) {
        console.error("Failed to get unspent input note nullifiers: ", err);
        throw err;
    }
}