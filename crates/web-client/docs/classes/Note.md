[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / Note

# Class: Note

Defined in: miden\_client\_web.d.ts:241

## Constructors

### Constructor

> **new Note**(`note_assets`, `note_metadata`, `note_recipient`): `Note`

Defined in: miden\_client\_web.d.ts:243

#### Parameters

##### note\_assets

[`NoteAssets`](NoteAssets.md)

##### note\_metadata

[`NoteMetadata`](NoteMetadata.md)

##### note\_recipient

[`NoteRecipient`](NoteRecipient.md)

#### Returns

`Note`

## Methods

### assets()

> **assets**(): [`NoteAssets`](NoteAssets.md)

Defined in: miden\_client\_web.d.ts:247

#### Returns

[`NoteAssets`](NoteAssets.md)

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:242

#### Returns

`void`

***

### id()

> **id**(): [`NoteId`](NoteId.md)

Defined in: miden\_client\_web.d.ts:244

#### Returns

[`NoteId`](NoteId.md)

***

### metadata()

> **metadata**(): [`NoteMetadata`](NoteMetadata.md)

Defined in: miden\_client\_web.d.ts:245

#### Returns

[`NoteMetadata`](NoteMetadata.md)

***

### recipient()

> **recipient**(): [`NoteRecipient`](NoteRecipient.md)

Defined in: miden\_client\_web.d.ts:246

#### Returns

[`NoteRecipient`](NoteRecipient.md)

***

### createP2IDNote()

> `static` **createP2IDNote**(`sender`, `target`, `assets`, `note_type`, `serial_num`, `aux`): `Note`

Defined in: miden\_client\_web.d.ts:248

#### Parameters

##### sender

[`AccountId`](AccountId.md)

##### target

[`AccountId`](AccountId.md)

##### assets

[`NoteAssets`](NoteAssets.md)

##### note\_type

[`NoteType`](NoteType.md)

##### serial\_num

[`Word`](Word.md)

##### aux

[`Felt`](Felt.md)

#### Returns

`Note`

***

### createP2IDRNote()

> `static` **createP2IDRNote**(`sender`, `target`, `assets`, `note_type`, `serial_num`, `recall_height`, `aux`): `Note`

Defined in: miden\_client\_web.d.ts:249

#### Parameters

##### sender

[`AccountId`](AccountId.md)

##### target

[`AccountId`](AccountId.md)

##### assets

[`NoteAssets`](NoteAssets.md)

##### note\_type

[`NoteType`](NoteType.md)

##### serial\_num

[`Word`](Word.md)

##### recall\_height

`number`

##### aux

[`Felt`](Felt.md)

#### Returns

`Note`
