[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / WebClient

# Class: WebClient

## Constructors

### Constructor

> **new WebClient**(): `WebClient`

#### Returns

`WebClient`

## Methods

### addTag()

> **addTag**(`tag`): `Promise`\<`void`\>

#### Parameters

##### tag

`string`

#### Returns

`Promise`\<`void`\>

***

### compileNoteScript()

> **compileNoteScript**(`script`): [`NoteScript`](NoteScript.md)

#### Parameters

##### script

`string`

#### Returns

[`NoteScript`](NoteScript.md)

***

### compileTxScript()

> **compileTxScript**(`script`): [`TransactionScript`](TransactionScript.md)

#### Parameters

##### script

`string`

#### Returns

[`TransactionScript`](TransactionScript.md)

***

### createClient()

> **createClient**(`node_url`?, `seed`?): `Promise`\<`any`\>

#### Parameters

##### node\_url?

`string`

##### seed?

`Uint8Array`

#### Returns

`Promise`\<`any`\>

***

### exportNote()

> **exportNote**(`note_id`, `export_type`): `Promise`\<`any`\>

#### Parameters

##### note\_id

`string`

##### export\_type

`string`

#### Returns

`Promise`\<`any`\>

***

### exportStore()

> **exportStore**(): `Promise`\<`any`\>

Retrieves the entire underlying web store and returns it as a JsValue

Meant to be used in conjunction with the force_import_store method

#### Returns

`Promise`\<`any`\>

***

### forceImportStore()

> **forceImportStore**(`store_dump`): `Promise`\<`any`\>

#### Parameters

##### store\_dump

`any`

#### Returns

`Promise`\<`any`\>

***

### free()

> **free**(): `void`

#### Returns

`void`

***

### getAccount()

> **getAccount**(`account_id`): `Promise`\<[`Account`](Account.md)\>

#### Parameters

##### account\_id

[`AccountId`](AccountId.md)

#### Returns

`Promise`\<[`Account`](Account.md)\>

***

### getAccounts()

> **getAccounts**(): `Promise`\<[`AccountHeader`](AccountHeader.md)[]\>

#### Returns

`Promise`\<[`AccountHeader`](AccountHeader.md)[]\>

***

### getConsumableNotes()

> **getConsumableNotes**(`account_id`?): `Promise`\<[`ConsumableNoteRecord`](ConsumableNoteRecord.md)[]\>

#### Parameters

##### account\_id?

[`AccountId`](AccountId.md)

#### Returns

`Promise`\<[`ConsumableNoteRecord`](ConsumableNoteRecord.md)[]\>

***

### getInputNote()

> **getInputNote**(`note_id`): `Promise`\<[`InputNoteRecord`](InputNoteRecord.md)\>

#### Parameters

##### note\_id

`string`

#### Returns

`Promise`\<[`InputNoteRecord`](InputNoteRecord.md)\>

***

### getInputNotes()

> **getInputNotes**(`filter`): `Promise`\<[`InputNoteRecord`](InputNoteRecord.md)[]\>

#### Parameters

##### filter

[`NoteFilter`](NoteFilter.md)

#### Returns

`Promise`\<[`InputNoteRecord`](InputNoteRecord.md)[]\>

***

### getOutputNote()

> **getOutputNote**(`note_id`): `Promise`\<`any`\>

#### Parameters

##### note\_id

`string`

#### Returns

`Promise`\<`any`\>

***

### getOutputNotes()

> **getOutputNotes**(`filter`): `Promise`\<`any`\>

#### Parameters

##### filter

[`NoteFilter`](NoteFilter.md)

#### Returns

`Promise`\<`any`\>

***

### getTransactions()

> **getTransactions**(`transaction_filter`): `Promise`\<[`TransactionRecord`](TransactionRecord.md)[]\>

#### Parameters

##### transaction\_filter

[`TransactionFilter`](TransactionFilter.md)

#### Returns

`Promise`\<[`TransactionRecord`](TransactionRecord.md)[]\>

***

### importAccount()

> **importAccount**(`account_bytes`): `Promise`\<`any`\>

#### Parameters

##### account\_bytes

`any`

#### Returns

`Promise`\<`any`\>

***

### importNote()

> **importNote**(`note_bytes`): `Promise`\<`any`\>

#### Parameters

##### note\_bytes

`any`

#### Returns

`Promise`\<`any`\>

***

### importPublicAccountFromSeed()

> **importPublicAccountFromSeed**(`init_seed`, `mutable`): `Promise`\<[`Account`](Account.md)\>

#### Parameters

##### init\_seed

`Uint8Array`

##### mutable

`boolean`

#### Returns

`Promise`\<[`Account`](Account.md)\>

***

### listTags()

> **listTags**(): `Promise`\<`any`\>

#### Returns

`Promise`\<`any`\>

***

### newConsumeTransactionRequest()

> **newConsumeTransactionRequest**(`list_of_note_ids`): [`TransactionRequest`](TransactionRequest.md)

#### Parameters

##### list\_of\_note\_ids

`string`[]

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### newFaucet()

> **newFaucet**(`storage_mode`, `non_fungible`, `token_symbol`, `decimals`, `max_supply`): `Promise`\<[`Account`](Account.md)\>

#### Parameters

##### storage\_mode

[`AccountStorageMode`](AccountStorageMode.md)

##### non\_fungible

`boolean`

##### token\_symbol

`string`

##### decimals

`number`

##### max\_supply

`bigint`

#### Returns

`Promise`\<[`Account`](Account.md)\>

***

### newMintTransactionRequest()

> **newMintTransactionRequest**(`target_account_id`, `faucet_id`, `note_type`, `amount`): [`TransactionRequest`](TransactionRequest.md)

#### Parameters

##### target\_account\_id

[`AccountId`](AccountId.md)

##### faucet\_id

[`AccountId`](AccountId.md)

##### note\_type

[`NoteType`](../enumerations/NoteType.md)

##### amount

`bigint`

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### newSendTransactionRequest()

> **newSendTransactionRequest**(`sender_account_id`, `target_account_id`, `faucet_id`, `note_type`, `amount`, `recall_height`?): [`TransactionRequest`](TransactionRequest.md)

#### Parameters

##### sender\_account\_id

[`AccountId`](AccountId.md)

##### target\_account\_id

[`AccountId`](AccountId.md)

##### faucet\_id

[`AccountId`](AccountId.md)

##### note\_type

[`NoteType`](../enumerations/NoteType.md)

##### amount

`bigint`

##### recall\_height?

`number`

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### newSwapTransaction()

> **newSwapTransaction**(`sender_account_id`, `offered_asset_faucet_id`, `offered_asset_amount`, `requested_asset_faucet_id`, `requested_asset_amount`, `note_type`): `Promise`\<[`NewSwapTransactionResult`](NewSwapTransactionResult.md)\>

#### Parameters

##### sender\_account\_id

`string`

##### offered\_asset\_faucet\_id

`string`

##### offered\_asset\_amount

`string`

##### requested\_asset\_faucet\_id

`string`

##### requested\_asset\_amount

`string`

##### note\_type

[`NoteType`](../enumerations/NoteType.md)

#### Returns

`Promise`\<[`NewSwapTransactionResult`](NewSwapTransactionResult.md)\>

***

### newTransaction()

> **newTransaction**(`account_id`, `transaction_request`): `Promise`\<[`TransactionResult`](TransactionResult.md)\>

#### Parameters

##### account\_id

[`AccountId`](AccountId.md)

##### transaction\_request

[`TransactionRequest`](TransactionRequest.md)

#### Returns

`Promise`\<[`TransactionResult`](TransactionResult.md)\>

***

### newWallet()

> **newWallet**(`storage_mode`, `mutable`, `init_seed`?): `Promise`\<[`Account`](Account.md)\>

#### Parameters

##### storage\_mode

[`AccountStorageMode`](AccountStorageMode.md)

##### mutable

`boolean`

##### init\_seed?

`Uint8Array`

#### Returns

`Promise`\<[`Account`](Account.md)\>

***

### removeTag()

> **removeTag**(`tag`): `Promise`\<`void`\>

#### Parameters

##### tag

`string`

#### Returns

`Promise`\<`void`\>

***

### submitTransaction()

> **submitTransaction**(`transaction_result`, `prover`?): `Promise`\<`void`\>

#### Parameters

##### transaction\_result

[`TransactionResult`](TransactionResult.md)

##### prover?

[`TransactionProver`](TransactionProver.md)

#### Returns

`Promise`\<`void`\>

***

### syncState()

> **syncState**(): `Promise`\<[`SyncSummary`](SyncSummary.md)\>

#### Returns

`Promise`\<[`SyncSummary`](SyncSummary.md)\>

***

### testingApplyTransaction()

> **testingApplyTransaction**(`tx_result`): `Promise`\<`void`\>

#### Parameters

##### tx\_result

[`TransactionResult`](TransactionResult.md)

#### Returns

`Promise`\<`void`\>
