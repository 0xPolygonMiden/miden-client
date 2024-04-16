import {
    db,
    stateSync,
    inputNotes,
    outputNotes,
    transactions
} from './schema.js';

export async function getNoteTags() {
    try {
        const record = await stateSync.get(1);  // Since id is the primary key and always 1
        if (record) {
            console.log('Retrieved record:', record);
            return record.tags;  // Accessing blockNum directly from the record
        } else {
            console.log('No record found with id: 1');
            return null;
        }
    } catch (error) {
        console.error('Error fetching record:', error);
        return null;
    }
}

export async function getSyncHeight() {
    try {
        const record = await stateSync.get(1);  // Since id is the primary key and always 1
        if (record) {
            console.log('Retrieved record:', record);
            return record.blockNum;  // Accessing blockNum directly from the record
        } else {
            console.log('No record found with id: 1');
            return null;
        }
    } catch (error) {
        console.error('Error fetching record:', error);
        return null;
    }
}

export async function addNoteTag(
    tags
) {
    try {
        await stateSync.update(1, { tags: tags });
    } catch {
        console.error("Failed to add note tag: ", err);
        throw err;
    }
}

export async function applyStateSync(
    blockNum,
    nullifiers,
    noteIds,
    inclusionProofs,
    transactionIds
) {
    return db.transaction('rw', stateSync, inputNotes, outputNotes, transactions, async (tx) => {
        await updateSyncHeight(tx, blockNum);
        await updateSpentNotes(tx, nullifiers);
        await updateCommittedNotes(tx, noteIds, inclusionProofs);
        await updateCommittedTransactions(tx, blockNum, transactionIds);
    });
}

async function updateSyncHeight(
    tx, 
    blockNum
) {
    try {
        await tx.stateSync.update(1, { blockNum: blockNum });
    } catch (error) {
        console.error("Failed to update sync height: ", error);
        throw error;
    }
}

async function updateSpentNotes(
    tx,
    nullifiers
) {
    try {
        // Fetch all notes
        const inputNotes = await tx.inputNotes.toArray();
        const outputNotes = await tx.outputNotes.toArray();

        // Pre-parse all details and store them with their respective note ids for quick access
        const parsedInputNotes = inputNotes.map(note => ({
            noteId: note.noteId,
            details: JSON.parse(note.details)  // Parse the JSON string into an object
        }));

        // Iterate through each parsed note and check against the list of nullifiers
        for (const note of parsedInputNotes) {
            if (nullifiers.includes(note.details.nullifier)) {
                // If the nullifier is in the list, update the note's status
                await tx.inputNotes.update(note.noteId, { status: 'consumed' });
            }
        }

         // Pre-parse all details and store them with their respective note ids for quick access
         const parsedOutputNotes = outputNotes.map(note => ({
            noteId: note.noteId,
            details: JSON.parse(note.details)  // Parse the JSON string into an object
        }));

        // Iterate through each parsed note and check against the list of nullifiers
        for (const note of parsedOutputNotes) {
            if (nullifiers.includes(note.details.nullifier)) {
                // If the nullifier is in the list, update the note's status
                await tx.outputNotes.update(note.noteId, { status: 'consumed' });
            }
        }

        console.log("Spent notes have been updated successfully.");
    } catch (error) {
        console.error("Error updating input notes:", error);
        throw error;
    }
}

async function updateCommittedNotes(
    tx, 
    noteIds, 
    inclusionProofs
) {
    try {
        if (noteIds.length !== inclusionProofs.length) {
            throw new Error("Arrays noteIds and inclusionProofs must be of the same length");
        }

        for (let i = 0; i < noteIds.length; i++) {
            const noteId = noteIds[i];
            const inclusionProof = inclusionProofs[i];

            // Update input notes
            await tx.inputNotes.where({ noteId: noteId }).modify({
                status: 'committed',
                inclusion_proof: inclusionProof
            });

            // Update output notes
            await tx.outputNotes.where({ noteId: noteId }).modify({
                status: 'committed',
                inclusion_proof: inclusionProof
            });
        }
    } catch (error) {
        console.error("Error updating committed notes:", error);
        throw error;
    }
}

async function updateCommittedTransactions(
    tx, 
    blockNum, 
    transactionIds
) {
    try {
        const updates = transactionIds.map(transactionId => ({
            id: transactionId,
            commitHeight: blockNum
        }));

        await tx.transactions.bulkPut(updates);
    } catch (err) {
        console.error("Failed to mark transactions as committed: ", err);
        throw err;
    }
}