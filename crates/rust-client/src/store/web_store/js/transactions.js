import { transactions, transactionScripts } from "./schema.js";

export async function getTransactions(filter) {
  let transactionRecords;

  try {
    if (filter === "Uncomitted") {
      transactionRecords = await transactions
        .filter(
          (tx) => tx.commitHeight === undefined || tx.commitHeight === null
        )
        .toArray();
    } else {
      transactionRecords = await transactions.toArray();
    }

    if (transactionRecords.length === 0) {
      return [];
    }

    const scriptCommitments = transactionRecords.map((transactionRecord) => {
      return transactionRecord.scriptCommitment;
    });

    const scripts = await transactionScripts
      .where("scriptCommitment")
      .anyOf(scriptCommitments)
      .toArray();

    // Create a map of scriptCommitment to script for quick lookup
    const scriptMap = new Map();
    scripts.forEach((script) => {
      scriptMap.set(script.scriptCommitment, script.txScript);
    });

    const processedTransactions = await Promise.all(
      transactionRecords.map(async (transactionRecord) => {
        let txScriptBase64 = null;

        if (transactionRecord.scriptCommitment) {
          const txScript = scriptMap.get(transactionRecord.scriptCommitment);

          if (txScript) {
            let txScriptArrayBuffer = await txScript.arrayBuffer();
            let txScriptArray = new Uint8Array(txScriptArrayBuffer);
            txScriptBase64 = uint8ArrayToBase64(txScriptArray);
          }
        }

        let inputNotesArrayBuffer =
          await transactionRecord.inputNotes.arrayBuffer();
        let inputNotesArray = new Uint8Array(inputNotesArrayBuffer);
        let inputNotesBase64 = uint8ArrayToBase64(inputNotesArray);
        transactionRecord.inputNotes = inputNotesBase64;

        let outputNotesArrayBuffer =
          await transactionRecord.outputNotes.arrayBuffer();
        let outputNotesArray = new Uint8Array(outputNotesArrayBuffer);
        let outputNotesBase64 = uint8ArrayToBase64(outputNotesArray);
        transactionRecord.outputNotes = outputNotesBase64;

        let data = {
          id: transactionRecord.id,
          accountId: transactionRecord.accountId,
          initAccountState: transactionRecord.initAccountState,
          finalAccountState: transactionRecord.finalAccountState,
          inputNotes: transactionRecord.inputNotes,
          outputNotes: transactionRecord.outputNotes,
          scriptCommitment: transactionRecord.scriptCommitment
            ? transactionRecord.scriptCommitment
            : null,
          txScript: txScriptBase64,
          blockNum: transactionRecord.blockNum,
          commitHeight: transactionRecord.commitHeight
            ? transactionRecord.commitHeight
            : null,
        };

        return data;
      })
    );

    return processedTransactions;
  } catch {
    console.error("Failed to get transactions: ", err);
    throw err;
  }
}

export async function insertTransactionScript(scriptCommitment, txScript) {
  try {
    // check if script commitment already exists
    let record = await transactionScripts
      .where("scriptCommitment")
      .equals(scriptCommitment)
      .first();

    if (record) {
      return;
    }

    if (!scriptCommitment) {
      throw new Error("Script commitment must be provided");
    }

    let scriptCommitmentArray = new Uint8Array(scriptCommitment);
    let scriptCommitmentBase64 = uint8ArrayToBase64(scriptCommitmentArray);

    let txScriptBlob = null;
    if (txScript) {
      txScriptBlob = new Blob([new Uint8Array(txScript)]);
    }

    const data = {
      scriptCommitment: scriptCommitmentBase64,
      txScript: txScriptBlob,
    };

    await transactionScripts.add(data);
  } catch (error) {
    // Check if the error is because the record already exists
    if (error.name === "ConstraintError") {
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
  scriptCommitment,
  blockNum,
  committed
) {
  try {
    let inputNotesBlob = new Blob([new Uint8Array(inputNotes)]);
    let outputNotesBlob = new Blob([new Uint8Array(outputNotes)]);
    let scriptCommitmentBase64 = null;
    if (scriptCommitment !== null) {
      let scriptCommitmentArray = new Uint8Array(scriptCommitment);
      scriptCommitmentBase64 = uint8ArrayToBase64(scriptCommitmentArray);
    }

    const data = {
      id: transactionId,
      accountId: accountId,
      initAccountState: initAccountState,
      finalAccountState: finalAccountState,
      inputNotes: inputNotesBlob,
      outputNotes: outputNotesBlob,
      scriptCommitment: scriptCommitmentBase64,
      blockNum: blockNum,
      commitHeight: committed ? committed : null,
    };

    await transactions.add(data);
  } catch (err) {
    console.error("Failed to insert proven transaction data: ", err);
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
