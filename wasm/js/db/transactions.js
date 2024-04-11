import {
    transactions, 
    transactionScripts,
} from './schema.js'

export async function getTransactions(
    filter
) {
    let transactionRecords;

    try {
        if (filter === 'Uncomitted') {
            transactionRecords = await transactions.where("commitHeight").equals(null).toArray();
        } else {
            transactionRecords = await transactions.toArray();
        }
        const scriptHashes = transactionRecords.map(transactionRecord => transactionRecord.scriptHash);
        const scripts = await transactionScripts.where("scriptHash").anyOf(scriptHashes).toArray();

        // Create a map of scriptHash to script for quick lookup
        const scriptMap = scripts.reduce((map, script) => {
            map[script.scriptHash] = script.program;
            return map;
        }, {});

        const processedTransactions = await Promise.all(transactionRecords.map(async transactionRecord => {
            let scriptHashArrayBuffer = null;
            let scriptProgramArrayBuffer = null;
            
            outputNotesArrayBuffer = await transactionRecord.outputNotes.arrayBuffer();
            if (transactionRecord.scriptHash !== null) {
                scriptHashArrayBuffer = await transactionRecord.scriptHash.arrayBuffer();
            }
            if (scriptMap[transactionRecord.scriptHash] !== null) {
                scriptProgramArrayBuffer = await scriptMap[transactionRecord.scriptHash].arrayBuffer();
            }

            transactionRecord.outputNotes = new Uint8Array(outputNotesArrayBuffer);
            if (scriptHashArrayBuffer !== null) {
                transactionRecord.scriptHash = new Uint8Array(scriptHashArrayBuffer);
            }
            if (scriptProgramArrayBuffer !== null) {
                transactionRecord.scriptProgram = new Uint8Array(scriptProgramArrayBuffer);
            }

            return transactionRecord;
        }));

        return processedTransactions
    } catch {
        console.error("Failed to get transactions: ", err);
        throw err;
    
    }
}

export async function insertTransactionScript(
    scriptHash,
    scriptProgram
) {
    try {
        if (scriptHash === null || scriptProgram === null) {
            throw new Error("Script hash and program must be provided.");
        }
        let scriptHashBlob = new Blob([scriptHash]);
        let scriptProgramBlob = new Blob([scriptProgram]);

        const data = {
            scriptHash: scriptHashBlob,
            program: scriptProgramBlob
        }

        await transactionScripts.add(data);
    } catch (error) {
        // Check if the error is because the record already exists
        if (error.name === 'ConstraintError') {
            console.log("Transaction script already exists, ignoring.");
        } else {
            // Re-throw the error if it's not a constraint error
            throw error;
        }
    }
}

export async function insertProvenTransactionData(
    transactionId,
    accountId,
    initAccountState,
    finalAccountState,
    inputNotes,
    outputNotes,
    scriptProgram,
    scriptHash,
    scriptInputs,
    blockNum,
    committed
) {
    try {
        let scriptProgramBlob = null;
        let scriptHashBlob = null;
        let outputNotesBlob = new Blob([outputNotes]);
        if (scriptProgram !== null) {
            scriptProgramBlob = new Blob([scriptProgram]);
        }
        if (scriptHash !== null) {
            scriptHashBlob = new Blob([scriptHash]);
        }

        const data = {
            id: transactionId,
            accountId: accountId,
            initAccountState: initAccountState,
            finalAccountState: finalAccountState,
            inputNotes: inputNotes,
            outputNotes: outputNotesBlob,
            scriptProgram: scriptProgramBlob,
            scriptHash: scriptHashBlob,
            scriptInputs: scriptInputs,
            blockNum: blockNum,
            commitHeight: committed
        }

        await transactions.add(data);
    } catch (err) {
        console.error("Failed to insert proven transaction data: ", err);
        throw err;
    }
}

export async function markTransactionsAsCommitted(
    blockNum,
    transactionIds
) {
    try {
        const updates = transactionIds.map(transactionId => ({
            id: transactionId,
            commitHeight: blockNum
        }));

        await transactions.bulkPut(updates);
    } catch (err) {
        console.error("Failed to mark transactions as committed: ", err);
        throw err;
    }
}