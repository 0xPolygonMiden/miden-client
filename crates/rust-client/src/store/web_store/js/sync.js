import {
  db,
  stateSync,
  inputNotes,
  outputNotes,
  transactions,
  blockHeaders,
  chainMmrNodes,
  tags,
} from "./schema.js";

export async function getNoteTags() {
  try {
    let records = await tags.toArray();

    let processedRecords = records.map((record) => {
      record.source_note_id =
        record.source_note_id == "" ? null : record.source_note_id;
      record.source_account_id =
        record.source_account_id == "" ? null : record.source_account_id;
      return record;
    });

    return processedRecords;
  } catch (error) {
    console.error("Error fetching tag record:", error.toString());
    return null;
  }
}

export async function getSyncHeight() {
  try {
    const record = await stateSync.get(1); // Since id is the primary key and always 1
    if (record) {
      let data = {
        block_num: record.blockNum,
      };
      return data;
    } else {
      return null;
    }
  } catch (error) {
    console.error("Error fetching sync height:", error.toString());
    return null;
  }
}

export async function addNoteTag(tag, source_note_id, source_account_id) {
  try {
    let tagArray = new Uint8Array(tag);
    let tagBase64 = uint8ArrayToBase64(tagArray);
    await tags.add({
      tag: tagBase64,
      source_note_id: source_note_id ? source_note_id : "",
      source_account_id: source_account_id ? source_account_id : "",
    });
  } catch (err) {
    console.error("Failed to add note tag: ", err);
    throw err;
  }
}

export async function removeNoteTag(tag, source_note_id, source_account_id) {
  try {
    let tagArray = new Uint8Array(tag);
    let tagBase64 = uint8ArrayToBase64(tagArray);

    return await tags
      .where({
        tag: tagBase64,
        source_note_id: source_note_id ? source_note_id : "",
        source_account_id: source_account_id ? source_account_id : "",
      })
      .delete();
  } catch {
    console.log("Failed to remove note tag: ", err.toString());
    throw err;
  }
}

export async function applyStateSync(
  blockNum,
  blockHeader,
  chainMmrPeaks,
  hasClientNotes,
  nodeIndexes,
  nodes,
  inputNoteIds,
  committedTransactionIds,
  transactionBlockNums
) {
  return db.transaction(
    "rw",
    stateSync,
    inputNotes,
    outputNotes,
    transactions,
    blockHeaders,
    chainMmrNodes,
    tags,
    async (tx) => {
      await updateSyncHeight(tx, blockNum);
      await updateBlockHeader(
        tx,
        blockNum,
        blockHeader,
        chainMmrPeaks,
        hasClientNotes
      );
      await updateChainMmrNodes(tx, nodeIndexes, nodes);
      await updateCommittedNoteTags(tx, inputNoteIds);
      await updateCommittedTransactions(
        tx,
        transactionBlockNums,
        committedTransactionIds
      );
    }
  );
}

async function updateSyncHeight(tx, blockNum) {
  try {
    await tx.stateSync.update(1, { blockNum: blockNum });
  } catch (error) {
    console.error("Failed to update sync height: ", error);
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
      hasClientNotes: hasClientNotes.toString(),
    };

    await tx.blockHeaders.add(data);
  } catch (err) {
    console.error("Failed to insert block header: ", err);
    throw err;
  }
}

async function updateChainMmrNodes(tx, nodeIndexes, nodes) {
  try {
    // Check if the arrays are not of the same length
    if (nodeIndexes.length !== nodes.length) {
      throw new Error(
        "nodeIndexes and nodes arrays must be of the same length"
      );
    }

    if (nodeIndexes.length === 0) {
      return;
    }

    // Create the updates array with objects matching the structure expected by your IndexedDB schema
    const updates = nodeIndexes.map((index, i) => ({
      id: index, // Assuming 'index' is the primary key or part of it
      node: nodes[i], // Other attributes of the object
    }));

    // Perform bulk update or insertion; assumes tx.chainMmrNodes is a valid table reference in a transaction
    await tx.chainMmrNodes.bulkAdd(updates);
  } catch (err) {
    console.error("Failed to update chain mmr nodes: ", err);
    throw err;
  }
}

async function updateCommittedNoteTags(tx, inputNoteIds) {
  try {
    for (let i = 0; i < inputNoteIds.length; i++) {
      const noteId = inputNoteIds[i];

      // Remove note tags
      await tx.tags.where("source_note_id").equals(noteId).delete();
    }
  } catch (error) {
    console.error("Error updating committed notes:", error);
    throw error;
  }
}

async function updateCommittedTransactions(tx, blockNums, transactionIds) {
  try {
    if (transactionIds.length === 0) {
      return;
    }

    // Fetch existing records
    const existingRecords = await tx.transactions
      .where("id")
      .anyOf(transactionIds)
      .toArray();

    // Create a mapping of transaction IDs to block numbers
    const transactionBlockMap = transactionIds.reduce((map, id, index) => {
      map[id] = blockNums[index];
      return map;
    }, {});

    // Create updates by merging existing records with the new values
    const updates = existingRecords.map((record) => ({
      ...record, // Spread existing fields
      commitHeight: transactionBlockMap[record.id], // Update specific field
    }));

    // Perform the update
    await tx.transactions.bulkPut(updates);
  } catch (err) {
    console.error("Failed to mark transactions as committed: ", err);
    throw err;
  }
}

function uint8ArrayToBase64(bytes) {
  const binary = bytes.reduce(
    (acc, byte) => acc + String.fromCharCode(byte),
    ""
  );
  return btoa(binary);
}
