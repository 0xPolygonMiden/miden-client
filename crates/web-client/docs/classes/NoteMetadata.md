[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / NoteMetadata

# Class: NoteMetadata

Defined in: miden\_client\_web.d.ts:351

## Constructors

### Constructor

> **new NoteMetadata**(`sender`, `note_type`, `note_tag`, `note_execution_hint`, `aux`?): `NoteMetadata`

Defined in: miden\_client\_web.d.ts:353

#### Parameters

##### sender

[`AccountId`](AccountId.md)

##### note\_type

[`NoteType`](NoteType.md)

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

Defined in: miden\_client\_web.d.ts:352

#### Returns

`void`

***

### noteType()

> **noteType**(): [`NoteType`](NoteType.md)

Defined in: miden\_client\_web.d.ts:356

#### Returns

[`NoteType`](NoteType.md)

***

### sender()

> **sender**(): [`AccountId`](AccountId.md)

Defined in: miden\_client\_web.d.ts:354

#### Returns

[`AccountId`](AccountId.md)

***

### tag()

> **tag**(): [`NoteTag`](NoteTag.md)

Defined in: miden\_client\_web.d.ts:355

#### Returns

[`NoteTag`](NoteTag.md)
