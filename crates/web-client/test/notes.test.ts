import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  badHexId,
  consumeTransaction,
  fetchAndCacheAccountAuth,
  mintTransaction,
  setupWalletAndFaucet,
} from "./webClientTestUtils";

const getInputNote = async (noteId: string) => {
  return await testingPage.evaluate(async (_noteId) => {
    const client = window.client;
    const note = await client.get_input_note(_noteId);
    return {
      noteId: note.id().to_string(),
    };
  }, noteId);
};

// TODO: Figure out a way to easily pass NoteFilters into the tests
const getInputNotes = async () => {
  return await testingPage.evaluate(async () => {
    const client = window.client;
    const filter = new window.NoteFilter(window.NoteFilterTypes.All);
    const notes = await client.get_input_notes(filter);
    return {
      noteIds: notes.map((note) => note.id().to_string()),
    };
  });
};

const setupConsumedNote = async () => {
  const { accountId, faucetId } = await setupWalletAndFaucet();
  const { createdNoteId } = await mintTransaction(accountId, faucetId);
  await consumeTransaction(accountId, faucetId, createdNoteId);

  return { consumedNoteId: createdNoteId };
};

describe("get_input_note", () => {
  it("retrieve input note that does not exist", async () => {
    await setupWalletAndFaucet();
    try {
      await getInputNote(badHexId);
    } catch (err) {
      expect(err).to.contain("Failed to get input note:");
    }
  });

  it("retrieve an input note that does exist", async () => {
    const { consumedNoteId } = await setupConsumedNote();

    const { noteId } = await getInputNote(consumedNoteId);
    expect(noteId).to.equal(consumedNoteId);
  });
});

describe("get_input_notes", () => {
  it("note exists, note filter all", async () => {
    const { consumedNoteId } = await setupConsumedNote();
    const { noteIds } = await getInputNotes();
    expect(noteIds).to.have.lengthOf(1);
    expect(noteIds[0]).to.equal(consumedNoteId);
  });
});

// TODO:
describe("get_output_note", () => {});

describe("get_output_notes", () => {});
