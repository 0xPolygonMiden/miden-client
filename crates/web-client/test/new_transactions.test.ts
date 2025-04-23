import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  consumeTransaction,
  mintAndConsumeTransaction,
  mintTransaction,
  sendTransaction,
  setupWalletAndFaucet,
} from "./webClientTestUtils";
import { setupConsumedNote } from "./notes.test";
import { Account, TransactionRecord } from "../dist/crates/miden_client_web";

// NEW_MINT_TRANSACTION TESTS
// =======================================================================================================

describe("mint transaction tests", () => {
  const testCases = [
    { flag: false, description: "mint transaction completes successfully" },
    {
      flag: true,
      description: "mint transaction with remote prover completes successfully",
    },
  ];

  testCases.forEach(({ flag, description }) => {
    it(description, async () => {
      const { faucetId, accountId } = await setupWalletAndFaucet();
      const result = await mintTransaction(accountId, faucetId, flag);

      expect(result.transactionId).to.not.be.empty;
      expect(result.numOutputNotesCreated).to.equal(1);
      expect(result.nonce).to.equal("1");
    });
  });
});

// NEW_CONSUME_TRANSACTION TESTS
// =======================================================================================================

describe("consume transaction tests", () => {
  const testCases = [
    { flag: false, description: "consume transaction completes successfully" },
    {
      flag: true,
      description:
        "consume transaction with remote prover completes successfully",
    },
  ];

  testCases.forEach(({ flag, description }) => {
    it(description, async () => {
      const { faucetId, accountId } = await setupWalletAndFaucet();
      const { consumeResult: result } = await mintAndConsumeTransaction(
        accountId,
        faucetId,
        flag
      );

      expect(result.transactionId).to.not.be.empty;
      expect(result.nonce).to.equal("1");
      expect(result.numConsumedNotes).to.equal(1);
      expect(result.targetAccountBalanace).to.equal("1000");
    });
  });
});

// NEW_SEND_TRANSACTION TESTS
// =======================================================================================================

interface SendTransactionResult {
  senderAccountBalance: string;
  changedTargetBalance: string;
}

export const sendTransactionTest = async (
  senderAccount: string,
  targetAccount: string,
  faucetAccount: string
): Promise<SendTransactionResult> => {
  return await testingPage.evaluate(
    async (
      _senderAccount: string,
      _targetAccount: string,
      _faucetAccount: string
    ) => {
      const client = window.client;

      await client.syncState();

      const targetAccountId = window.AccountId.fromHex(_targetAccount);
      const senderAccountId = window.AccountId.fromHex(_senderAccount);
      const faucetAccountId = window.AccountId.fromHex(_faucetAccount);

      const changedSenderAccount = await client.getAccount(senderAccountId);
      const changedTargetAccount = await client.getAccount(targetAccountId);

      return {
        senderAccountBalance: changedSenderAccount
          .vault()
          .getBalance(faucetAccountId)
          .toString(),
        changedTargetBalance: changedTargetAccount
          .vault()
          .getBalance(faucetAccountId)
          .toString(),
      };
    },
    senderAccount,
    targetAccount,
    faucetAccount
  );
};

describe("send transaction tests", () => {
  const testCases = [
    { flag: false, description: "send transaction completes successfully" },
    {
      flag: true,
      description: "send transaction with remote prover completes successfully",
    },
  ];

  testCases.forEach(({ flag, description }) => {
    it(description, async () => {
      const { accountId: senderAccountId, faucetId } =
        await setupWalletAndFaucet();
      const { accountId: targetAccountId } = await setupWalletAndFaucet();
      const recallHeight = 100;
      let createdSendNotes = await sendTransaction(
        senderAccountId,
        targetAccountId,
        faucetId,
        recallHeight,
        flag
      );

      await consumeTransaction(targetAccountId, faucetId, createdSendNotes[0]);
      const result = await sendTransactionTest(
        senderAccountId,
        targetAccountId,
        faucetId
      );

      expect(result.senderAccountBalance).to.equal("900");
      expect(result.changedTargetBalance).to.equal("100");
    });
  });
});

// CUSTOM_TRANSACTIONS TESTS
// =======================================================================================================

export const customTransaction = async (
  assertedValue: string,
  withRemoteProver: boolean
): Promise<void> => {
  return await testingPage.evaluate(
    async (_assertedValue: string, _withRemoteProver: boolean) => {
      const client = window.client;

      const walletAccount = await client.newWallet(
        window.AccountStorageMode.private(),
        false
      );
      const faucetAccount = await client.newFaucet(
        window.AccountStorageMode.private(),
        false,
        "DAG",
        8,
        BigInt(10000000)
      );
      await client.syncState();

      // Creating Custom Note which needs the following:
      // - Note Assets
      // - Note Metadata
      // - Note Recipient

      // Creating NOTE_ARGS
      let felt1 = new window.Felt(BigInt(9));
      let felt2 = new window.Felt(BigInt(12));
      let felt3 = new window.Felt(BigInt(18));
      let felt4 = new window.Felt(BigInt(3));
      let felt5 = new window.Felt(BigInt(3));
      let felt6 = new window.Felt(BigInt(18));
      let felt7 = new window.Felt(BigInt(12));
      let felt8 = new window.Felt(BigInt(9));

      let noteArgs = [felt1, felt2, felt3, felt4, felt5, felt6, felt7, felt8];
      let feltArray = new window.FeltArray();
      noteArgs.forEach((felt) => feltArray.append(felt));

      let noteAssets = new window.NoteAssets([
        new window.FungibleAsset(faucetAccount.id(), BigInt(10)),
      ]);

      let noteMetadata = new window.NoteMetadata(
        faucetAccount.id(),
        window.NoteType.Private,
        window.NoteTag.fromAccountId(
          walletAccount.id(),
          window.NoteExecutionMode.newLocal()
        ),
        window.NoteExecutionHint.none(),
        undefined
      );

      let expectedNoteArgs = noteArgs.map((felt) => felt.asInt());
      let memAddress = "1000";
      let memAddress2 = "1004";
      let expectedNoteArg1 = expectedNoteArgs.slice(0, 4).join(".");
      let expectedNoteArg2 = expectedNoteArgs.slice(4, 8).join(".");
      let noteScript = `
            # Custom P2ID note script
            #
            # This note script asserts that the note args are exactly the same as passed
            # (currently defined as {expected_note_arg_1} and {expected_note_arg_2}).
            # Since the args are too big to fit in a single note arg, we provide them via advice inputs and
            # address them via their commitment (noted as NOTE_ARG)
            # This note script is based off of the P2ID note script because notes currently need to have
            # assets, otherwise it could have been boiled down to the assert.

            use.miden::account
            use.miden::note
            use.miden::contracts::wallets::basic->wallet
            use.std::mem


            proc.add_note_assets_to_account
                push.0 exec.note::get_assets
                # => [num_of_assets, 0 = ptr, ...]

                # compute the pointer at which we should stop iterating
                mul.4 dup.1 add
                # => [end_ptr, ptr, ...]

                # pad the stack and move the pointer to the top
                padw movup.5
                # => [ptr, 0, 0, 0, 0, end_ptr, ...]

                # compute the loop latch
                dup dup.6 neq
                # => [latch, ptr, 0, 0, 0, 0, end_ptr, ...]

                while.true
                    # => [ptr, 0, 0, 0, 0, end_ptr, ...]

                    # save the pointer so that we can use it later
                    dup movdn.5
                    # => [ptr, 0, 0, 0, 0, ptr, end_ptr, ...]

                    # load the asset
                    mem_loadw
                    # => [ASSET, ptr, end_ptr, ...]

                    # pad the stack before call
                    padw swapw padw padw swapdw
                    # => [ASSET, pad(12), ptr, end_ptr, ...]

                    # add asset to the account
                    call.wallet::receive_asset
                    # => [pad(16), ptr, end_ptr, ...]

                    # clean the stack after call
                    dropw dropw dropw
                    # => [0, 0, 0, 0, ptr, end_ptr, ...]

                    # increment the pointer and compare it to the end_ptr
                    movup.4 add.4 dup dup.6 neq
                    # => [latch, ptr+4, ASSET, end_ptr, ...]
                end

                # clear the stack
                drop dropw drop
            end

            begin
                # push data from the advice map into the advice stack
                adv.push_mapval
                # => [NOTE_ARG]

                # memory address where to write the data
                push.${memAddress}
                # => [target_mem_addr, NOTE_ARG_COMMITMENT]
                # number of words
                push.2
                # => [number_of_words, target_mem_addr, NOTE_ARG_COMMITMENT]
                exec.mem::pipe_preimage_to_memory
                # => [target_mem_addr']
                dropw
                # => []

                # read first word
                push.${memAddress}
                # => [data_mem_address]
                mem_loadw
                # => [NOTE_ARG_1]

                push.${expectedNoteArg1} assert_eqw.err=101
                # => []

                # read second word
                push.${memAddress2}
                # => [data_mem_address_2]
                mem_loadw
                # => [NOTE_ARG_2]

                push.${expectedNoteArg2} assert_eqw.err=102
                # => []

                # store the note inputs to memory starting at address 0
                push.0 exec.note::get_inputs
                # => [num_inputs, inputs_ptr]

                # make sure the number of inputs is 1
                eq.2 assert.err=103
                # => [inputs_ptr]

                # read the target account id from the note inputs
                mem_load
                # => [target_account_id_prefix]

                exec.account::get_id swap drop
                # => [account_id_prefix, target_account_id_prefix, ...]

                # ensure account_id = target_account_id, fails otherwise
                assert_eq.err=104
                # => [...]

                exec.add_note_assets_to_account
                # => [...]
            end
        `;

      let compiledNoteScript = await client.compileNoteScript(noteScript);
      let noteInputs = new window.NoteInputs(
        new window.FeltArray([
          walletAccount.id().prefix(),
          walletAccount.id().suffix(),
        ])
      );

      const serialNum = window.Word.newFromU64s(
        new BigUint64Array([BigInt(1), BigInt(2), BigInt(3), BigInt(4)])
      );
      let noteRecipient = new window.NoteRecipient(
        serialNum,
        compiledNoteScript,
        noteInputs
      );

      let note = new window.Note(noteAssets, noteMetadata, noteRecipient);

      // Creating First Custom Transaction Request to Mint the Custom Note
      let transactionRequest = new window.TransactionRequestBuilder()
        .withOwnOutputNotes(
          new window.OutputNotesArray([window.OutputNote.full(note)])
        )
        .build();

      // Execute and Submit Transaction
      let transactionResult = await client.newTransaction(
        faucetAccount.id(),
        transactionRequest
      );

      if (_withRemoteProver && window.remoteProverUrl != null) {
        await client.submitTransaction(
          transactionResult,
          window.remoteProverInstance
        );
      } else {
        await client.submitTransaction(transactionResult);
      }

      await window.helpers.waitForTransaction(
        transactionResult.executedTransaction().id().toHex()
      );

      // Just like in the miden test, you can modify this script to get the execution to fail
      // by modifying the assert
      let txScript = `
            use.miden::contracts::auth::basic->auth_tx
            use.miden::kernels::tx::prologue
            use.miden::kernels::tx::memory

            begin
                push.0 push.${_assertedValue}
                # => [0, ${_assertedValue}]
                assert_eq

                call.auth_tx::auth_tx_rpo_falcon512
            end
        `;

      // Creating Second Custom Transaction Request to Consume Custom Note
      // with Invalid/Valid Transaction Script
      let transactionScript = await client.compileTxScript(txScript);
      let noteId = note.id();
      let noteArgsCommitment = window.Rpo256.hashElements(feltArray); // gets consumed by NoteIdAndArgs
      let noteIdAndArgs = new window.NoteIdAndArgs(
        noteId,
        noteArgsCommitment.toWord()
      );
      let noteIdAndArgsArray = new window.NoteIdAndArgsArray([noteIdAndArgs]);
      let adviceMap = new window.AdviceMap();
      let noteArgsCommitment2 = window.Rpo256.hashElements(feltArray);
      adviceMap.insert(noteArgsCommitment2, feltArray);

      let transactionRequest2 = new window.TransactionRequestBuilder()
        .withAuthenticatedInputNotes(noteIdAndArgsArray)
        .withCustomScript(transactionScript)
        .extendAdviceMap(adviceMap)
        .build();

      // Execute and Submit Transaction
      let transactionResult2 = await client.newTransaction(
        walletAccount.id(),
        transactionRequest2
      );

      if (_withRemoteProver && window.remoteProverUrl != null) {
        await client.submitTransaction(
          transactionResult2,
          window.remoteProverInstance
        );
      } else {
        await client.submitTransaction(transactionResult2);
      }

      await window.helpers.waitForTransaction(
        transactionResult2.executedTransaction().id().toHex()
      );
    },
    assertedValue,
    withRemoteProver
  );
};

const customTxWithMultipleNotes = async (
  isSerialNumSame: boolean,
  senderAccountId: string,
  faucetAccountId: string
) => {
  return await testingPage.evaluate(
    async (_isSerialNumSame, _senderAccountId, _faucetAccountId) => {
      const client = window.client;
      const amount = BigInt(10);
      const targetAccount = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const targetAccountId = targetAccount.id();
      const senderAccountId = window.AccountId.fromHex(_senderAccountId);
      const faucetAccountId = window.AccountId.fromHex(_faucetAccountId);

      // Create custom note with multiple assets to send to target account
      // Error should happen if serial numbers are the same in each set of
      // note assets. Otherwise, the transaction should go through.

      let noteAssets1 = new window.NoteAssets([
        new window.FungibleAsset(faucetAccountId, amount),
      ]);
      let noteAssets2 = new window.NoteAssets([
        new window.FungibleAsset(faucetAccountId, amount),
      ]);

      let noteMetadata = new window.NoteMetadata(
        senderAccountId,
        window.NoteType.Public,
        window.NoteTag.fromAccountId(
          targetAccountId,
          window.NoteExecutionMode.newLocal()
        ),
        window.NoteExecutionHint.none(),
        undefined
      );

      let serialNum1 = window.Word.newFromU64s(
        new BigUint64Array([BigInt(1), BigInt(2), BigInt(3), BigInt(4)])
      );
      let serialNum2 = window.Word.newFromU64s(
        new BigUint64Array([BigInt(5), BigInt(6), BigInt(7), BigInt(8)])
      );

      const p2idScript = window.NoteScript.p2id();

      let noteInputs = new window.NoteInputs(
        new window.FeltArray([
          targetAccount.id().suffix(),
          targetAccount.id().prefix(),
        ])
      );

      let noteRecipient1 = new window.NoteRecipient(
        serialNum1,
        p2idScript,
        noteInputs
      );
      let noteRecipient2 = new window.NoteRecipient(
        _isSerialNumSame ? serialNum1 : serialNum2,
        p2idScript,
        noteInputs
      );

      let note1 = new window.Note(noteAssets1, noteMetadata, noteRecipient1);
      let note2 = new window.Note(noteAssets2, noteMetadata, noteRecipient2);

      let transactionRequest = new window.TransactionRequestBuilder()
        .withOwnOutputNotes(
          new window.OutputNotesArray([
            window.OutputNote.full(note1),
            window.OutputNote.full(note2),
          ])
        )
        .build();

      let transactionResult = await client.newTransaction(
        senderAccountId,
        transactionRequest
      );

      await client.submitTransaction(transactionResult);

      await window.helpers.waitForTransaction(
        transactionResult.executedTransaction().id().toHex()
      );
    },
    isSerialNumSame,
    senderAccountId,
    faucetAccountId
  );
};

describe("custom transaction tests", () => {
  it("custom transaction completes successfully", async () => {
    await expect(customTransaction("0", false)).to.be.fulfilled;
  });

  it("custom transaction fails", async () => {
    await expect(customTransaction("1", false)).to.be.rejected;
  });

  it("custom transaction with remote prover completes successfully", async () => {
    await expect(customTransaction("0", true)).to.be.fulfilled;
  });
});

describe("custom transaction with multiple output notes", () => {
  const testCases = [
    {
      description: "does not fail when output note serial numbers are unique",
      shouldFail: false,
    },
    {
      description: "fails when output note serial numbers are the same",
      shouldFail: true,
    },
  ];

  testCases.forEach(({ description, shouldFail }) => {
    it(description, async () => {
      const { accountId, faucetId } = await setupConsumedNote();
      if (shouldFail) {
        await expect(customTxWithMultipleNotes(shouldFail, accountId, faucetId))
          .to.be.rejected;
      } else {
        await expect(customTxWithMultipleNotes(shouldFail, accountId, faucetId))
          .to.be.fulfilled;
      }
    });
  });
});

// DISCARDED TRANSACTIONS TESTS
// ================================================================================================

interface DiscardedTransactionResult {
  discardedTransactions: TransactionRecord[];
  commitmentBeforeTx: string;
  commitmentAfterTx: string;
  commitmentAfterDiscardedTx: string;
}

export const discardedTransaction =
  async (): Promise<DiscardedTransactionResult> => {
    return await testingPage.evaluate(async () => {
      const client = window.client;

      const senderAccount = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const targetAccount = await client.newWallet(
        window.AccountStorageMode.private(),
        true
      );
      const faucetAccount = await client.newFaucet(
        window.AccountStorageMode.private(),
        false,
        "DAG",
        8,
        BigInt(10000000)
      );
      await client.syncState();

      let mintTransactionRequest = client.newMintTransactionRequest(
        senderAccount.id(),
        faucetAccount.id(),
        window.NoteType.Private,
        BigInt(1000)
      );
      let mintTransactionResult = await client.newTransaction(
        faucetAccount.id(),
        mintTransactionRequest
      );
      await client.submitTransaction(mintTransactionResult);
      let createdNotes = mintTransactionResult.createdNotes().notes();
      let createdNoteIds = createdNotes.map((note) => note.id().toString());
      await window.helpers.waitForTransaction(
        mintTransactionResult.executedTransaction().id().toHex()
      );

      const senderConsumeTransactionRequest =
        client.newConsumeTransactionRequest(createdNoteIds);
      let senderConsumeTransactionResult = await client.newTransaction(
        senderAccount.id(),
        senderConsumeTransactionRequest
      );
      await client.submitTransaction(senderConsumeTransactionResult);
      await window.helpers.waitForTransaction(
        senderConsumeTransactionResult.executedTransaction().id().toHex()
      );

      let sendTransactionRequest = client.newSendTransactionRequest(
        senderAccount.id(),
        targetAccount.id(),
        faucetAccount.id(),
        window.NoteType.Private,
        BigInt(100),
        1
      );
      let sendTransactionResult = await client.newTransaction(
        senderAccount.id(),
        sendTransactionRequest
      );
      await client.submitTransaction(sendTransactionResult);
      let sendCreatedNotes = sendTransactionResult.createdNotes().notes();
      let sendCreatedNoteIds = sendCreatedNotes.map((note) =>
        note.id().toString()
      );

      await window.helpers.waitForTransaction(
        sendTransactionResult.executedTransaction().id().toHex()
      );

      let noteIdAndArgs = new window.NoteIdAndArgs(
        sendCreatedNotes[0].id(),
        null
      );
      let noteIdAndArgsArray = new window.NoteIdAndArgsArray([noteIdAndArgs]);
      const consumeTransactionRequest = new window.TransactionRequestBuilder()
        .withAuthenticatedInputNotes(noteIdAndArgsArray)
        .build();

      let preConsumeStore = await client.exportStore();

      // Sender retrieves the note
      let senderTxRequest =
        await client.newConsumeTransactionRequest(sendCreatedNoteIds);
      let senderTxResult = await client.newTransaction(
        senderAccount.id(),
        senderTxRequest
      );
      await client.submitTransaction(senderTxResult);
      await window.helpers.waitForTransaction(
        senderTxResult.executedTransaction().id().toHex()
      );

      await client.forceImportStore(preConsumeStore);

      // Get the account state before the transaction is applied
      const accountStateBeforeTx = (await client.getAccount(
        targetAccount.id()
      )) as Account;
      if (!accountStateBeforeTx) {
        throw new Error("Failed to get account state before transaction");
      }

      // Target tries consuming but the transaction will not be submitted
      let targetTxResult = await client.newTransaction(
        targetAccount.id(),
        consumeTransactionRequest
      );

      await client.testingApplyTransaction(targetTxResult);
      // Get the account state after the transaction is applied
      const accountStateAfterTx = (await client.getAccount(
        targetAccount.id()
      )) as Account;
      if (!accountStateAfterTx) {
        throw new Error("Failed to get account state after transaction");
      }

      await client.syncState();

      const allTransactions = await client.getTransactions(
        window.TransactionFilter.all()
      );

      const discardedTransactions = allTransactions.filter((tx) =>
        tx.transactionStatus().isDiscarded()
      );

      // Get the account state after the discarded transactions are applied
      const accountStateAfterDiscardedTx = (await client.getAccount(
        targetAccount.id()
      )) as Account;
      if (!accountStateAfterDiscardedTx) {
        throw new Error(
          "Failed to get account state after discarded transaction"
        );
      }

      // Perform a `.commitment()` check on each account
      const commitmentBeforeTx = accountStateBeforeTx.commitment().toHex();
      const commitmentAfterTx = accountStateAfterTx.commitment().toHex();
      const commitmentAfterDiscardedTx = accountStateAfterDiscardedTx
        .commitment()
        .toHex();

      return {
        discardedTransactions: discardedTransactions,
        commitmentBeforeTx,
        commitmentAfterTx,
        commitmentAfterDiscardedTx,
      };
    });
  };

describe("discarded_transaction tests", () => {
  it("transaction gets discarded", async () => {
    const result = await discardedTransaction();

    expect(result.discardedTransactions.length).to.equal(1);
    expect(result.commitmentBeforeTx).to.equal(
      result.commitmentAfterDiscardedTx
    );
    expect(result.commitmentAfterTx).to.not.equal(
      result.commitmentAfterDiscardedTx
    );
  });
});
