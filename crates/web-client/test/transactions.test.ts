import { expect } from 'chai';
import { testingPage } from "./mocha.global.setup.mjs";

// GET_TRANSACTIONS TESTS
// =======================================================================================================

interface GetAllTransactionsResult {
    mint_transaction_result_id: string;
    consume_transaction_result_id: string;
    transaction_ids: string[];
    uncomitted_transaction_ids: string[];
}

export const getAllTransactions = async (): Promise<GetAllTransactionsResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }
        const client = window.client;

        const targetAccount = await client.new_wallet(window.AccountStorageMode.private(), true);
        const faucetAccount = await client.new_faucet(window.AccountStorageMode.private(), false, "DAG", 8, BigInt(10000000));
        await client.sync_state();

        await client.fetch_and_cache_account_auth_by_pub_key(faucetAccount.id());
        let mint_transaction_result = await client.new_mint_transaction(targetAccount.id(), faucetAccount.id(), window.NoteType.private(), BigInt(1000));
        let created_notes = mint_transaction_result.created_notes().notes();
        let created_note_ids = created_notes.map(note => note.id().to_string());
        await new Promise(r => setTimeout(r, 20000)); // TODO: Replace this with loop of sync -> check uncommitted transactions -> sleep
        await client.sync_state();

        await client.fetch_and_cache_account_auth_by_pub_key(targetAccount.id());
        let consumeTransactionResult = await client.new_consume_transaction(targetAccount.id(), created_note_ids);
        await new Promise(r => setTimeout(r, 20000)); // TODO: Replace this with loop of sync -> check uncommitted transactions -> sleep
        await client.sync_state();

        let transactions = await client.get_transactions(window.TransactionFilter.all());
        let uncomitted_transactions = await client.get_transactions(window.TransactionFilter.uncomitted());
        let transaction_ids = transactions.map(transaction => transaction.id().to_hex());
        let uncomitted_transaction_ids = uncomitted_transactions.map(transaction => transaction.id().to_hex());

        return {
            mint_transaction_result_id: mint_transaction_result.executed_transaction().id().to_hex(),
            consume_transaction_result_id: consumeTransactionResult.executed_transaction().id().to_hex(),
            transaction_ids: transaction_ids,
            uncomitted_transaction_ids: uncomitted_transaction_ids
        }
    });
};

interface GetUncomittedTransactionsResult {
    mint_transaction_result_id: string;
    transaction_ids: string[];
    uncomitted_transaction_ids: string[];
}

export const getUncomittedTransactions = async (): Promise<GetUncomittedTransactionsResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }
        const client = window.client;

        const targetAccount = await client.new_wallet(window.AccountStorageMode.private(), true);
        const faucetAccount = await client.new_faucet(window.AccountStorageMode.private(), false, "DAG", 8, BigInt(10000000));
        await client.sync_state();

        await client.fetch_and_cache_account_auth_by_pub_key(faucetAccount.id());
        let mint_transaction_result = await client.new_mint_transaction(targetAccount.id(), faucetAccount.id(), window.NoteType.private(), BigInt(1000));

        let transactions = await client.get_transactions(window.TransactionFilter.all());
        let uncomitted_transactions = await client.get_transactions(window.TransactionFilter.uncomitted());
        let transaction_ids = transactions.map(transaction => transaction.id().to_hex());
        let uncomitted_transaction_ids = uncomitted_transactions.map(transaction => transaction.id().to_hex());

        return {
            mint_transaction_result_id: mint_transaction_result.executed_transaction().id().to_hex(),
            transaction_ids: transaction_ids,
            uncomitted_transaction_ids: uncomitted_transaction_ids
        }
    });
};

interface GetNoTransactionsResult {
    transaction_ids: string[];
}

export const getNoTransactions = async (): Promise<GetNoTransactionsResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }
        const client = window.client;

        let transactions = await client.get_transactions(window.TransactionFilter.all());
        let transaction_ids = transactions.map(transaction => transaction.id().to_hex());

        return {
            transaction_ids: transaction_ids,
        }
    });
};

describe("get_transactions tests", () => {
    beforeEach(async () => {
        await testingPage.evaluate(async () => {
            // Open a connection to the list of databases
            const databases = await indexedDB.databases();
            for (const db of databases) {
                // Delete each database by name
                indexedDB.deleteDatabase(db.name!);
            }
        });
    });

    it("get_transactions retrieves all transactions successfully", async () => {
        const result = await getAllTransactions();

        expect(result.transaction_ids).to.include(result.mint_transaction_result_id);
        expect(result.transaction_ids).to.include(result.consume_transaction_result_id);
        expect(result.uncomitted_transaction_ids.length).to.equal(0);
    });

    it('get_transactions retrieves uncommitted transactions successfully', async () => {
        const result = await getUncomittedTransactions();

        expect(result.transaction_ids).to.include(result.mint_transaction_result_id);
        expect(result.uncomitted_transaction_ids).to.include(result.mint_transaction_result_id);
        expect(result.transaction_ids.length).to.equal(result.uncomitted_transaction_ids.length);
    });

    it('get_transactions retrieves no transactions successfully', async () => {
        const result = await getNoTransactions();

        expect(result.transaction_ids.length).to.equal(0);
    });
});

// COMPILE_TX_SCRIPT TESTS
// =======================================================================================================

interface CompileTxScriptResult {
    scriptHash: string;
}

export const compileTxScript = async (): Promise<CompileTxScriptResult> => {
    return await testingPage.evaluate(async () => {
        if (!window.client) {
            await window.create_client();
        }
        const client = window.client;

        let walletAccount = await client.new_wallet(window.AccountStorageMode.private(), true);

        let account_auth = await client.get_account_auth(walletAccount.id());
        let public_key = account_auth.get_rpo_falcon_512_public_key_as_word();
        let secret_key = account_auth.get_rpo_falcon_512_secret_key_as_felts();
        let transcription_script_input_pair_array = new window.TransactionScriptInputPairArray([new window.TransactionScriptInputPair(public_key, secret_key)]);

        let tx_script = `
            use.miden::contracts::auth::basic->auth_tx
            use.miden::kernels::tx::prologue
            use.miden::kernels::tx::memory

            begin
                push.0 push.0
                # => [0, 0]
                assert_eq

                call.auth_tx::auth_tx_rpo_falcon512
            end
        `;

        const compiledScript = await client.compile_tx_script(tx_script, transcription_script_input_pair_array)

        return {
            scriptHash: compiledScript.hash().to_hex()
        }
    });
};

describe("compile_tx_script tests", () => {
    it("compile_tx_script compiles script successfully", async () => {
        const result = await compileTxScript();

        expect(result.scriptHash).to.not.be.empty;
    });
});
