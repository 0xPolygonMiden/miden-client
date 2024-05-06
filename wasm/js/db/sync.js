import {
    db,
    stateSync,
    inputNotes,
    outputNotes,
    transactions,
    blockHeaders,
} from './schema.js';

export async function getNoteTags() {
    try {
        const record = await stateSync.get(1);  // Since id is the primary key and always 1
        if (record) {
            console.log('Retrieved record:', record);
            return record.tags;
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
            let data = {
                block_num: record.blockNum
            };
            return data;
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
    blockHeader,
    chainMmrPeaks,
    hasClientNotes,
    nodeIndices,
    nodes,
    noteIds,
    inclusionProofs,
    transactionIds,
) {
    console.log('blockNum: ', blockNum)
    console.log('blockHeader: ', blockHeader)
    console.log('chainMmrPeaks: ', chainMmrPeaks)
    console.log('nullifiers: ', nullifiers)
    console.log('noteIds: ', noteIds)
    console.log('inclusionProofs: ', inclusionProofs)
    console.log('transactionIds: ', transactionIds)
    console.log('nodeIndices: ', nodeIndices)
    console.log('nodes: ', nodes)
    console.log('hasClientNotes: ', hasClientNotes)
    return db.transaction('rw', stateSync, inputNotes, outputNotes, transactions, blockHeaders, async (tx) => {
        await updateSyncHeight(tx, blockNum);
        await updateSpentNotes(tx, nullifiers);
        await updateBlockHeader(tx, blockNum, blockHeader, chainMmrPeaks, hasClientNotes);
        await updateChainMmrNodes(tx, nodeIndices, nodes);
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
        console.log("Sync height updated successfully.");
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
                await tx.inputNotes.update(note.noteId, { status: 'Consumed' });
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
                await tx.outputNotes.update(note.noteId, { status: 'Consumed' });
            }
        }

        console.log("Spent notes have been updated successfully.");
    } catch (error) {
        console.error("Error updating input notes:", error);
        throw error;
    }
}

async function updateBlockHeader(
    tx,
    blockNum, 
    blockHeader,
    chainMmrPeaks,
    hasClientNotes
) {
    try {
        const data = {
            blockNum: blockNum,
            header: blockHeader,
            chainMmrPeaks: chainMmrPeaks,
            hasClientNotes: hasClientNotes
        };

        await tx.blockHeaders.add(data);
        console.log(`Block header for block ${blockNum} inserted successfully.`);
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw error;
    }
}

async function updateChainMmrNodes(
    tx,
    nodeIndices,
    nodes
) {
    try {
        // Check if the arrays are not of the same length
        if (nodeIndices.length !== nodes.length) {
            throw new Error("nodeIndices and nodes arrays must be of the same length");
        }

        if (nodeIndices.length === 0) {
            console.log("No chain MMR nodes to update");
            return;
        }

        // Create the updates array with objects matching the structure expected by your IndexedDB schema
        const updates = nodeIndices.map((index, i) => ({
            index: index,  // Assuming 'index' is the primary key or part of it
            node: nodes[i] // Other attributes of the object
        }));

        // Perform bulk update or insertion; assumes tx.chainMmrNodes is a valid table reference in a transaction
        await tx.chainMmrNodes.bulkAdd(updates);
        console.log("Successfully updated chain MMR nodes");
    } catch (err) {
        console.error("Failed to update chain mmr nodes: ", err);
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

        if (noteIds.length === 0) {
            console.log("No notes to update");
            return;
        }

        for (let i = 0; i < noteIds.length; i++) {
            const noteId = noteIds[i];
            const inclusionProof = inclusionProofs[i];

            // Update input notes
            await tx.inputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
                inclusion_proof: inclusionProof
            });

            // Update output notes
            await tx.outputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
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
        if (transactionIds.length === 0) {
            console.log("No transactions to update");
            return;
        }

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