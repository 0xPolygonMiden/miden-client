import {
    db,
    stateSync,
    inputNotes,
    outputNotes,
    transactions,
    blockHeaders,
    chainMmrNodes,
    tags,
} from './schema.js';

export async function getNoteTags() {
    try {
        let records = await tags.toArray();

        let processedRecords = records.map((record) => {
            record.source_note_id = record.source_note_id == "" ? null : record.source_note_id;
            record.source_account_id = record.source_account_id == "" ? null : record.source_account_id;
            return record;
        });

        return processedRecords;
    } catch (error) {
        console.error('Error fetching tag record:', error.toString());
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
        console.error('Error fetching sync height:', error.toString());
        return null;
    }
}

export async function addNoteTag(
    tag,
    source_note_id,
    source_account_id
) {
    try {
        let tagArray = new Uint8Array(tag);
        let tagBase64 = uint8ArrayToBase64(tagArray);
        await tags.add({
            tag: tagBase64,
            source_note_id: source_note_id ? source_note_id : "",
            source_account_id: source_account_id ? source_account_id : ""
        });
    } catch {
        console.error("Failed to add note tag: ", err);
        throw err;
    }
}

export async function removeNoteTag(
    tag,
    source_note_id,
    source_account_id
) {
    try {
        let tagArray = new Uint8Array(tag);
        let tagBase64 = uint8ArrayToBase64(tagArray);

        return await tags.where({
            tag: tagBase64,
            source_note_id: source_note_id ? source_note_id : "",
            source_account_id: source_account_id ? source_account_id : ""
        }).delete();
    } catch {
        console.log("Failed to remove note tag: ", err.toString());
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
    outputNoteInclusionProofsAsFlattenedVec,
    inputNoteIds,
    transactionIds,
    transactionBlockNums
) {
    return db.transaction('rw', stateSync, inputNotes, outputNotes, transactions, blockHeaders, chainMmrNodes, async (tx) => {
        await updateSyncHeight(tx, blockNum);
        await updateSpentNotes(tx, nullifierBlockNums, nullifiers);
        await updateBlockHeader(tx, blockNum, blockHeader, chainMmrPeaks, hasClientNotes);
        await updateChainMmrNodes(tx, nodeIndexes, nodes);
        await updateCommittedNotes(tx, outputNoteIds, outputNoteInclusionProofsAsFlattenedVec, inputNoteIds);
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
async function updateSpentNotes(tx, nullifierBlockNums, nullifiers) {
    try {
        // Modify all input notes that match any of the nullifiers
        await tx.inputNotes
            .where('nullifier')
            .anyOf(nullifiers)
            .modify((inputNote, ref) => {
                const nullifierIndex = nullifiers.indexOf(inputNote.nullifier);
                if (nullifierIndex !== -1) {
                    ref.status = 'Consumed';
                    ref.nullifierHeight = nullifierBlockNums[nullifierIndex];
                }
            });

        // Modify all output notes that match any of the nullifiers
        await tx.outputNotes
            .where('nullifier')
            .anyOf(nullifiers)
            .modify((outputNote, ref) => {
                const nullifierIndex = nullifiers.indexOf(outputNote.nullifier);
                if (nullifierIndex !== -1) {
                    ref.status = 'Consumed';
                    ref.nullifierHeight = nullifierBlockNums[nullifierIndex];
                }
            });
    } catch (error) {
        console.error("Error updating spent notes:", error);
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
        const headerBlob = new Blob([new Uint8Array(blockHeader)]);
        const chainMmrPeaksBlob = new Blob([new Uint8Array(chainMmrPeaks)]);

        const data = {
            blockNum: blockNum,
            header: headerBlob,
            chainMmrPeaks: chainMmrPeaksBlob,
            hasClientNotes: hasClientNotes.toString()
        };

        await tx.blockHeaders.add(data);
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw err;
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
    outputNoteInclusionProofsAsFlattenedVec,
    inputNoteIds,
) {
    try {
        // Helper function to reconstruct arrays from flattened data
        function reconstructFlattenedVec(flattenedVec) {
            const data = flattenedVec.data();
            const lengths = flattenedVec.lengths();

            let index = 0;
            const result = [];
            lengths.forEach(length => {
                result.push(data.slice(index, index + length));
                index += length;
            });
            return result;
        }

        const outputNoteInclusionProofs = reconstructFlattenedVec(outputNoteInclusionProofsAsFlattenedVec);

        if (outputNoteIds.length !== outputNoteInclusionProofsAsFlattenedVec.num_inner_vecs()) {
            throw new Error("Arrays outputNoteIds and outputNoteInclusionProofs must be of the same length");
        }

        for (let i = 0; i < outputNoteIds.length; i++) {
            const noteId = outputNoteIds[i];
            const inclusionProof = outputNoteInclusionProofs[i];
            const inclusionProofBlob = new Blob([new Uint8Array(inclusionProof)]);

            // Update output notes
            await tx.outputNotes.where({ noteId: noteId }).modify({
                status: 'Committed',
                inclusionProof: inclusionProofBlob
            });
        }

        for (let i = 0; i < inputNoteIds.length; i++) {
            const noteId = inputNoteIds[i];

            // Update input notes
            await tx.inputNotes.where({ noteId: noteId }).modify({
                stateDiscriminant: 2, // STATE_COMMITTED
            });

            // Remove note tags
            await tags.delete({ source_note_id: noteId });
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

function uint8ArrayToBase64(bytes) {
    const binary = bytes.reduce((acc, byte) => acc + String.fromCharCode(byte), '');
    return btoa(binary);
}
