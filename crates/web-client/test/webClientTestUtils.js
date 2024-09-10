import { expect } from "chai";
import { testingPage } from "./mocha.global.setup.mjs";

/**
 * @typedef {import("../dist/index").WebClient} WebClient
 */

/**
 *
 * @param {string} storageMode
 * @param {boolean} mutable
 *
 * @returns {Promise<string>} The new wallet identifier as a string.
 */
export const createNewWallet = async (storageMode, mutable) => {
  return await testingPage.evaluate(
    async (_storageMode, _mutable) => {
      if (!window.client) {
        await window.create_client();
      }

      /** @type {WebClient} */
      const client = window.client;
      const newWallet = client.new_wallet(_storageMode, _mutable);

      return newWallet;
    },
    storageMode,
    mutable
  );
};

/**
 *
 * @returns {Promise<any[]>}
 */
export const getAccounts = async () => {
  return await testingPage.evaluate(async () => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return client.get_accounts();
  });
};

/**
 *
 * @param {string} accountId
 *
 * @returns {Promise<any>}
 */
export const getAccount = async (accountId) => {
  return await testingPage.evaluate(async (_accountId) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    console.log("_accountId: ", _accountId);
    return client.get_account(_accountId);
  }, accountId);
};

/**
 *
 * @param {string} storageMode
 * @param {boolean} nonFungible
 * @param {string} tokenSymbol
 * @param {string} decimals
 * @param {string} maxSupply
 * @returns {Promise<any>}
 */
export const createNewFaucet = async (
  storageMode,
  nonFungible,
  tokenSymbol,
  decimals,
  maxSupply
) => {
  return await testingPage.evaluate(
    async (_storageMode, _nonFungible, _tokenSymbol, _decimals, _maxSupply) => {
      if (!window.client) {
        await window.create_client();
      }
      console.log("creating new faucet...");
      /** @type {WebClient} */
      const client = window.client;
      return client.new_faucet(
        _storageMode,
        _nonFungible,
        _tokenSymbol,
        _decimals,
        _maxSupply
      );
    },
    storageMode,
    nonFungible,
    tokenSymbol,
    decimals,
    maxSupply
  );
};

/**
 *
 * @param {string} targetAccountId
 * @param {string} faucetId
 * @param {string} noteType
 * @param {string} amount
 * @returns {Promise<NewTransactionResult>}
 */
export const createNewMintTransaction = async (
  targetAccountId,
  faucetId,
  noteType,
  amount
) => {
  return await testingPage.evaluate(
    async (_targetAccountId, _faucetId, _noteType, _amount) => {
      if (!window.client) {
        await window.create_client();
      }

      /** @type {WebClient} */
      const client = window.client;
      return client.new_mint_transaction(
        _targetAccountId,
        _faucetId,
        _noteType,
        _amount
      );
    },
    targetAccountId,
    faucetId,
    noteType,
    amount
  );
};

/**
 *
 * @param {string} accountId
 * @param {string[]} listOfNotes
 * @returns {Promise<NewTransactionResult>}
 */
export const createNewConsumeTransaction = async (accountId, listOfNotes) => {
  return await testingPage.evaluate(
    async (_accountId, _listOfNotes) => {
      if (!window.client) {
        await window.create_client();
      }

      /** @type {WebClient} */
      const client = window.client;
      return client.new_consume_transaction(_accountId, _listOfNotes);
    },
    accountId,
    listOfNotes
  );
};

/**
 *
 * @param {string} senderAccountId
 * @param {string} targetAccountId
 * @param {string} faucetId
 * @param {string} noteType
 * @param {string} amount
 * @param {?string} recallHeight
 * @returns {Promise<NewTransactionResult>}
 */
export const createNewSendTransaction = async (
  senderAccountId,
  targetAccountId,
  faucetId,
  noteType,
  amount,
  recallHeight
) => {
  return await testingPage.evaluate(
    async (
      _senderAccountId,
      _targetAccountId,
      _faucetId,
      _noteType,
      _amount,
      _recallHeight
    ) => {
      if (!window.client) {
        await window.create_client();
      }

      /** @type {WebClient} */
      const client = window.client;
      return client.new_send_transaction(
        _senderAccountId,
        _targetAccountId,
        _faucetId,
        _noteType,
        _amount,
        _recallHeight
      );
    },
    senderAccountId,
    targetAccountId,
    faucetId,
    noteType,
    amount,
    recallHeight
  );
};

/**
 *
 * @param {string} senderAccountId
 * @param {string} offeredAssetFaucetId
 * @param {string} offeredAssetAmount
 * @param {string} requestedAssetFaucetId
 * @param {string} requestedAssetAmount
 * @param {string} noteType
 * @returns {Promise<NewTransactionResult>}
 */
export const createNewSwapTransaction = async (
  senderAccountId,
  offeredAssetFaucetId,
  offeredAssetAmount,
  requestedAssetFaucetId,
  requestedAssetAmount,
  noteType
) => {
  return await testingPage.evaluate(
    async (
      _senderAccountId,
      _offeredAssetFaucetId,
      _offeredAssetAmount,
      _requestedAssetFaucetId,
      _requestedAssetAmount,
      _noteType
    ) => {
      if (!window.client) {
        await window.create_client();
      }

      /** @type {WebClient} */
      const client = window.client;
      return client.new_swap_transaction(
        _senderAccountId,
        _offeredAssetFaucetId,
        _offeredAssetAmount,
        _requestedAssetFaucetId,
        _requestedAssetAmount,
        _noteType
      );
    },
    senderAccountId,
    offeredAssetFaucetId,
    offeredAssetAmount,
    requestedAssetFaucetId,
    requestedAssetAmount,
    noteType
  );
};

/**
 *
 * @returns {Promise<any>}
 */
export const getTransactions = async () => {
  return await testingPage.evaluate(async () => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    const transactions = client.get_transactions();
    return transactions;
  });
};

/**
 *
 * @param {string} [status="All"]
 * @returns {Promise<any>}
 */
export const getInputNotes = async (status = "All") => {
  return await testingPage.evaluate(async (_status) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return client.get_input_notes(_status);
  }, status);
};

/**
 *
 * @param {string} noteId
 * @returns {Promise<any>}
 */
export const getInputNote = async (noteId) => {
  return await testingPage.evaluate(async (_noteId) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return client.get_input_note(_noteId);
  }, noteId);
};

/**
 *
 * @param {string} [status="All"]
 * @returns {Promise<any>}
 */
export const getOutputNotes = async (status = "All") => {
  return await testingPage.evaluate(async (_status) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return client.get_output_notes(_status);
  }, status);
};

/**
 *
 * @param {string} noteId
 * @returns {Promise<any>}
 */
export const getOutputNote = async (noteId) => {
  return await testingPage.evaluate(async (_noteId) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return client.get_output_note(_noteId);
  }, noteId);
};

/**
 *
 * @param {*} notesAsBytes
 * @returns {Promise<any>}
 */
export const importInputNote = async (notesAsBytes) => {
  return await testingPage.evaluate(async (_notesAsBytes) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return client.import_note(_notesAsBytes);
  }, notesAsBytes);
};

/**
 *
 * @param {string} noteId
 * @returns {Promise<Uint8Array>}
 */
export const exportNote = async (noteId) => {
  return await testingPage.evaluate(async (_noteId) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    const note = client.export_note(_noteId);
    return new Uint8Array(note);
  }, noteId);
};

/**
 *
 *
 * @returns {Promise<any>}
 */
export const syncState = async () => {
  return await testingPage.evaluate(async () => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    await client.sync_state();
  });
};

/**
 *
 * @param {string} noteTag
 *
 * @returns {Promise<any>}
 */
export const addTag = async (noteTag) => {
  return await testingPage.evaluate(async (_noteTag) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return await client.add_tag(_noteTag);
  }, noteTag);
};

/**
 *
 * @param {string} accountId
 * @returns {Promise<any>}
 */
export const fetchCacheAccountAuth = async (accountId) => {
  return await testingPage.evaluate(async (_accountId) => {
    if (!window.client) {
      await window.create_client();
    }

    /** @type {WebClient} */
    const client = window.client;
    return await client.fetch_and_cache_account_auth_by_pub_key(_accountId);
  }, accountId);
};

/**
 *
 * @param {string} address
 */
export const isValidAddress = (address) => {
  expect(address.startsWith("0x")).to.be.true;
};
