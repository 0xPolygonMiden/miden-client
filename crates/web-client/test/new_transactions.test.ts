import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";
import {
  consumeTransaction,
  mintTransaction,
  setupWalletAndFaucet,
} from "./webClientTestUtils";
import { TransactionProver } from "../dist";
import { setupConsumedNote } from "./notes.test";

// NEW_MINT_TRANSACTION TESTS
// =======================================================================================================

describe("new_mint_transactions tests", () => {
  it("new_mint_transaction completes successfully", async () => {
    const { faucetId, accountId } = await setupWalletAndFaucet();
    const result = await mintTransaction(accountId, faucetId);

    expect(result.transactionId).to.not.be.empty;
    expect(result.numOutputNotesCreated).to.equal(1);
    expect(result.nonce).to.equal("1");
  });
});

// NEW_CONSUME_TRANSACTION TESTS
// =======================================================================================================

describe("new_consume_transaction tests", () => {
  it("new_consume_transaction completes successfully", async () => {
    const { faucetId, accountId } = await setupWalletAndFaucet();
    const { createdNoteId } = await mintTransaction(accountId, faucetId);
    const result = await consumeTransaction(accountId, faucetId, createdNoteId);

    expect(result.transactionId).to.not.be.empty;
    expect(result.nonce).to.equal("1");
    expect(result.numConsumedNotes).to.equal(1);
    expect(result.targetAccountBalanace).to.equal("1000");
  });
});

// NEW_SEND_TRANSACTION TESTS
// =======================================================================================================

interface SendTransactionResult {
  senderAccountBalance: string;
  changedTargetBalance: string;
}

export const sendTransaction = async (): Promise<SendTransactionResult> => {
  return await testingPage.evaluate(async () => {
    const client = window.client;

    const senderAccount = await client.new_wallet(
      window.AccountStorageMode.private(),
      true
    );
    const targetAccount = await client.new_wallet(
      window.AccountStorageMode.private(),
      true
    );
    const faucetAccount = await client.new_faucet(
      window.AccountStorageMode.private(),
      false,
      "DAG",
      8,
      BigInt(10000000)
    );
    await client.sync_state();

    let mint_transaction_result = await client.new_mint_transaction(
      senderAccount.id(),
      faucetAccount.id(),
      window.NoteType.private(),
      BigInt(1000)
    );
    let created_notes = mint_transaction_result.created_notes().notes();
    let created_note_ids = created_notes.map((note) => note.id().to_string());
    await window.helpers.waitForTransaction(
      mint_transaction_result.executed_transaction().id().to_hex()
    );

    const senderConsumeTransactionResult = await client.new_consume_transaction(
      senderAccount.id(),
      created_note_ids
    );
    await window.helpers.waitForTransaction(
      senderConsumeTransactionResult.executed_transaction().id().to_hex()
    );

    let send_transaction_result = await client.new_send_transaction(
      senderAccount.id(),
      targetAccount.id(),
      faucetAccount.id(),
      window.NoteType.private(),
      BigInt(100)
    );
    let send_created_notes = send_transaction_result.created_notes().notes();
    let send_created_note_ids = send_created_notes.map((note) =>
      note.id().to_string()
    );
    await window.helpers.waitForTransaction(
      send_transaction_result.executed_transaction().id().to_hex()
    );

    const targetConsumeTransactionResult = await client.new_consume_transaction(
      targetAccount.id(),
      send_created_note_ids
    );
    await window.helpers.waitForTransaction(
      targetConsumeTransactionResult.executed_transaction().id().to_hex()
    );

    const changedSenderAccount = await client.get_account(senderAccount.id());
    const changedTargetAccount = await client.get_account(targetAccount.id());

    return {
      senderAccountBalance: changedSenderAccount
        .vault()
        .get_balance(faucetAccount.id())
        .toString(),
      changedTargetBalance: changedTargetAccount
        .vault()
        .get_balance(faucetAccount.id())
        .toString(),
    };
  });
};

describe("new_send_transaction tests", () => {
  it("new_send_transaction completes successfully", async () => {
    const result = await sendTransaction();

    expect(result.senderAccountBalance).to.equal("900");
    expect(result.changedTargetBalance).to.equal("100");
  });
});

// CUSTOM_TRANSACTIONS TESTS
// =======================================================================================================

export const customTransaction = async (
  asserted_value: string,
  with_custom_prover: boolean
): Promise<void> => {
  return await testingPage.evaluate(
    async (_asserted_value: string, _with_custom_prover: boolean) => {
      const client = window.client;

      const walletAccount = await client.new_wallet(
        window.AccountStorageMode.private(),
        false
      );
      const faucetAccount = await client.new_faucet(
        window.AccountStorageMode.private(),
        false,
        "DAG",
        8,
        BigInt(10000000)
      );
      await client.sync_state();

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
        window.NoteType.private(),
        window.NoteTag.from_account_id(
          walletAccount.id(),
          window.NoteExecutionMode.new_local()
        ),
        window.NoteExecutionHint.none(),
        undefined
      );

      let expectedNoteArgs = noteArgs.map((felt) => felt.as_int());
      let memAddress = "1000";
      let memAddress2 = "1004";
      let expectedNoteArg1 = expectedNoteArgs.slice(0, 4).join(".");
      let expectedNoteArg2 = expectedNoteArgs.slice(4, 8).join(".");
      let note_script = `
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

      let compiledNoteScript = await client.compile_note_script(note_script);
      let noteInputs = new window.NoteInputs(
        new window.FeltArray([
          walletAccount.id().prefix(),
          walletAccount.id().suffix(),
        ])
      );

      const serialNum = window.Word.new_from_u64s(
        new BigUint64Array([BigInt(1), BigInt(2), BigInt(3), BigInt(4)])
      );
      let noteRecipient = new window.NoteRecipient(
        serialNum,
        compiledNoteScript,
        noteInputs
      );

      let note = new window.Note(noteAssets, noteMetadata, noteRecipient);

      // Creating First Custom Transaction Request to Mint the Custom Note
      let transaction_request = new window.TransactionRequestBuilder()
        .with_own_output_notes(
          new window.OutputNotesArray([window.OutputNote.full(note)])
        )
        .build();

      // Execute and Submit Transaction
      let transaction_result = await client.new_transaction(
        faucetAccount.id(),
        transaction_request
      );

      if (_with_custom_prover) {
        await client.submit_transaction_with_prover(
          transaction_result,
          await selectProver()
        );
      } else {
        await client.submit_transaction(transaction_result);
      }

      await window.helpers.waitForTransaction(
        transaction_result.executed_transaction().id().to_hex()
      );

      // Just like in the miden test, you can modify this script to get the execution to fail
      // by modifying the assert
      let tx_script = `
            use.miden::contracts::auth::basic->auth_tx
            use.miden::kernels::tx::prologue
            use.miden::kernels::tx::memory

            begin
                push.0 push.${_asserted_value}
                # => [0, ${_asserted_value}]
                assert_eq

                call.auth_tx::auth_tx_rpo_falcon512
            end
        `;

      // Creating Second Custom Transaction Request to Consume Custom Note
      // with Invalid/Valid Transaction Script
      let transaction_script = await client.compile_tx_script(tx_script);
      let note_id = note.id();
      let note_args_commitment = window.Rpo256.hash_elements(feltArray); // gets consumed by NoteIdAndArgs
      let note_id_and_args = new window.NoteIdAndArgs(
        note_id,
        note_args_commitment.to_word()
      );
      let note_id_and_args_array = new window.NoteIdAndArgsArray([
        note_id_and_args,
      ]);
      let advice_map = new window.AdviceMap();
      let note_args_commitment_2 = window.Rpo256.hash_elements(feltArray);
      advice_map.insert(note_args_commitment_2, feltArray);

      let transaction_request_2 = new window.TransactionRequestBuilder()
        .with_authenticated_input_notes(note_id_and_args_array)
        .with_custom_script(transaction_script)
        .extend_advice_map(advice_map)
        .build();

      // Execute and Submit Transaction
      let transaction_result_2 = await client.new_transaction(
        walletAccount.id(),
        transaction_request_2
      );

      if (_with_custom_prover) {
        await client.submit_transaction_with_prover(
          transaction_result_2,
          await selectProver()
        );
      } else {
        await client.submit_transaction(transaction_result_2);
      }

      await window.helpers.waitForTransaction(
        transaction_result_2.executed_transaction().id().to_hex()
      );
    },
    asserted_value,
    with_custom_prover
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
      const targetAccount = await client.new_wallet(
        window.AccountStorageMode.private(),
        true
      );
      const targetAccountId = targetAccount.id();
      const senderAccountId = window.AccountId.from_hex(_senderAccountId);
      const faucetAccountId = window.AccountId.from_hex(_faucetAccountId);

      // Create custom note with multiple assets to send to target account
      // Error should happen if serial numbers are the same in each set of
      // note assets. Otherwise, the transaction should go through.

      let noteAssets_1 = new window.NoteAssets([
        new window.FungibleAsset(faucetAccountId, amount),
      ]);
      let noteAssets_2 = new window.NoteAssets([
        new window.FungibleAsset(faucetAccountId, amount),
      ]);

      let noteMetadata = new window.NoteMetadata(
        senderAccountId,
        window.NoteType.public(),
        window.NoteTag.from_account_id(
          targetAccountId,
          window.NoteExecutionMode.new_local()
        ),
        window.NoteExecutionHint.none(),
        undefined
      );

      let serialNum1 = window.Word.new_from_u64s(
        new BigUint64Array([BigInt(1), BigInt(2), BigInt(3), BigInt(4)])
      );
      let serialNum2 = window.Word.new_from_u64s(
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

      let note1 = new window.Note(noteAssets_1, noteMetadata, noteRecipient1);
      let note2 = new window.Note(noteAssets_2, noteMetadata, noteRecipient2);

      let transaction_request = new window.TransactionRequestBuilder()
        .with_own_output_notes(
          new window.OutputNotesArray([
            window.OutputNote.full(note1),
            window.OutputNote.full(note2),
          ])
        )
        .build();

      let transactionResult = await client.new_transaction(
        senderAccountId,
        transaction_request
      );

      await client.submit_transaction(transactionResult);

      await window.helpers.waitForTransaction(
        transactionResult.executed_transaction().id().to_hex()
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

// CUSTOM PROVERS TEST
// ================================================================================================

export const selectProver = async (): Promise<TransactionProver> => {
  if (window.remote_prover_url != null) {
    return window.TransactionProver.new_remote_prover(window.remote_prover_url);
  } else {
    return window.TransactionProver.new_local_prover();
  }
};

describe("use custom transaction prover per request", () => {
  it("custom transaction prover completes successfully"),
    async () => {
      await expect(customTransaction("0", true)).to.be.fulfilled;
    };
});
