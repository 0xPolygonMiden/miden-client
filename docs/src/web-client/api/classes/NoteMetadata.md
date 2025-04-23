[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / NoteMetadata

# Class: NoteMetadata

## Constructors

### Constructor

> **new NoteMetadata**(`sender`, `note_type`, `note_tag`, `note_execution_hint`, `aux`?): `NoteMetadata`

#### Parameters

##### sender

[`AccountId`](AccountId.md)

##### note\_type

[`NoteType`](../enumerations/NoteType.md)

##### note\_tag

[`NoteTag`](NoteTag.md)

##### note\_execution\_hint

[`NoteExecutionHint`](NoteExecutionHint.md)

##### aux?

[`Felt`](Felt.md)

#### Returns

`NoteMetadata`

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### noteType()

> **noteType**(): [`NoteType`](../enumerations/NoteType.md)

#### Returns

[`NoteType`](../enumerations/NoteType.md)

***

### sender()

> **sender**(): [`AccountId`](AccountId.md)

#### Returns

[`AccountId`](AccountId.md)

***

### tag()

> **tag**(): [`NoteTag`](NoteTag.md)

#### Returns

[`NoteTag`](NoteTag.md)
