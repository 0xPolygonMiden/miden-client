/* tslint:disable */
/* eslint-disable */
/**
*/
export class IntoUnderlyingByteSource {
  free(): void;
/**
* @param {ReadableByteStreamController} controller
*/
  start(controller: ReadableByteStreamController): void;
/**
* @param {ReadableByteStreamController} controller
* @returns {Promise<any>}
*/
  pull(controller: ReadableByteStreamController): Promise<any>;
/**
*/
  cancel(): void;
/**
*/
  readonly autoAllocateChunkSize: number;
/**
*/
  readonly type: string;
}
/**
*/
export class IntoUnderlyingSink {
  free(): void;
/**
* @param {any} chunk
* @returns {Promise<any>}
*/
  write(chunk: any): Promise<any>;
/**
* @returns {Promise<any>}
*/
  close(): Promise<any>;
/**
* @param {any} reason
* @returns {Promise<any>}
*/
  abort(reason: any): Promise<any>;
}
/**
*/
export class IntoUnderlyingSource {
  free(): void;
/**
* @param {ReadableStreamDefaultController} controller
* @returns {Promise<any>}
*/
  pull(controller: ReadableStreamDefaultController): Promise<any>;
/**
*/
  cancel(): void;
}
/**
*/
export class NewSwapTransactionResult {
  free(): void;
/**
* @param {string} transaction_id
* @param {(string)[]} expected_output_note_ids
* @param {(string)[]} expected_partial_note_ids
* @param {string | undefined} [payback_note_tag]
* @returns {NewSwapTransactionResult}
*/
  static new(transaction_id: string, expected_output_note_ids: (string)[], expected_partial_note_ids: (string)[], payback_note_tag?: string): NewSwapTransactionResult;
/**
* @param {string} payback_note_tag
*/
  set_note_tag(payback_note_tag: string): void;
/**
*/
  readonly expected_output_note_ids: any;
/**
*/
  readonly expected_partial_note_ids: any;
/**
*/
  readonly payback_note_tag: string;
/**
*/
  readonly transaction_id: string;
}
/**
*/
export class NewTransactionResult {
  free(): void;
/**
* @param {string} transaction_id
* @param {(string)[]} created_note_ids
* @returns {NewTransactionResult}
*/
  static new(transaction_id: string, created_note_ids: (string)[]): NewTransactionResult;
/**
*/
  readonly created_note_ids: any;
/**
*/
  readonly transaction_id: string;
}
/**
*/
export class SerializedAccountStub {
  free(): void;
/**
* @param {string} id
* @param {string} nonce
* @param {string} vault_root
* @param {string} storage_root
* @param {string} code_root
* @returns {SerializedAccountStub}
*/
  static new(id: string, nonce: string, vault_root: string, storage_root: string, code_root: string): SerializedAccountStub;
/**
*/
  readonly code_root: string;
/**
*/
  readonly id: string;
/**
*/
  readonly nonce: string;
/**
*/
  readonly storage_root: string;
/**
*/
  readonly vault_root: string;
}
/**
*/
export class WebClient {
  free(): void;
/**
* @returns {Promise<any>}
*/
  get_accounts(): Promise<any>;
/**
* @param {string} account_id
* @returns {Promise<any>}
*/
  get_account(account_id: string): Promise<any>;
/**
* @param {any} pub_key_bytes
* @returns {any}
*/
  get_account_auth_by_pub_key(pub_key_bytes: any): any;
/**
* @param {string} account_id
* @returns {Promise<any>}
*/
  fetch_and_cache_account_auth_by_pub_key(account_id: string): Promise<any>;
/**
* @param {string} note_id
* @returns {Promise<any>}
*/
  export_note(note_id: string): Promise<any>;
/**
* @param {any} account_bytes
* @returns {Promise<any>}
*/
  import_account(account_bytes: any): Promise<any>;
/**
* @param {any} note_bytes
* @param {boolean} verify
* @returns {Promise<any>}
*/
  import_note(note_bytes: any, verify: boolean): Promise<any>;
/**
* @param {string} storage_type
* @param {boolean} mutable
* @returns {Promise<any>}
*/
  new_wallet(storage_type: string, mutable: boolean): Promise<any>;
/**
* @param {string} storage_type
* @param {boolean} non_fungible
* @param {string} token_symbol
* @param {string} decimals
* @param {string} max_supply
* @returns {Promise<any>}
*/
  new_faucet(storage_type: string, non_fungible: boolean, token_symbol: string, decimals: string, max_supply: string): Promise<any>;
/**
* @param {string} target_account_id
* @param {string} faucet_id
* @param {string} note_type
* @param {string} amount
* @returns {Promise<NewTransactionResult>}
*/
  new_mint_transaction(target_account_id: string, faucet_id: string, note_type: string, amount: string): Promise<NewTransactionResult>;
/**
* @param {string} sender_account_id
* @param {string} target_account_id
* @param {string} faucet_id
* @param {string} note_type
* @param {string} amount
* @param {string | undefined} [recall_height]
* @returns {Promise<NewTransactionResult>}
*/
  new_send_transaction(sender_account_id: string, target_account_id: string, faucet_id: string, note_type: string, amount: string, recall_height?: string): Promise<NewTransactionResult>;
/**
* @param {string} account_id
* @param {(string)[]} list_of_notes
* @returns {Promise<NewTransactionResult>}
*/
  new_consume_transaction(account_id: string, list_of_notes: (string)[]): Promise<NewTransactionResult>;
/**
* @param {string} sender_account_id
* @param {string} offered_asset_faucet_id
* @param {string} offered_asset_amount
* @param {string} requested_asset_faucet_id
* @param {string} requested_asset_amount
* @param {string} note_type
* @returns {Promise<NewSwapTransactionResult>}
*/
  new_swap_transaction(sender_account_id: string, offered_asset_faucet_id: string, offered_asset_amount: string, requested_asset_faucet_id: string, requested_asset_amount: string, note_type: string): Promise<NewSwapTransactionResult>;
/**
* @param {any} filter
* @returns {Promise<any>}
*/
  get_input_notes(filter: any): Promise<any>;
/**
* @param {string} note_id
* @returns {Promise<any>}
*/
  get_input_note(note_id: string): Promise<any>;
/**
* @param {any} filter
* @returns {Promise<any>}
*/
  get_output_notes(filter: any): Promise<any>;
/**
* @param {string} note_id
* @returns {Promise<any>}
*/
  get_output_note(note_id: string): Promise<any>;
/**
* @returns {Promise<any>}
*/
  sync_state(): Promise<any>;
/**
* @param {string} tag
* @returns {Promise<any>}
*/
  add_tag(tag: string): Promise<any>;
/**
* @returns {Promise<any>}
*/
  get_transactions(): Promise<any>;
/**
*/
  constructor();
/**
* @param {string | undefined} [node_url]
* @returns {Promise<any>}
*/
  create_client(node_url?: string): Promise<any>;
}