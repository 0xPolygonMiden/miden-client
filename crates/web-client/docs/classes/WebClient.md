[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / WebClient

# Class: WebClient

Defined in: miden\_client\_web.d.ts:554

## Constructors

### Constructor

> **new WebClient**(): `WebClient`

Defined in: miden\_client\_web.d.ts:591

#### Returns

`WebClient`

## Methods

### addTag()

> **addTag**(`tag`): `Promise`\<`void`\>

Defined in: miden\_client\_web.d.ts:585

#### Parameters

##### tag

`string`

#### Returns

`Promise`\<`void`\>

***

### compileNoteScript()

> **compileNoteScript**(`script`): [`NoteScript`](NoteScript.md)

Defined in: miden\_client\_web.d.ts:582

#### Parameters

##### script

`string`

#### Returns

[`NoteScript`](NoteScript.md)

***

### compileTxScript()

> **compileTxScript**(`script`): [`TransactionScript`](TransactionScript.md)

Defined in: miden\_client\_web.d.ts:589

#### Parameters

##### script

`string`

#### Returns

[`TransactionScript`](TransactionScript.md)

***

### createClient()

> **createClient**(`node_url`?, `seed`?): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:592

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

Defined in: miden\_client\_web.d.ts:559

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

Defined in: miden\_client\_web.d.ts:565

Retrieves the entire underlying web store and returns it as a JsValue

Meant to be used in conjunction with the force_import_store method

#### Returns

`Promise`\<`any`\>

***

### fetchAndCacheAccountAuthByAccountId()

> **fetchAndCacheAccountAuthByAccountId**(`account_id`): `Promise`\<`string`\>

Defined in: miden\_client\_web.d.ts:558

#### Parameters

##### account\_id

[`AccountId`](AccountId.md)

#### Returns

`Promise`\<`string`\>

***

### forceImportStore()

> **forceImportStore**(`store_dump`): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:569

#### Parameters

##### store\_dump

`any`

#### Returns

`Promise`\<`any`\>

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:555

#### Returns

`void`

***

### getAccount()

> **getAccount**(`account_id`): `Promise`\<[`Account`](Account.md)\>

Defined in: miden\_client\_web.d.ts:557

#### Parameters

##### account\_id

[`AccountId`](AccountId.md)

#### Returns

`Promise`\<[`Account`](Account.md)\>

***

### getAccounts()

> **getAccounts**(): `Promise`\<[`AccountHeader`](AccountHeader.md)[]\>

Defined in: miden\_client\_web.d.ts:556

#### Returns

`Promise`\<[`AccountHeader`](AccountHeader.md)[]\>

***

### getConsumableNotes()

> **getConsumableNotes**(`account_id`?): `Promise`\<[`ConsumableNoteRecord`](ConsumableNoteRecord.md)[]\>

Defined in: miden\_client\_web.d.ts:583

#### Parameters

##### account\_id?

[`AccountId`](AccountId.md)

#### Returns

`Promise`\<[`ConsumableNoteRecord`](ConsumableNoteRecord.md)[]\>

***

### getInputNote()

> **getInputNote**(`note_id`): `Promise`\<[`InputNoteRecord`](InputNoteRecord.md)\>

Defined in: miden\_client\_web.d.ts:579

#### Parameters

##### note\_id

`string`

#### Returns

`Promise`\<[`InputNoteRecord`](InputNoteRecord.md)\>

***

### getInputNotes()

> **getInputNotes**(`filter`): `Promise`\<[`InputNoteRecord`](InputNoteRecord.md)[]\>

Defined in: miden\_client\_web.d.ts:578

#### Parameters

##### filter

[`NoteFilter`](NoteFilter.md)

#### Returns

`Promise`\<[`InputNoteRecord`](InputNoteRecord.md)[]\>

***

### getOutputNote()

> **getOutputNote**(`note_id`): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:581

#### Parameters

##### note\_id

`string`

#### Returns

`Promise`\<`any`\>

***

### getOutputNotes()

> **getOutputNotes**(`filter`): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:580

#### Parameters

##### filter

[`NoteFilter`](NoteFilter.md)

#### Returns

`Promise`\<`any`\>

***

### getTransactions()

> **getTransactions**(`transaction_filter`): `Promise`\<[`TransactionRecord`](TransactionRecord.md)[]\>

Defined in: miden\_client\_web.d.ts:588

#### Parameters

##### transaction\_filter

[`TransactionFilter`](TransactionFilter.md)

#### Returns

`Promise`\<[`TransactionRecord`](TransactionRecord.md)[]\>

***

### importAccount()

> **importAccount**(`account_bytes`): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:566

#### Parameters

##### account\_bytes

`any`

#### Returns

`Promise`\<`any`\>

***

### importNote()

> **importNote**(`note_bytes`): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:568

#### Parameters

##### note\_bytes

`any`

#### Returns

`Promise`\<`any`\>

***

### importPublicAccountFromSeed()

> **importPublicAccountFromSeed**(`init_seed`, `mutable`): `Promise`\<[`Account`](Account.md)\>

Defined in: miden\_client\_web.d.ts:567

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

Defined in: miden\_client\_web.d.ts:587

#### Returns

`Promise`\<`any`\>

***

### newConsumeTransactionRequest()

> **newConsumeTransactionRequest**(`list_of_note_ids`): [`TransactionRequest`](TransactionRequest.md)

Defined in: miden\_client\_web.d.ts:576

#### Parameters

##### list\_of\_note\_ids

`string`[]

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### newFaucet()

> **newFaucet**(`storage_mode`, `non_fungible`, `token_symbol`, `decimals`, `max_supply`): `Promise`\<[`Account`](Account.md)\>

Defined in: miden\_client\_web.d.ts:571

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

Defined in: miden\_client\_web.d.ts:574

#### Parameters

##### target\_account\_id

[`AccountId`](AccountId.md)

##### faucet\_id

[`AccountId`](AccountId.md)

##### note\_type

[`NoteType`](NoteType.md)

##### amount

`bigint`

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### newSendTransactionRequest()

> **newSendTransactionRequest**(`sender_account_id`, `target_account_id`, `faucet_id`, `note_type`, `amount`, `recall_height`?): [`TransactionRequest`](TransactionRequest.md)

Defined in: miden\_client\_web.d.ts:575

#### Parameters

##### sender\_account\_id

[`AccountId`](AccountId.md)

##### target\_account\_id

[`AccountId`](AccountId.md)

##### faucet\_id

[`AccountId`](AccountId.md)

##### note\_type

[`NoteType`](NoteType.md)

##### amount

`bigint`

##### recall\_height?

`number`

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### newSwapTransaction()

> **newSwapTransaction**(`sender_account_id`, `offered_asset_faucet_id`, `offered_asset_amount`, `requested_asset_faucet_id`, `requested_asset_amount`, `note_type`): `Promise`\<[`NewSwapTransactionResult`](NewSwapTransactionResult.md)\>

Defined in: miden\_client\_web.d.ts:577

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

[`NoteType`](NoteType.md)

#### Returns

`Promise`\<[`NewSwapTransactionResult`](NewSwapTransactionResult.md)\>

***

### newTransaction()

> **newTransaction**(`account_id`, `transaction_request`): `Promise`\<[`TransactionResult`](TransactionResult.md)\>

Defined in: miden\_client\_web.d.ts:572

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

Defined in: miden\_client\_web.d.ts:570

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

Defined in: miden\_client\_web.d.ts:586

#### Parameters

##### tag

`string`

#### Returns

`Promise`\<`void`\>

***

### submitTransaction()

> **submitTransaction**(`transaction_result`, `prover`?): `Promise`\<`void`\>

Defined in: miden\_client\_web.d.ts:573

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

Defined in: miden\_client\_web.d.ts:584

#### Returns

`Promise`\<[`SyncSummary`](SyncSummary.md)\>

***

### testingApplyTransaction()

> **testingApplyTransaction**(`tx_result`): `Promise`\<`void`\>

Defined in: miden\_client\_web.d.ts:590

#### Parameters

##### tx\_result

[`TransactionResult`](TransactionResult.md)

#### Returns

`Promise`\<`void`\>
