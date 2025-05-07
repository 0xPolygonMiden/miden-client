import { blockHeaders, partialBlockchainNodes } from "./schema.js";

// INSERT FUNCTIONS
export async function insertBlockHeader(
  blockNum,
  header,
  partialBlockchainPeaks,
  hasClientNotes
) {
  try {
    const headerBlob = new Blob([new Uint8Array(header)]);
    const partialBlockchainPeaksBlob = new Blob([
      new Uint8Array(partialBlockchainPeaks),
    ]);

    const data = {
      blockNum: blockNum,
      header: headerBlob,
      partialBlockchainPeaks: partialBlockchainPeaksBlob,
      hasClientNotes: hasClientNotes.toString(),
    };

    const existingBlockHeader = await blockHeaders.get(blockNum);

    if (!existingBlockHeader) {
      await blockHeaders.add(data);
    } else {
      console.log("Block header already exists, checking for update.");

      // Update the hasClientNotes if the existing value is false
      if (existingBlockHeader.hasClientNotes === "false" && hasClientNotes) {
        await blockHeaders.update(blockNum, {
          hasClientNotes: hasClientNotes.toString(),
        });
        console.log("Updated hasClientNotes to true.");
      } else {
        console.log("No update needed for hasClientNotes.");
      }
    }
  } catch (err) {
    console.error("Failed to insert block header: ", err.toString());
    throw err;
  }
}

export async function insertPartialBlockchainNodes(ids, nodes) {
  try {
    // Check if the arrays are not of the same length
    if (ids.length !== nodes.length) {
      throw new Error("ids and nodes arrays must be of the same length");
    }

    if (ids.length === 0) {
      return;
    }

    // Create array of objects with id and node
    const data = nodes.map((node, index) => ({
      id: ids[index],
      node: node,
    }));

    // Use bulkPut to add/overwrite the entries
    await partialBlockchainNodes.bulkPut(data);
  } catch (err) {
    console.error(
      "Failed to insert partial blockchain nodes: ",
      err.toString()
    );
    throw err;
  }
}

// GET FUNCTIONS
export async function getBlockHeaders(blockNumbers) {
  try {
    const results = await blockHeaders.bulkGet(blockNumbers);

    const processedResults = await Promise.all(
      results.map(async (result, index) => {
        if (result === undefined) {
          return null;
        } else {
          const headerArrayBuffer = await result.header.arrayBuffer();
          const headerArray = new Uint8Array(headerArrayBuffer);
          const headerBase64 = uint8ArrayToBase64(headerArray);

          const partialBlockchainPeaksArrayBuffer =
            await result.partialBlockchainPeaks.arrayBuffer();
          const partialBlockchainPeaksArray = new Uint8Array(
            partialBlockchainPeaksArrayBuffer
          );
          const partialBlockchainPeaksBase64 = uint8ArrayToBase64(
            partialBlockchainPeaksArray
          );

          return {
            blockNum: result.blockNum,
            header: headerBase64,
            partialBlockchainPeaks: partialBlockchainPeaksBase64,
            hasClientNotes: result.hasClientNotes === "true",
          };
        }
      })
    );

    return processedResults;
  } catch (err) {
    console.error("Failed to get block headers: ", err.toString());
    throw err;
  }
}

export async function getTrackedBlockHeaders() {
  try {
    // Fetch all records matching the given root
    const allMatchingRecords = await blockHeaders
      .where("hasClientNotes")
      .equals("true")
      .toArray();

    // Process all records with async operations
    const processedRecords = await Promise.all(
      allMatchingRecords.map(async (record) => {
        const headerArrayBuffer = await record.header.arrayBuffer();
        const headerArray = new Uint8Array(headerArrayBuffer);
        const headerBase64 = uint8ArrayToBase64(headerArray);

        const partialBlockchainPeaksArrayBuffer =
          await record.partialBlockchainPeaks.arrayBuffer();
        const partialBlockchainPeaksArray = new Uint8Array(
          partialBlockchainPeaksArrayBuffer
        );
        const partialBlockchainPeaksBase64 = uint8ArrayToBase64(
          partialBlockchainPeaksArray
        );

        return {
          blockNum: record.blockNum,
          header: headerBase64,
          partialBlockchainPeaks: partialBlockchainPeaksBase64,
          hasClientNotes: record.hasClientNotes === "true",
        };
      })
    );

    return processedRecords;
  } catch (err) {
    console.error("Failed to get tracked block headers: ", err.toString());
    throw err;
  }
}

export async function getPartialBlockchainPeaksByBlockNum(blockNum) {
  try {
    const blockHeader = await blockHeaders.get(blockNum);

    const partialBlockchainPeaksArrayBuffer =
      await blockHeader.partialBlockchainPeaks.arrayBuffer();
    const partialBlockchainPeaksArray = new Uint8Array(
      partialBlockchainPeaksArrayBuffer
    );
    const partialBlockchainPeaksBase64 = uint8ArrayToBase64(
      partialBlockchainPeaksArray
    );

    return {
      peaks: partialBlockchainPeaksBase64,
    };
  } catch (err) {
    console.error("Failed to get partial blockchain peaks: ", err.toString());
    throw err;
  }
}

export async function getPartialBlockchainNodesAll() {
  try {
    const partialBlockchainNodesAll = await partialBlockchainNodes.toArray();
    return partialBlockchainNodesAll;
  } catch (err) {
    console.error("Failed to get partial blockchain nodes: ", err.toString());
    throw err;
  }
}

export async function getPartialBlockchainNodes(ids) {
  try {
    const results = await partialBlockchainNodes.bulkGet(ids);

    return results;
  } catch (err) {
    console.error("Failed to get partial blockchain nodes: ", err.toString());
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
