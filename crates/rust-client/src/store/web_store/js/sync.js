import {
    db,
    stateSync,
    inputNotes,
    outputNotes,
    transactions,
    blockHeaders,
    chainMmrNodes,
} from './schema.js';

export async function getNoteTags() {
    try {
        const record = await stateSync.get(1);  // Since id is the primary key and always 1
        if (record) {
            const tagsArrayBuffer = await record.tags.arrayBuffer();
            const tagsArray = new Uint8Array(tagsArrayBuffer);
            const tagsBase64 = uint8ArrayToBase64(tagsArray);

            return {
                tags: tagsBase64
            };
        } else {
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
        const tagsBlob = new Blob([new Uint8Array(tags)]);
        console.log({tagsBlob})
        await stateSync.update(1, { tags: tagsBlob });
    } catch {
        console.error("Failed to add note tag: ", err);
        throw err;
    }
}

export async function applyStateSync(
    blockNum,
    nullifiers,
    nullifierBlockNums,
    blockHeader,
    chainMmrPeaks,
    hasClientNotes,
    nodeIndexes,
    nodes,
    outputNoteIds,
    outputNoteInclusionProofs,
    inputNoteIds,
    inputNoteInluclusionProofs,
    inputeNoteMetadatas,
    transactionIds,
    transactionBlockNums
) {
    return db.transaction('rw', stateSync, inputNotes, outputNotes, transactions, blockHeaders, chainMmrNodes, async (tx) => {
        await updateSyncHeight(tx, blockNum);
        await updateSpentNotes(tx, nullifierBlockNums, nullifiers);
        await updateBlockHeader(tx, blockNum, blockHeader, chainMmrPeaks, hasClientNotes);
        await updateChainMmrNodes(tx, nodeIndexes, nodes);
        await updateCommittedNotes(tx, outputNoteIds, outputNoteInclusionProofs, inputNoteIds, inputNoteInluclusionProofs, inputeNoteMetadatas);
        await updateCommittedTransactions(tx, transactionBlockNums, transactionIds);
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

// NOTE: nullifierBlockNums are the same length and ordered consistently with nullifiers
async function updateSpentNotes(
    tx,
    nullifierBlockNums,
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
            if (note.details && note.details.nullifier) {
                const nullifierIndex = nullifiers.indexOf(note.details.nullifier);
                if (nullifierIndex !== -1) {
                    // If the nullifier is in the list, update the note's status and set nullifierHeight to the index
                    await tx.inputNotes.update(note.noteId, { status: 'Consumed', nullifierHeight: nullifierBlockNums[nullifierIndex] });
                }
            }
        }

         // Pre-parse all details and store them with their respective note ids for quick access
         const parsedOutputNotes = outputNotes.map(note => ({
            noteId: note.noteId,
            details: JSON.parse(note.details)  // Parse the JSON string into an object
        }));

        // Iterate through each parsed note and check against the list of nullifiers
        for (const note of parsedOutputNotes) {
            if (note.details && note.details.nullifier) {
                const nullifierIndex = nullifiers.indexOf(note.details.nullifier);
                if (nullifierIndex !== -1) {
                    // If the nullifier is in the list, update the note's status and set nullifierHeight to the index
                    await tx.outputNotes.update(note.noteId, { status: 'Consumed', nullifierHeight: nullifierBlockNums[nullifierIndex] });
                }
            }
        }
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
            hasClientNotes: hasClientNotes.toString()
        };

        await tx.blockHeaders.add(data);
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw error;
    }
}

async function updateChainMmrNodes(
    tx,
    nodeIndexes,
    nodes
) {
    try {
        // Check if the arrays are not of the same length
        if (nodeIndexes.length !== nodes.length) {
            throw new Error("nodeIndexes and nodes arrays must be of the same length");
        }

        if (nodeIndexes.length === 0) {
            return;
        }

        // Create the updates array with objects matching the structure expected by your IndexedDB schema
        const updates = nodeIndexes.map((index, i) => ({
            id: index,  // Assuming 'index' is the primary key or part of it
            node: nodes[i] // Other attributes of the object
        }));

        // Perform bulk update or insertion; assumes tx.chainMmrNodes is a valid table reference in a transaction
        await tx.chainMmrNodes.bulkAdd(updates);
    } catch (err) {
        console.error("Failed to update chain mmr nodes: ", err);
        throw error;
    }
}

async function updateCommittedNotes(
    tx, 
    outputNoteIds, 
    outputNoteInclusionProofs,
    inputNoteIds,
    inputNoteInclusionProofs,
    inputNoteMetadatas
) {
    try {
        if (outputNoteIds.length !== outputNoteInclusionProofs.length) {
            throw new Error("Arrays outputNoteIds and outputNoteInclusionProofs must be of the same length");
        }

        if (
            inputNoteIds.length !== inputNoteInclusionProofs.length && 
            inputNoteIds.length !== inputNoteMetadatas.length && 
            inputNoteInclusionProofs.length !== inputNoteMetadatas.length
        ) {
            throw new Error("Arrays inputNoteIds and inputNoteInclusionProofs and inputNoteMetadatas must be of the same length");
        }

        for (let i = 0; i < outputNoteIds.length; i++) {
            const noteId = outputNoteIds[i];
            const inclusionProof = outputNoteInclusionProofs[i];

            // Update output notes
            await tx.outputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
                inclusionProof: inclusionProof
            });
        }

        for (let i = 0; i < inputNoteIds.length; i++) {
            const noteId = inputNoteIds[i];
            const inclusionProof = inputNoteInclusionProofs[i];
            const metadata = inputNoteMetadatas[i];

            // Update input notes
            await tx.inputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
                inclusionProof: inclusionProof,
                metadata: metadata
            });
        }
    } catch (error) {
        console.error("Error updating committed notes:", error);
        throw error;
    }
}

async function updateCommittedTransactions(
    tx, 
    blockNums, 
    transactionIds
) {
    try {
        if (transactionIds.length === 0) {
            return;
        }

        // Fetch existing records
        const existingRecords = await tx.transactions.where('id').anyOf(transactionIds).toArray();

        // Create a mapping of transaction IDs to block numbers
        const transactionBlockMap = transactionIds.reduce((map, id, index) => {
            map[id] = blockNums[index];
            return map;
        }, {});

        // Create updates by merging existing records with the new values
        const updates = existingRecords.map(record => ({
            ...record, // Spread existing fields
            commitHeight: transactionBlockMap[record.id] // Update specific field
        }));

        // Perform the update
        await tx.transactions.bulkPut(updates);
    } catch (err) {
        console.error("Failed to mark transactions as committed: ", err);
        throw err;
    }
}

export async function updateIgnoredNotesForTag(
    tag
) {
    try {
        await inputNotes
            .where('importedTag')
            .equals(tag)
            .modify(note => {
                note.ignored = false;
            });
    } catch (err) {
        console.error("Failed to update ignored field for notes: ", err);
        throw err;
    }
}

export function uint8ArrayToBase64(bytes) {
  const binary = bytes.reduce(
    (acc, byte) => acc + String.fromCharCode(byte),
    ""
  );
  return btoa(binary);
}
