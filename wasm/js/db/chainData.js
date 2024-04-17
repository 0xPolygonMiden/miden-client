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
        return `Block header for block ${blockNum} inserted successfully.`
    } catch (err) {
        console.error("Failed to insert block header: ", err);
        throw error;
    }
}

export async function getBlockHeaders(
    blockNumbers
) {
    try {
        const blockHeaderPromises = blockNumbers.map(blockNum => 
            blockHeaders.get(blockNum)
        );

        const results = await Promise.all(blockHeaderPromises);
        return results
    } catch (err) {
        console.error("Failed to get block headers: ", err);
        throw error;
    }
}

export async function getTrackedBlockHeaders() {
    try {
        // Fetch all records matching the given root
        const allMatchingRecords = await blockHeaders
            .where('hasClientNotes')
            .equals(true)
            .toArray();
    } catch (error) {
        console.error("Failed to get tracked block headers: ", err);
        throw error;
    }
}

export async function getChainMmrNodesAll() {
    try {
        const chainMmrNodesAll = await chainMmrNodes.toArray();
        return chainMmrNodesAll;
    } catch (err) {
        console.error("Failed to get chain mmr nodes: ", err);
        throw error;
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
        throw error;
    }
}