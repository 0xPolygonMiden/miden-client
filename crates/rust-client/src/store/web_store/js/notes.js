import {
  db,
  inputNotes,
  outputNotes,
  notesScripts,
  transactions,
} from "./schema.js";

export async function getOutputNotes(status) {
  try {
    let notes;

    // Fetch the records based on the filter
    if (status === "All") {
      notes = await outputNotes.toArray();
    } else {
      notes = await outputNotes.where("status").equals(status).toArray();
    }

    return await processOutputNotes(notes);
  } catch (err) {
    console.error("Failed to get input notes: ", err);
    throw err;
  }
}

export async function getInputNotes(states) {
  try {
    let notes;

    // Fetch the records based on the filter
    if (states.length === 0) {
      notes = await inputNotes.toArray();
    } else {
      notes = await inputNotes
        .where("stateDiscriminant")
        .anyOf(states)
        .toArray();
    }

    return await processInputNotes(notes);
  } catch (err) {
    console.error("Failed to get input notes: ", err);
    throw err;
  }
}

export async function getIgnoredOutputNotes() {
  try {
    const notes = await outputNotes.where("ignored").equals("true").toArray();

    return await processOutputNotes(notes);
  } catch (err) {
    console.error("Failed to get ignored output notes: ", err);
    throw err;
  }
}

export async function getInputNotesFromIds(noteIds) {
  try {
    let notes;

    // Fetch the records based on a list of IDs
    notes = await inputNotes.where("noteId").anyOf(noteIds).toArray();

    return await processInputNotes(notes);
  } catch (err) {
    console.error("Failed to get input notes: ", err);
    throw err;
  }
}

export async function getInputNotesFromNullifiers(nullifiers) {
  try {
    let notes;

    // Fetch the records based on a list of IDs
    notes = await inputNotes.where("nullifier").anyOf(nullifiers).toArray();

    return await processInputNotes(notes);
  } catch (err) {
    console.error("Failed to get input notes: ", err);
    throw err;
  }
}

export async function getOutputNotesFromIds(noteIds) {
  try {
    let notes;

    // Fetch the records based on a list of IDs
    notes = await outputNotes.where("noteId").anyOf(noteIds).toArray();

    return await processOutputNotes(notes);
  } catch (err) {
    console.error("Failed to get input notes: ", err);
    throw err;
  }
}

export async function getUnspentInputNoteNullifiers() {
  try {
    const notes = await inputNotes
      .where("stateDiscriminant")
      .anyOf([2, 4, 5]) // STATE_COMMITTED, STATE_PROCESSING_AUTHENTICATED, STATE_PROCESSING_UNAUTHENTICATED
      .toArray();
    const nullifiers = notes.map((note) => note.nullifier);

    return nullifiers;
  } catch (err) {
    console.error("Failed to get unspent input note nullifiers: ", err);
    throw err;
  }
}

export async function upsertInputNote(
  noteId,
  assets,
  serialNumber,
  inputs,
  noteScriptHash,
  serializedNoteScript,
  nullifier,
  serializedCreatedAt,
  stateDiscriminant,
  state
) {
  return db.transaction("rw", inputNotes, notesScripts, async (tx) => {
    try {
      let assetsBlob = new Blob([new Uint8Array(assets)]);
      let serialNumberBlob = new Blob([new Uint8Array(serialNumber)]);
      let inputsBlob = new Blob([new Uint8Array(inputs)]);
      let stateBlob = new Blob([new Uint8Array(state)]);

      // Prepare the data object to insert
      const data = {
        noteId: noteId,
        assets: assetsBlob,
        serialNumber: serialNumberBlob,
        inputs: inputsBlob,
        noteScriptHash: noteScriptHash,
        nullifier: nullifier,
        state: stateBlob,
        stateDiscriminant: stateDiscriminant,
        createdAt: serializedCreatedAt,
      };

      // Perform the insert using Dexie
      await tx.inputNotes.put(data);

      let serializedNoteScriptBlob = new Blob([
        new Uint8Array(serializedNoteScript),
      ]);

      const noteScriptData = {
        scriptHash: noteScriptHash,
        serializedNoteScript: serializedNoteScriptBlob,
      };

      await tx.notesScripts.put(noteScriptData);
    } catch {
      console.error(`Error inserting note: ${noteId}:`, error);
      throw error; // Rethrow the error to handle it further up the call chain if needed
    }
  });
}

export async function insertOutputNote(
  noteId,
  assets,
  recipient,
  status,
  metadata,
  nullifier,
  details,
  noteScriptHash,
  serializedNoteScript,
  inclusionProof,
  serializedCreatedAt,
  expectedHeight
) {
  return db.transaction("rw", outputNotes, notesScripts, async (tx) => {
    try {
      let assetsBlob = new Blob([new Uint8Array(assets)]);
      let detailsBlob = details ? new Blob([new Uint8Array(details)]) : null;
      let metadataBlob = new Blob([new Uint8Array(metadata)]);
      let inclusionProofBlob = inclusionProof
        ? new Blob([new Uint8Array(inclusionProof)])
        : null;

      // Prepare the data object to insert
      const data = {
        noteId: noteId,
        assets: assetsBlob,
        recipient: recipient,
        status: status,
        metadata: metadataBlob,
        nullifier: nullifier ? nullifier : null,
        details: detailsBlob,
        noteScriptHash: noteScriptHash ? noteScriptHash : null,
        inclusionProof: inclusionProofBlob,
        consumerTransactionId: null,
        createdAt: serializedCreatedAt,
        expectedHeight: expectedHeight ? expectedHeight : null, // todo change to block_num
        ignored: "false",
        imported_tag: null,
      };

      // Perform the insert using Dexie
      await tx.outputNotes.add(data);

      if (noteScriptHash) {
        const exists = await tx.notesScripts.get(noteScriptHash);
        if (!exists) {
          let serializedNoteScriptBlob = null;
          if (serializedNoteScript) {
            serializedNoteScriptBlob = new Blob([
              new Uint8Array(serializedNoteScript),
            ]);
          }

          const data = {
            scriptHash: noteScriptHash,
            serializedNoteScript: serializedNoteScriptBlob,
          };
          await tx.notesScripts.add(data);
        }
      }
    } catch {
      console.error(`Error inserting note: ${noteId}:`, error);
      throw error; // Rethrow the error to handle it further up the call chain if needed
    }
  });
}

async function processInputNotes(notes) {
  // Fetch all scripts from the scripts table for joining
  const transactionRecords = await transactions.toArray();
  const transactionMap = new Map(
    transactionRecords.map((transaction) => [
      transaction.id,
      transaction.accountId,
    ])
  );

  const processedNotes = await Promise.all(
    notes.map(async (note) => {
      // Convert the assets blob to base64
      const assetsArrayBuffer = await note.assets.arrayBuffer();
      const assetsArray = new Uint8Array(assetsArrayBuffer);
      const assetsBase64 = uint8ArrayToBase64(assetsArray);
      note.assets = assetsBase64;

      const serialNumberBuffer = await note.serialNumber.arrayBuffer();
      const serialNumberArray = new Uint8Array(serialNumberBuffer);
      const serialNumberBase64 = uint8ArrayToBase64(serialNumberArray);
      note.serialNumber = serialNumberBase64;

      const inputsBuffer = await note.inputs.arrayBuffer();
      const inputsArray = new Uint8Array(inputsBuffer);
      const inputsBase64 = uint8ArrayToBase64(inputsArray);
      note.inputs = inputsBase64;

      // Convert the serialized note script blob to base64
      let serializedNoteScriptBase64 = null;
      if (note.noteScriptHash) {
        let record = await notesScripts.get(note.noteScriptHash);
        let serializedNoteScriptArrayBuffer =
          await record.serializedNoteScript.arrayBuffer();
        const serializedNoteScriptArray = new Uint8Array(
          serializedNoteScriptArrayBuffer
        );
        serializedNoteScriptBase64 = uint8ArrayToBase64(
          serializedNoteScriptArray
        );
      }

      const stateBuffer = await note.state.arrayBuffer();
      const stateArray = new Uint8Array(stateBuffer);
      const stateBase64 = uint8ArrayToBase64(stateArray);
      note.state = stateBase64;

      return {
        assets: note.assets,
        serial_number: note.serialNumber,
        inputs: note.inputs,
        created_at: note.createdAt,
        serialized_note_script: serializedNoteScriptBase64,
        state: note.state,
      };
    })
  );

  return processedNotes;
}

async function processOutputNotes(notes) {
  // Fetch all scripts from the scripts table for joining
  const transactionRecords = await transactions.toArray();
  const transactionMap = new Map(
    transactionRecords.map((transaction) => [
      transaction.id,
      transaction.accountId,
    ])
  );

  // Process each note to convert 'blobField' from Blob to Uint8Array
  const processedNotes = await Promise.all(
    notes.map(async (note) => {
      const assetsArrayBuffer = await note.assets.arrayBuffer();
      const assetsArray = new Uint8Array(assetsArrayBuffer);
      const assetsBase64 = uint8ArrayToBase64(assetsArray);
      note.assets = assetsBase64;

      // Convert the details blob to base64
      let detailsBase64 = null;
      if (note.details) {
        const detailsArrayBuffer = await note.details.arrayBuffer();
        const detailsArray = new Uint8Array(detailsArrayBuffer);
        detailsBase64 = uint8ArrayToBase64(detailsArray);
      }

      // Convert the metadata blob to base64
      const metadataArrayBuffer = await note.metadata.arrayBuffer();
      const metadataArray = new Uint8Array(metadataArrayBuffer);
      const metadataBase64 = uint8ArrayToBase64(metadataArray);
      note.metadata = metadataBase64;

      // Convert inclusion proof blob to base64
      let inclusionProofBase64 = null;
      if (note.inclusionProof) {
        const inclusionProofArrayBuffer =
          await note.inclusionProof.arrayBuffer();
        const inclusionProofArray = new Uint8Array(inclusionProofArrayBuffer);
        inclusionProofBase64 = uint8ArrayToBase64(inclusionProofArray);
      }

      let serializedNoteScriptBase64 = null;
      if (note.noteScriptHash) {
        let record = await notesScripts.get(note.noteScriptHash);
        let serializedNoteScriptArrayBuffer =
          await record.serializedNoteScript.arrayBuffer();
        const serializedNoteScriptArray = new Uint8Array(
          serializedNoteScriptArrayBuffer
        );
        serializedNoteScriptBase64 = uint8ArrayToBase64(
          serializedNoteScriptArray
        );
      }

      // Perform a "join" with the transactions table
      let consumerAccountId = null;
      if (transactionMap.has(note.consumerTransactionId)) {
        consumerAccountId = transactionMap.get(note.consumerTransactionId);
      }

      return {
        assets: note.assets,
        details: detailsBase64,
        recipient: note.recipient,
        status: note.status,
        metadata: note.metadata,
        inclusion_proof: inclusionProofBase64,
        serialized_note_script: serializedNoteScriptBase64,
        consumer_account_id: consumerAccountId,
        created_at: note.createdAt,
        expected_height: note.expectedHeight ? note.expectedHeight : null,
        submitted_at: note.submittedAt ? note.submittedAt : null,
        nullifier_height: note.nullifierHeight ? note.nullifierHeight : null,
        ignored: note.ignored === "true",
        imported_tag: note.importedTag ? note.importedTag : null,
      };
    })
  );
  return processedNotes;
}

function uint8ArrayToBase64(bytes) {
  const binary = bytes.reduce(
    (acc, byte) => acc + String.fromCharCode(byte),
    ""
  );
  return btoa(binary);
}
