[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / SyncSummary

# Class: SyncSummary

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### blockNum()

> **blockNum**(): `number`

#### Returns

`number`

***

### committedNotes()

> **committedNotes**(): [`NoteId`](NoteId.md)[]

#### Returns

[`NoteId`](NoteId.md)[]

***

### consumedNotes()

> **consumedNotes**(): [`NoteId`](NoteId.md)[]

#### Returns

[`NoteId`](NoteId.md)[]

***

### updatedAccounts()

> **updatedAccounts**(): [`AccountId`](AccountId.md)[]

#### Returns

[`AccountId`](AccountId.md)[]

***

### committedTransactions()

> **committedTransactions**(): [`TransactionId`](TransactionId.md)[]

#### Returns

[`TransactionId`](TransactionId.md)[]

***

### serialize()

> **serialize**(): `Uint8Array`

#### Returns

`Uint8Array`

***

### deserialize()

> `static` **deserialize**(`bytes`): `SyncSummary`

#### Parameters

##### bytes

`Uint8Array`

#### Returns

`SyncSummary`
