[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / NoteTag

# Class: NoteTag

Defined in: miden\_client\_web.d.ts:370

## Methods

### executionMode()

> **executionMode**(): [`NoteExecutionMode`](NoteExecutionMode.md)

Defined in: miden\_client\_web.d.ts:377

#### Returns

[`NoteExecutionMode`](NoteExecutionMode.md)

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:372

#### Returns

`void`

***

### isSingleTarget()

> **isSingleTarget**(): `boolean`

Defined in: miden\_client\_web.d.ts:376

#### Returns

`boolean`

***

### forLocalUseCase()

> `static` **forLocalUseCase**(`use_case_id`, `payload`): `NoteTag`

Defined in: miden\_client\_web.d.ts:375

#### Parameters

##### use\_case\_id

`number`

##### payload

`number`

#### Returns

`NoteTag`

***

### forPublicUseCase()

> `static` **forPublicUseCase**(`use_case_id`, `payload`, `execution`): `NoteTag`

Defined in: miden\_client\_web.d.ts:374

#### Parameters

##### use\_case\_id

`number`

##### payload

`number`

##### execution

[`NoteExecutionMode`](NoteExecutionMode.md)

#### Returns

`NoteTag`

***

### fromAccountId()

> `static` **fromAccountId**(`account_id`, `execution`): `NoteTag`

Defined in: miden\_client\_web.d.ts:373

#### Parameters

##### account\_id

[`AccountId`](AccountId.md)

##### execution

[`NoteExecutionMode`](NoteExecutionMode.md)

#### Returns

`NoteTag`
