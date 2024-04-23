import { 
    inputNotes,
    outputNotes,
    notesScripts,
} from './schema.js';

export async function getOutputNotes(
    status
) {
    try {
        let notes;

        // Fetch the records based on the filter
        if (status === 'All') {
            notes = await outputNotes.toArray();
        } else {
            notes = await outputNotes.where('status').equals(status.toLowerCase()).toArray();
        }

        // Fetch all scripts from the scripts table for joining
        const scripts = await notesScripts.toArray();
        const scriptMap = new Map(scripts.map(script => [script.script_hash, script.serialized_note_script]));

        // Process each note to convert 'blobField' from Blob to Uint8Array
        const processedNotes = await Promise.all(notes.map(async note => {
            const assetsArrayBuffer = await note.assets.arrayBuffer();
            const assetsArray = new Uint8Array(assetsArrayBuffer);
            const assetsBase64 = uint8ArrayToBase64(assetsArray);
            note.assets = assetsBase64;

            // Parse details JSON and perform a "join"
            if (note.details) {
                const details = JSON.parse(note.details);
                if (details.script_hash) {
                    note.serialized_note_script = scriptMap.get(details.script_hash) || null;
                }
            }

            return note;
        }));

        return processedNotes;
    } catch {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

export async function getInputNotes(
    status
) {
    try {
        let notes;

        // Fetch the records based on the filter
        if (status === 'All') {
            notes = await inputNotes.toArray();
        } else {
            notes = await inputNotes.where('status').equals(status.toLowerCase()).toArray();
        }

        // Fetch all scripts from the scripts table for joining
        const scripts = await notesScripts.toArray();
        const scriptMap = new Map(scripts.map(script => [script.script_hash, script.serialized_note_script]));

        // Process each note to convert 'blobField' from Blob to Uint8Array
        const processedNotes = await Promise.all(notes.map(async note => {
            const assetsArrayBuffer = await note.assets.arrayBuffer();
            const assetsArray = new Uint8Array(assetsArrayBuffer);
            const assetsBase64 = uint8ArrayToBase64(assetsArray);
            note.assets = assetsBase64;

            // Parse details JSON and perform a "join"
            if (note.details) {
                const details = JSON.parse(note.details);
                if (details.script_hash) {
                    note.serialized_note_script = scriptMap.get(details.script_hash) || null;
                }
            }

            return note;
        }));

        return processedNotes;
    } catch {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

export async function getInputNote(
    noteId
) {
    try {
        const note = await inputNotes.get(noteId);

        const assetsArrayBuffer = await note.assets.arrayBuffer();
        const assetsArray = new Uint8Array(assetsArrayBuffer);
        const assetsBase64 = uint8ArrayToBase64(assetsArray);

        if (note.details) {
            const details = JSON.parse(note.details);
            if (details.script_hash) {
                const script = await notesScripts.get(details.script_hash);
                note.serialized_note_script = script ? script.serialized_note_script : null;
            }
        }

        note.assets = assetsBase64

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
    noteScriptHash,
    serializedNoteScript,
    inclusionProof
) {
    return db.transaction('rw', inputNotes, notesScripts, async (tx) => {
        try {
            let assetsBlob = new Blob([new Uint8Array(assets)]);

            // Prepare the data object to insert
            const data = {
                noteId: noteId,
                assets: assetsBlob,
                recipient: recipient,
                status: status,
                metadata: metadata ? JSON.stringify(metadata) : null,
                details: JSON.stringify(details),
                inclusionProof: inclusionProof ? JSON.stringify(inclusionProof) : null,
            };

            // Perform the insert using Dexie
            await tx.inputNotes.add(data);

            const exists = await tx.notesScripts.get(noteScriptHash);
            if (!exists) {
                let serializedNoteScriptBlob = new Blob([new Uint8Array(serializedNoteScript)]);

                const data = {
                    scriptHash: noteScriptHash,
                    serializedNoteScript: serializedNoteScriptBlob,
                };
                await tx.notesScripts.add(data);
            }
        } catch {
            console.error(`Error inserting note: ${noteId}:`, error);
            throw error; // Rethrow the error to handle it further up the call chain if needed
        }
    });
}

export async function insertOutputNote(
    noteId,
    assets,
    recipient,
    status,
    metadata,
    details,
    noteScriptHash,
    serializedNoteScript,
    inclusionProof
) {
    return db.transaction('rw', outputNotes, notesScripts, async (tx) => {
        try {
            let assetsBlob = new Blob([new Uint8Array(assets)]);

            // Prepare the data object to insert
            const data = {
                noteId: noteId,
                assets: assetsBlob,
                recipient: recipient,
                status: status,
                metadata: JSON.stringify(metadata),
                details: details ? JSON.stringify(details) : null,
                inclusionProof: inclusionProof ? JSON.stringify(inclusionProof) : null,
            };

            // Perform the insert using Dexie
            await tx.outputNotes.add(data);

            const exists = await tx.notesScripts.get(noteScriptHash);
            if (!exists) {
                let serializedNoteScriptBlob = null;
                if (serializedNoteScript) {
                    serializedNoteScriptBlob = new Blob([new Uint8Array(serializedNoteScript)]);
                }

                const data = {
                    scriptHash: noteScriptHash,
                    serializedNoteScript: serializedNoteScriptBlob,
                };
                await tx.notesScripts.add(data);
            }
        } catch {
            console.error(`Error inserting note: ${noteId}:`, error);
            throw error; // Rethrow the error to handle it further up the call chain if needed
        }
    });
}

export async function getUnspentInputNoteNullifiers() {
    try {
        const notes = await db.InputNotes.where('status').equals('Committed').toArray();
        const nullifiers = notes.map(note => JSON.parse(note.details).nullifier);

        return nullifiers;
    } catch (err) {
        console.error("Failed to get unspent input note nullifiers: ", err);
        throw err;
    }
}

function uint8ArrayToBase64(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}