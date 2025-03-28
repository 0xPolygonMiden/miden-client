import { transactions, transactionScripts } from "./schema.js";

const IDS_FILTER_PREFIX = "Ids:";
export async function getTransactions(filter) {
  let transactionRecords;

  try {
    if (filter === "Uncommitted") {
      transactionRecords = await transactions
        .filter(
          (tx) => tx.commitHeight === undefined || tx.commitHeight === null
        )
        .toArray();
    } else if (filter.startsWith(IDS_FILTER_PREFIX)) {
      const idsString = filter.substring(IDS_FILTER_PREFIX.length);
      const ids = idsString.split(",");

      if (ids.length > 0) {
        transactionRecords = await transactions
          .where("id")
          .anyOf(ids)
          .toArray();
      } else {
        transactionRecords = [];
      }
    } else {
      transactionRecords = await transactions.toArray();
    }

    if (transactionRecords.length === 0) {
      return [];
    }

    const scriptRoots = transactionRecords.map((transactionRecord) => {
      return transactionRecord.scriptRoot;
    });

    const scripts = await transactionScripts
      .where("scriptRoot")
      .anyOf(scriptRoots)
      .toArray();

    // Create a map of scriptRoot to script for quick lookup
    const scriptMap = new Map();
    scripts.forEach((script) => {
      scriptMap.set(script.scriptRoot, script.txScript);
    });

    const processedTransactions = await Promise.all(
      transactionRecords.map(async (transactionRecord) => {
        let txScriptBase64 = null;

        if (transactionRecord.scriptRoot) {
          const txScript = scriptMap.get(transactionRecord.scriptRoot);

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
          scriptRoot: transactionRecord.scriptRoot
            ? transactionRecord.scriptRoot
            : null,
          txScript: txScriptBase64,
          blockNum: transactionRecord.blockNum,
          commitHeight: transactionRecord.commitHeight
            ? transactionRecord.commitHeight
            : null,
          discarded: transactionRecord.discarded,
        };

        return data;
      })
    );

    return processedTransactions;
  } catch (err) {
    console.error("Failed to get transactions: ", err.toString());
    throw err;
  }
}

export async function insertTransactionScript(scriptRoot, txScript) {
  try {
    // check if script root already exists
    let record = await transactionScripts
      .where("scriptRoot")
      .equals(scriptRoot)
      .first();

    if (record) {
      return;
    }

    if (!scriptRoot) {
      throw new Error("Script root must be provided");
    }

    let scriptRootArray = new Uint8Array(scriptRoot);
    let scriptRootBase64 = uint8ArrayToBase64(scriptRootArray);

    let txScriptBlob = null;
    if (txScript) {
      txScriptBlob = new Blob([new Uint8Array(txScript)]);
    }

    const data = {
      scriptRoot: scriptRootBase64,
      txScript: txScriptBlob,
    };

    await transactionScripts.add(data);
  } catch (error) {
    // Check if the error is because the record already exists
    if (error.name === "ConstraintError") {
    } else {
      console.error("Failed to insert transaction script: ", error.toString());
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
  scriptRoot,
  blockNum,
  committed
) {
  try {
    let inputNotesBlob = new Blob([new Uint8Array(inputNotes)]);
    let outputNotesBlob = new Blob([new Uint8Array(outputNotes)]);
    let scriptRootBase64 = null;
    if (scriptRoot !== null) {
      let scriptRootArray = new Uint8Array(scriptRoot);
      scriptRootBase64 = uint8ArrayToBase64(scriptRootArray);
    }

    const data = {
      id: transactionId,
      accountId: accountId,
      initAccountState: initAccountState,
      finalAccountState: finalAccountState,
      inputNotes: inputNotesBlob,
      outputNotes: outputNotesBlob,
      scriptRoot: scriptRootBase64,
      blockNum: blockNum,
      commitHeight: committed ? committed : null,
      discarded: false,
    };

    await transactions.add(data);
  } catch (err) {
    console.error("Failed to insert proven transaction data: ", err.toString());
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
