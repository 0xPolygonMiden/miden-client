import {
    blockHeaders,
    chainMmrNodes
} from './schema.js'

// INSERT FUNCTIONS
export async function insertBlockHeader(
    blockNum,
    header,
    chainMmrPeaks,
    hasClientNotes
) {
    try {
        const headerBlob = new Blob([new Uint8Array(header)]);
        const chainMmrPeaksBlob = new Blob([new Uint8Array(chainMmrPeaks)]);

        const data = {
            blockNum: blockNum,
            header: headerBlob,
            chainMmrPeaks: chainMmrPeaksBlob,
            hasClientNotes: hasClientNotes.toString()
        };

        const existingBlockHeader = await blockHeaders.get(blockNum);
        
        if (!existingBlockHeader) {
            await blockHeaders.add(data);
        } else {
            console.log("Block header already exists, checking for update.");

            // Update the hasClientNotes if the existing value is false
            if (existingBlockHeader.hasClientNotes === 'false' && hasClientNotes) {
                await blockHeaders.update(blockNum, { hasClientNotes: hasClientNotes.toString() });
                console.log("Updated hasClientNotes to true.");
            } else {
                console.log("No update needed for hasClientNotes.");
            }
        }
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw err;
    }
}

export async function insertChainMmrNodes(
    ids,
    nodes
) {
    try {
        const data = nodes.map((node, index) => {
            return {
                id: ids[index],
                node: node
            }
        });

        await chainMmrNodes.bulkAdd(data);
    } catch (err) {
        console.error("Failed to insert chain mmr nodes: ", err);
        throw err;
    }
}

// GET FUNCTIONS
export async function getBlockHeaders(
    blockNumbers
) {
    try {
        const results = await blockHeaders.bulkGet(blockNumbers);

        results.forEach(async (result, index) => {
            if (result === undefined) {
                results[index] = null;
            } else {
                const headerArrayBuffer = await results[index].header.arrayBuffer();
                const headerArray = new Uint8Array(headerArrayBuffer);
                const headerBase64 = uint8ArrayToBase64(headerArray);

                const chainMmrPeaksArrayBuffer = await results[index].chainMmrPeaks.arrayBuffer();
                const chainMmrPeaksArray = new Uint8Array(chainMmrPeaksArrayBuffer);
                const chainMmrPeaksBase64 = uint8ArrayToBase64(chainMmrPeaksArray);

                results[index] = {
                    block_num: results[index].blockNum,
                    header: headerBase64,
                    chain_mmr: chainMmrPeaksBase64,
                    has_client_notes: results[index].hasClientNotes === "true"
                }
            }
        });

        return results
    } catch (err) {
        console.error("Failed to get block headers: ", err);
        throw err;
    }
}

export async function getTrackedBlockHeaders() {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await blockHeaders
            .where('hasClientNotes')
            .equals("true")
            .toArray();
        // Convert hasClientNotes from string to boolean
        const processedRecords = allMatchingRecords.map(async record => {
            const headerArrayBuffer = await record.header.arrayBuffer();
            const headerArray = new Uint8Array(headerArrayBuffer);
            const headerBase64 = uint8ArrayToBase64(headerArray);

            const chainMmrPeaksArrayBuffer = await record.chainMmrPeaks.arrayBuffer();
            const chainMmrPeaksArray = new Uint8Array(chainMmrPeaksArrayBuffer);
            const chainMmrPeaksBase64 = uint8ArrayToBase64(chainMmrPeaksArray);

            return {
                block_num: record.blockNum,
                header: headerBase64,
                chain_mmr: chainMmrPeaksBase64,
                has_client_notes: record.hasClientNotes === 'true'
            };
        });

        return processedRecords;
    } catch (err) {
        console.error("Failed to get tracked block headers: ", err);
        throw err;
    }
}

export async function getChainMmrPeaksByBlockNum(
    blockNum
) {
    try {
        const blockHeader = await blockHeaders.get(blockNum);

        const chainMmrPeaksArrayBuffer = await blockHeader.chainMmrPeaks.arrayBuffer();
        const chainMmrPeaksArray = new Uint8Array(chainMmrPeaksArrayBuffer);
        const chainMmrPeaksBase64 = uint8ArrayToBase64(chainMmrPeaksArray);

        return {
            peaks: chainMmrPeaksBase64
        };
    } catch (err) {
        console.error("Failed to get chain mmr peaks: ", err);
        throw err;
    }
}

export async function getChainMmrNodesAll() {
    try {
        const chainMmrNodesAll = await chainMmrNodes.toArray();
        return chainMmrNodesAll;
    } catch (err) {
        console.error("Failed to get chain mmr nodes: ", err);
        throw err;
    }
}

export async function getChainMmrNodes(
    ids
) {
    try {
        const results = await chainMmrNodes.bulkGet(ids);

        return results;
    } catch (err) {
        console.error("Failed to get chain mmr nodes: ", err);
        throw err;
    }
}
