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

        if (transactionRecord.discardCause) {
          let discardCauseArrayBuffer =
            await transactionRecord.discardCause.arrayBuffer();
          let discardCauseArray = new Uint8Array(discardCauseArrayBuffer);
          let discardCauseBase64 = uint8ArrayToBase64(discardCauseArray);
          transactionRecord.discardCause = discardCauseBase64;
        }

        let detailsArrayBuffer = await transactionRecord.details.arrayBuffer();
        let detailsArray = new Uint8Array(detailsArrayBuffer);
        let detailsBase64 = uint8ArrayToBase64(detailsArray);
        transactionRecord.details = detailsBase64;

        let data = {
          id: transactionRecord.id,
          details: transactionRecord.details,
          scriptRoot: transactionRecord.scriptRoot
            ? transactionRecord.scriptRoot
            : null,
          txScript: txScriptBase64,
          blockNum: transactionRecord.blockNum,
          commitHeight: transactionRecord.commitHeight
            ? transactionRecord.commitHeight
            : null,
          discardCause: transactionRecord.discardCause
            ? transactionRecord.discardCause
            : null,
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

export async function upsertTransactionRecord(
  transactionId,
  details,
  scriptRoot,
  blockNum,
  committed,
  discardCause
) {
  try {
    let detailsBlob = new Blob([new Uint8Array(details)]);

    let scriptRootBase64 = null;
    if (scriptRoot !== null) {
      let scriptRootArray = new Uint8Array(scriptRoot);
      scriptRootBase64 = uint8ArrayToBase64(scriptRootArray);
    }

    let discardCauseBlob = null;
    if (discardCause !== null) {
      discardCauseBlob = new Blob([new Uint8Array(discardCause)]);
    }

    const data = {
      id: transactionId,
      details: detailsBlob,
      scriptRoot: scriptRootBase64,
      blockNum: blockNum,
      commitHeight: committed ? committed : null,
      discardCause: discardCauseBlob,
    };

    await transactions.put(data);
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
