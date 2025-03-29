import {
  db,
  inputNotes,
  outputNotes,
  notesScripts,
  transactions,
} from "./schema.js";

export async function getOutputNotes(states) {
  try {
    let notes;

    // Fetch the records based on the filter
    if (states.length === 0) {
      notes = await outputNotes.toArray();
    } else {
      notes = await outputNotes
        .where("stateDiscriminant")
        .anyOf(states)
        .toArray();
    }

    return await processOutputNotes(notes);
  } catch (err) {
    console.error("Failed to get input notes: ", err.toString());
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
    console.error("Failed to get input notes: ", err.toString());
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
    console.error("Failed to get input notes: ", err.toString());
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
    console.error("Failed to get input notes: ", err.toString());
    throw err;
  }
}

export async function getOutputNotesFromNullifiers(nullifiers) {
  try {
    let notes;

    // Fetch the records based on a list of IDs
    notes = await outputNotes.where("nullifier").anyOf(nullifiers).toArray();

    return await processOutputNotes(notes);
  } catch (err) {
    console.error("Failed to get output notes: ", err.toString());
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
    console.error("Failed to get input notes: ", err.toString());
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
    console.error(
      "Failed to get unspent input note nullifiers: ",
      err.toString()
    );
    throw err;
  }
}

export async function upsertInputNote(
  noteId,
  assets,
  serialNumber,
  inputs,
  noteScriptRoot,
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
        noteScriptRoot,
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
        scriptRoot: noteScriptRoot,
        serializedNoteScript: serializedNoteScriptBlob,
      };

      await tx.notesScripts.put(noteScriptData);
    } catch (error) {
      console.error(`Error inserting note: ${noteId}:`, error);
      throw error;
    }
  });
}

export async function upsertOutputNote(
  noteId,
  assets,
  recipientDigest,
  metadata,
  nullifier,
  expectedHeight,
  stateDiscriminant,
  state
) {
  return db.transaction("rw", outputNotes, notesScripts, async (tx) => {
    try {
      let assetsBlob = new Blob([new Uint8Array(assets)]);
      let metadataBlob = new Blob([new Uint8Array(metadata)]);
      let stateBlob = new Blob([new Uint8Array(state)]);

      // Prepare the data object to insert
      const data = {
        noteId: noteId,
        assets: assetsBlob,
        recipientDigest: recipientDigest,
        metadata: metadataBlob,
        nullifier: nullifier ? nullifier : null,
        expectedHeight: expectedHeight,
        stateDiscriminant,
        state: stateBlob,
      };

      // Perform the insert using Dexie
      await tx.outputNotes.put(data);
    } catch (error) {
      console.error(`Error inserting note: ${noteId}:`, error);
      throw error;
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
      if (note.noteScriptRoot) {
        let record = await notesScripts.get(note.noteScriptRoot);
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
        serialNumber: note.serialNumber,
        inputs: note.inputs,
        createdAt: note.createdAt,
        serializedNoteScript: serializedNoteScriptBase64,
        state: note.state,
      };
    })
  );

  return processedNotes;
}

async function processOutputNotes(notes) {
  // Process each note to convert 'blobField' from Blob to Uint8Array
  const processedNotes = await Promise.all(
    notes.map(async (note) => {
      const assetsArrayBuffer = await note.assets.arrayBuffer();
      const assetsArray = new Uint8Array(assetsArrayBuffer);
      const assetsBase64 = uint8ArrayToBase64(assetsArray);
      note.assets = assetsBase64;

      // Convert the metadata blob to base64
      const metadataArrayBuffer = await note.metadata.arrayBuffer();
      const metadataArray = new Uint8Array(metadataArrayBuffer);
      const metadataBase64 = uint8ArrayToBase64(metadataArray);
      note.metadata = metadataBase64;

      // Convert the state blob to base64
      const stateBuffer = await note.state.arrayBuffer();
      const stateArray = new Uint8Array(stateBuffer);
      const stateBase64 = uint8ArrayToBase64(stateArray);
      note.state = stateBase64;

      return {
        assets: note.assets,
        recipientDigest: note.recipientDigest,
        metadata: note.metadata,
        expectedHeight: note.expectedHeight,
        state: note.state,
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
