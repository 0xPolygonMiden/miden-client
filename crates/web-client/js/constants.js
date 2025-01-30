export const WorkerAction = Object.freeze({
  INIT: "init",
  CALL_METHOD: "callMethod",
});

export const MethodName = Object.freeze({
  CREATE_CLIENT: "create_client",
  NEW_WALLET: "new_wallet",
  NEW_FAUCET: "new_faucet",
  NEW_TRANSACTION: "new_transaction",
  NEW_MINT_TRANSACTION: "new_mint_transaction",
  NEW_CONSUME_TRANSACTION: "new_consume_transaction",
  NEW_SEND_TRANSACTION: "new_send_transaction",
  SUBMIT_TRANSACTION: "submit_transaction",
  SYNC_STATE: "sync_state",
});
