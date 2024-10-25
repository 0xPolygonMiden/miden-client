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

const setupMintedNote = async () => {
  const { accountId, faucetId } = await setupWalletAndFaucet();
  const { createdNoteId } = await mintTransaction(accountId, faucetId);

  return { createdNoteId, accountId, faucetId };
};

const setupConsumedNote = async () => {
  const { createdNoteId, accountId, faucetId } = await setupMintedNote();
  await consumeTransaction(accountId, faucetId, createdNoteId);

  return { consumedNoteId: createdNoteId };
};

const getConsumableNotes = async (accountId?: string) => {
  return await testingPage.evaluate(async (_accountId) => {
    const client = window.client;
    let records;
    if (_accountId) {
      const accountId = window.AccountId.from_hex(_accountId);
      records = await client.get_consumable_notes(accountId);
    } else {
      records = await client.get_consumable_notes();
    }

    return records.map((record) => ({
      noteId: record.input_note_record().id().to_string(),
      consumability: record.note_consumability().map((c) => ({
        accountId: c.account_id().to_string(),
        consumableAfterBlock: c.consumable_after_block(),
      })),
    }));
  }, accountId);
};

describe("get_input_note", () => {
  it("retrieve input note that does not exist", async () => {
    await setupWalletAndFaucet();
    await expect(getInputNote(badHexId)).to.be.rejectedWith(
      /Failed to get input note/
    );
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
    expect(noteIds).to.have.lengthOf.at.least(1);
    expect(noteIds).to.include(consumedNoteId);
  });
});

describe("get_consumable_notes", () => {
  it("filter by account", async () => {
    const { createdNoteId: noteId1, accountId: accountId1 } =
      await setupMintedNote();
    await setupMintedNote();

    const result = await getConsumableNotes(accountId1);
    expect(result).to.have.lengthOf(1);
    result.forEach((record) => {
      expect(record.consumability).to.have.lengthOf(1);
      expect(record.consumability[0].accountId).to.equal(accountId1);
      expect(record.noteId).to.equal(noteId1);
    });
  });
  it("no filter by account", async () => {
    const { createdNoteId: noteId1, accountId: accountId1 } =
      await setupMintedNote();
    const { createdNoteId: noteId2, accountId: accountId2 } =
      await setupMintedNote();

    const result = await getConsumableNotes();
    expect(result.map((r) => r.noteId)).to.include.members([noteId1, noteId2]);
    expect(result.map((r) => r.consumability[0].accountId)).to.include.members([
      accountId1,
      accountId2,
    ]);
    expect(result).to.have.lengthOf(2);
  });
});

// TODO:
describe("get_output_note", () => {});

describe("get_output_notes", () => {});
