import {
  db,
  stateSync,
  inputNotes,
  outputNotes,
  transactions,
  blockHeaders,
  partialBlockchainNodes,
  tags,
} from "./schema.js";

export async function getNoteTags() {
  try {
    let records = await tags.toArray();

    let processedRecords = records.map((record) => {
      record.sourceNoteId =
        record.sourceNoteId == "" ? null : record.sourceNoteId;
      record.sourceAccountId =
        record.sourceAccountId == "" ? null : record.sourceAccountId;
      return record;
    });

    return processedRecords;
  } catch (error) {
    console.error("Error fetching tag record:", error.toString());
    throw error;
  }
}

export async function getSyncHeight() {
  try {
    const record = await stateSync.get(1); // Since id is the primary key and always 1
    if (record) {
      let data = {
        blockNum: record.blockNum,
      };
      return data;
    } else {
      return null;
    }
  } catch (error) {
    console.error("Error fetching sync height:", error.toString());
    throw error;
  }
}

export async function addNoteTag(tag, sourceNoteId, sourceAccountId) {
  try {
    let tagArray = new Uint8Array(tag);
    let tagBase64 = uint8ArrayToBase64(tagArray);
    await tags.add({
      tag: tagBase64,
      sourceNoteId: sourceNoteId ? sourceNoteId : "",
      sourceAccountId: sourceAccountId ? sourceAccountId : "",
    });
  } catch (err) {
    console.error("Failed to add note tag: ", err.toString());
    throw err;
  }
}

export async function removeNoteTag(tag, sourceNoteId, sourceAccountId) {
  try {
    let tagArray = new Uint8Array(tag);
    let tagBase64 = uint8ArrayToBase64(tagArray);

    return await tags
      .where({
        tag: tagBase64,
        sourceNoteId: sourceNoteId ? sourceNoteId : "",
        sourceAccountId: sourceAccountId ? sourceAccountId : "",
      })
      .delete();
  } catch (err) {
    console.log("Failed to remove note tag: ", err.toString());
    throw err;
  }
}

export async function applyStateSync(
  blockNum,
  newBlockHeadersAsFlattenedVec,
  newBlockNums,
  partialBlockchainPeaksAsFlattenedVec,
  hasClientNotes,
  nodeIndexes,
  nodes,
  inputNoteIds
) {
  const newBlockHeaders = reconstructFlattenedVec(
    newBlockHeadersAsFlattenedVec
  );
  const partialBlockchainPeaks = reconstructFlattenedVec(
    partialBlockchainPeaksAsFlattenedVec
  );

  return db.transaction(
    "rw",
    stateSync,
    inputNotes,
    outputNotes,
    transactions,
    blockHeaders,
    partialBlockchainNodes,
    tags,
    async (tx) => {
      await updateSyncHeight(tx, blockNum);
      for (let i = 0; i < newBlockHeaders.length; i++) {
        await updateBlockHeader(
          tx,
          newBlockNums[i],
          newBlockHeaders[i],
          partialBlockchainPeaks[i],
          hasClientNotes[i]
        );
      }
      await updatePartialBlockchainNodes(tx, nodeIndexes, nodes);
      await updateCommittedNoteTags(tx, inputNoteIds);
    }
  );
}

async function updateSyncHeight(tx, blockNum) {
  try {
    await tx.stateSync.update(1, { blockNum: blockNum });
  } catch (error) {
    console.error("Failed to update sync height: ", error.toString());
    throw error;
  }
}

async function updateBlockHeader(
  tx,
  blockNum,
  blockHeader,
  partialBlockchainPeaks,
  hasClientNotes
) {
  try {
    const headerBlob = new Blob([new Uint8Array(blockHeader)]);
    const partialBlockchainPeaksBlob = new Blob([
      new Uint8Array(partialBlockchainPeaks),
    ]);

    const data = {
      blockNum: blockNum,
      header: headerBlob,
      partialBlockchainPeaks: partialBlockchainPeaksBlob,
      hasClientNotes: hasClientNotes.toString(),
    };

    await tx.blockHeaders.add(data);
  } catch (err) {
    console.error("Failed to insert block header: ", err.toString());
    throw err;
  }
}

async function updatePartialBlockchainNodes(tx, nodeIndexes, nodes) {
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

    // Create array of objects with id and node
    const data = nodes.map((node, index) => ({
      id: nodeIndexes[index],
      node: node,
    }));

    // Use bulkPut to add/overwrite the entries
    await tx.partialBlockchainNodes.bulkPut(data);
  } catch (err) {
    console.error(
      "Failed to update partial blockchain nodes: ",
      err.toString()
    );
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
    console.error("Error updating committed notes:", error.toString());
    throw error;
  }
}

function uint8ArrayToBase64(bytes) {
  const binary = bytes.reduce(
    (acc, byte) => acc + String.fromCharCode(byte),
    ""
  );
  return btoa(binary);
}

// Helper function to reconstruct arrays from flattened data
function reconstructFlattenedVec(flattenedVec) {
  const data = flattenedVec.data();
  const lengths = flattenedVec.lengths();

  let index = 0;
  const result = [];
  lengths.forEach((length) => {
    result.push(data.slice(index, index + length));
    index += length;
  });
  return result;
}
