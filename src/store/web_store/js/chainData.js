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
        const data = {
            blockNum: blockNum,
            header: header,
            chainMmrPeaks: chainMmrPeaks,
            hasClientNotes: hasClientNotes
        };

        await blockHeaders.add(data);
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
        const blockHeaderPromises = blockNumbers.map(blockNum => 
            blockHeaders.get(blockNum)
        );

        const results = await Promise.all(blockHeaderPromises);
        
        results.forEach((result, index) => {
            if (result === undefined) {
                results[index] = null;
            } else {
                results[index] = {
                    block_num: results[index].blockNum,
                    header: results[index].header,
                    chain_mmr: results[index].chainMmrPeaks,
                    has_client_notes: results[index].hasClientNotes
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
            .equals(true)
            .toArray();
        return allMatchingRecords;
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
        return {
            peaks: blockHeader.chainMmrPeaks
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
        const chainMmrNodesPromises = ids.map(id =>
            chainMmrNodes.get(id)
        );

        const results = await Promise.all(chainMmrNodesPromises);
        return results;
    } catch (err) {
        console.error("Failed to get chain mmr nodes: ", err);
        throw err;
    }
}