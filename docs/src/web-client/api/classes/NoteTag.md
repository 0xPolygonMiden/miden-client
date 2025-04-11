[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / NoteTag

# Class: NoteTag

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### fromAccountId()

> `static` **fromAccountId**(`account_id`, `execution`): `NoteTag`

#### Parameters

##### account\_id

[`AccountId`](AccountId.md)

##### execution

[`NoteExecutionMode`](NoteExecutionMode.md)

#### Returns

`NoteTag`

***

### forPublicUseCase()

> `static` **forPublicUseCase**(`use_case_id`, `payload`, `execution`): `NoteTag`

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

### forLocalUseCase()

> `static` **forLocalUseCase**(`use_case_id`, `payload`): `NoteTag`

#### Parameters

##### use\_case\_id

`number`

##### payload

`number`

#### Returns

`NoteTag`

***

### isSingleTarget()

> **isSingleTarget**(): `boolean`

#### Returns

`boolean`

***

### executionMode()

> **executionMode**(): [`NoteExecutionMode`](NoteExecutionMode.md)

#### Returns

[`NoteExecutionMode`](NoteExecutionMode.md)
