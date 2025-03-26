[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / TransactionRequestBuilder

# Class: TransactionRequestBuilder

Defined in: miden\_client\_web.d.ts:503

## Constructors

### Constructor

> **new TransactionRequestBuilder**(): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:505

#### Returns

`TransactionRequestBuilder`

## Methods

### build()

> **build**(): [`TransactionRequest`](TransactionRequest.md)

Defined in: miden\_client\_web.d.ts:513

#### Returns

[`TransactionRequest`](TransactionRequest.md)

***

### extendAdviceMap()

> **extendAdviceMap**(`advice_map`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:512

#### Parameters

##### advice\_map

[`AdviceMap`](AdviceMap.md)

#### Returns

`TransactionRequestBuilder`

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:504

#### Returns

`void`

***

### withAuthenticatedInputNotes()

> **withAuthenticatedInputNotes**(`notes`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:507

#### Parameters

##### notes

[`NoteIdAndArgsArray`](NoteIdAndArgsArray.md)

#### Returns

`TransactionRequestBuilder`

***

### withCustomScript()

> **withCustomScript**(`script`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:509

#### Parameters

##### script

[`TransactionScript`](TransactionScript.md)

#### Returns

`TransactionRequestBuilder`

***

### withExpectedFutureNotes()

> **withExpectedFutureNotes**(`note_details_and_tag`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:511

#### Parameters

##### note\_details\_and\_tag

[`NoteDetailsAndTagArray`](NoteDetailsAndTagArray.md)

#### Returns

`TransactionRequestBuilder`

***

### withExpectedOutputNotes()

> **withExpectedOutputNotes**(`notes`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:510

#### Parameters

##### notes

[`NotesArray`](NotesArray.md)

#### Returns

`TransactionRequestBuilder`

***

### withOwnOutputNotes()

> **withOwnOutputNotes**(`notes`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:508

#### Parameters

##### notes

[`OutputNotesArray`](OutputNotesArray.md)

#### Returns

`TransactionRequestBuilder`

***

### withUnauthenticatedInputNotes()

> **withUnauthenticatedInputNotes**(`notes`): `TransactionRequestBuilder`

Defined in: miden\_client\_web.d.ts:506

#### Parameters

##### notes

[`NoteAndArgsArray`](NoteAndArgsArray.md)

#### Returns

`TransactionRequestBuilder`
