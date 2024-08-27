import { 
    db,
    inputNotes,
    outputNotes,
    notesScripts,
    transactions
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
            notes = await outputNotes.where('status').equals(status).toArray();
        }

        return await processOutputNotes(notes);
    } catch (err) {
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
            notes = await inputNotes
                .where('status')
                .equals(status)
                .and(note => note.ignored === "false")
                .toArray();
        }

        return await processInputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

export async function getIgnoredInputNotes() {
    try {
        const notes = await inputNotes
            .where('ignored')
            .equals("true")
            .toArray();

        return await processInputNotes(notes);
    } catch (err) {
        console.error("Failed to get ignored input notes: ", err);
        throw err;
    }
}

export async function getIgnoredOutputNotes() {
    try {
        const notes = await outputNotes
            .where('ignored')
            .equals("true")
            .toArray();

        return await processOutputNotes(notes);
    } catch (err) {
        console.error("Failed to get ignored output notes: ", err);
        throw err;
    }

}

export async function getInputNotesFromIds(
    noteIds
) {
    try {
        let notes;

        // Fetch the records based on a list of IDs
        notes = await inputNotes.where('noteId').anyOf(noteIds).toArray();

        return await processInputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

export async function getOutputNotesFromIds(
    noteIds
) {
    try {
        let notes;

        // Fetch the records based on a list of IDs
        notes = await outputNotes.where('noteId').anyOf(noteIds).toArray();

        return await processOutputNotes(notes);
    } catch (err) {
        console.error("Failed to get input notes: ", err);
        throw err;
    }
}

export async function getUnspentInputNoteNullifiers() {
    try {
        const notes = await inputNotes
            .where('status')
            .anyOf(['Committed', 'Processing'])
            .toArray();
        const nullifiers = notes.map(note => JSON.parse(note.details).nullifier);

        return nullifiers;
    } catch (err) {
        console.error("Failed to get unspent input note nullifiers: ", err);
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
    inclusionProof,
    serializedCreatedAt,
    ignored,
    importedTag
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
                metadata: metadata ? metadata : null,
                details: details,
                inclusionProof: inclusionProof ? inclusionProof : null,
                consumerTransactionId: null,
                createdAt: serializedCreatedAt,
                ignored: ignored.toString(),
                importedTag: importedTag ? importedTag : null
            };

            // Perform the insert using Dexie
            await tx.inputNotes.add(data);

            let serializedNoteScriptBlob = new Blob([new Uint8Array(serializedNoteScript)]);

            const noteScriptData = {
                scriptHash: noteScriptHash,
                serializedNoteScript: serializedNoteScriptBlob,
            };

            await tx.notesScripts.put(noteScriptData);
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
    inclusionProof,
    serializedCreatedAt,
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
                metadata: metadata,
                details: details ? details : null,
                inclusionProof: inclusionProof ? inclusionProof : null,
                consumerTransactionId: null,
                createdAt: serializedCreatedAt,
                ignored: "false",
                imported_tag: null
            };

            // Perform the insert using Dexie
            await tx.outputNotes.add(data);

            if (noteScriptHash) {
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
            }
        } catch {
            console.error(`Error inserting note: ${noteId}:`, error);
            throw error; // Rethrow the error to handle it further up the call chain if needed
        }
    });
}

export async function updateNoteConsumerTxId(noteId, consumerTxId, submittedAt) {
    try {
        // Start a transaction that covers both tables
        await db.transaction('rw', inputNotes, outputNotes, async (tx) => {
            // Update input_notes where note_id matches
            const updatedInputNotes = await tx.inputNotes
                .where('noteId')
                .equals(noteId)
                .modify({ consumerTransactionId: consumerTxId, submittedAt: submittedAt, status: "Processing" });

            // Update output_notes where note_id matches
            const updatedOutputNotes = await tx.outputNotes
                .where('noteId')
                .equals(noteId)
                .modify({ consumerTransactionId: consumerTxId, submittedAt: submittedAt, status: "Processing" });

            // Log the count of updated entries in both tables (optional)
            console.log(`Updated ${updatedInputNotes} input notes and ${updatedOutputNotes} output notes`);
        });
    } catch (err) {
        console.error("Failed to update note consumer transaction ID: ", err);
        throw err;
    }
}

export async function updateNoteInclusionProof(
    noteId, 
    inclusionProof
) {
    try {
        await inputNotes
            .where('noteId')
            .equals(noteId)
            .modify({ inclusionProof: inclusionProof, status: "Committed" });

    } catch (err) {
        console.error("Failed to update inclusion proof: ", err);
        throw err;
    }
}

export async function updateNoteMetadata(
    noteId, 
    metadata
) {
    try {
        await inputNotes
            .where('noteId')
            .equals(noteId)
            .modify({ metadata: metadata });

    } catch (err) {
        console.error("Failed to update inclusion proof: ", err);
        throw err;
    }
}

async function processInputNotes(
    notes
) {
    // Fetch all scripts from the scripts table for joining
    const scripts = await notesScripts.toArray();
    const scriptMap = new Map(scripts.map(script => [script.scriptHash, script.serializedNoteScript]));

    const transactionRecords = await transactions.toArray();
    const transactionMap = new Map(transactionRecords.map(transaction => [transaction.id, transaction.accountId]));

    const processedNotes = await Promise.all(notes.map(async note => {
        // Convert the assets blob to base64
        const assetsArrayBuffer = await note.assets.arrayBuffer();
        const assetsArray = new Uint8Array(assetsArrayBuffer);
        const assetsBase64 = uint8ArrayToBase64(assetsArray);
        note.assets = assetsBase64;

        // Convert the serialized note script blob to base64
        let serializedNoteScriptBase64 = null;
        // Parse details JSON and perform a "join"
        if (note.details) {
            const details = JSON.parse(note.details);
            if (details.script_hash) {
                let serializedNoteScript = scriptMap.get(details.script_hash);
                let serializedNoteScriptArrayBuffer = await serializedNoteScript.arrayBuffer();
                const serializedNoteScriptArray = new Uint8Array(serializedNoteScriptArrayBuffer);
                serializedNoteScriptBase64 = uint8ArrayToBase64(serializedNoteScriptArray);
            }
        }

        // Perform a "join" with the transactions table
        let consumerAccountId = null;
        if (transactionMap.has(note.consumerTransactionId)) { 
            consumerAccountId = transactionMap.get(note.consumerTransactionId);
        }

        return {
            assets: note.assets,
            details: note.details,
            recipient: note.recipient,
            status: note.status,
            metadata: note.metadata ? note.metadata : null,
            inclusion_proof: note.inclusionProof ? note.inclusionProof : null,
            serialized_note_script: serializedNoteScriptBase64,
            consumer_account_id: consumerAccountId,
            created_at: note.createdAt,
            submitted_at: note.submittedAt ? note.submittedAt : null,
            nullifier_height: note.nullifierHeight ? note.nullifierHeight : null,
            ignored: note.ignored === "true",
            imported_tag: note.importedTag ? note.importedTag : null
        };
    }));

    return processedNotes;
}

async function processOutputNotes(
    notes
) {
    // Fetch all scripts from the scripts table for joining
    const scripts = await notesScripts.toArray();
    const scriptMap = new Map(scripts.map(script => [script.scriptHash, script.serializedNoteScript]));

    const transactionRecords = await transactions.toArray();
    const transactionMap = new Map(transactionRecords.map(transaction => [transaction.id, transaction.accountId]));

    // Process each note to convert 'blobField' from Blob to Uint8Array
    const processedNotes = await Promise.all(notes.map(async note => {
        const assetsArrayBuffer = await note.assets.arrayBuffer();
        const assetsArray = new Uint8Array(assetsArrayBuffer);
        const assetsBase64 = uint8ArrayToBase64(assetsArray);
        note.assets = assetsBase64;

        let serializedNoteScriptBase64 = null;
        // Parse details JSON and perform a "join"
        if (note.details) {
            const details = JSON.parse(note.details);
            if (details.script_hash) {
                let serializedNoteScript = scriptMap.get(details.script_hash);
                let serializedNoteScriptArrayBuffer = await serializedNoteScript.arrayBuffer();
                const serializedNoteScriptArray = new Uint8Array(serializedNoteScriptArrayBuffer);
                serializedNoteScriptBase64 = uint8ArrayToBase64(serializedNoteScriptArray);
            }
        }

        // Perform a "join" with the transactions table
        let consumerAccountId = null;
        if (transactionMap.has(note.consumerTransactionId)) { 
            consumerAccountId = transactionMap.get(note.consumerTransactionId);
        }

        return {
            assets: note.assets,
            details: note.details ? note.details : null,
            recipient: note.recipient,
            status: note.status,
            metadata: note.metadata,
            inclusion_proof: note.inclusionProof ? note.inclusionProof : null,
            serialized_note_script: serializedNoteScriptBase64,
            consumer_account_id: consumerAccountId,
            created_at: note.createdAt,
            submitted_at: note.submittedAt ? note.submittedAt : null,
            nullifier_height: note.nullifierHeight ? note.nullifierHeight : null,
            ignored: note.ignored === "true",
            imported_tag: note.importedTag ? note.importedTag : null
        };
    }));
    return processedNotes;
}

function uint8ArrayToBase64(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}
