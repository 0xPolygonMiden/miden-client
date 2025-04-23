[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / Note

# Class: Note

## Constructors

### Constructor

> **new Note**(`note_assets`, `note_metadata`, `note_recipient`): `Note`

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

#### Returns

[`NoteAssets`](NoteAssets.md)

***

### free()

> **free**(): `void`

#### Returns

`void`

***

### id()

> **id**(): [`NoteId`](NoteId.md)

#### Returns

[`NoteId`](NoteId.md)

***

### metadata()

> **metadata**(): [`NoteMetadata`](NoteMetadata.md)

#### Returns

[`NoteMetadata`](NoteMetadata.md)

***

### recipient()

> **recipient**(): [`NoteRecipient`](NoteRecipient.md)

#### Returns

[`NoteRecipient`](NoteRecipient.md)

***

### createP2IDNote()

> `static` **createP2IDNote**(`sender`, `target`, `assets`, `note_type`, `serial_num`, `aux`): `Note`

#### Parameters

##### sender

[`AccountId`](AccountId.md)

##### target

[`AccountId`](AccountId.md)

##### assets

[`NoteAssets`](NoteAssets.md)

##### note\_type

[`NoteType`](../enumerations/NoteType.md)

##### serial\_num

[`Word`](Word.md)

##### aux

[`Felt`](Felt.md)

#### Returns

`Note`

***

### createP2IDRNote()

> `static` **createP2IDRNote**(`sender`, `target`, `assets`, `note_type`, `serial_num`, `recall_height`, `aux`): `Note`

#### Parameters

##### sender

[`AccountId`](AccountId.md)

##### target

[`AccountId`](AccountId.md)

##### assets

[`NoteAssets`](NoteAssets.md)

##### note\_type

[`NoteType`](../enumerations/NoteType.md)

##### serial\_num

[`Word`](Word.md)

##### recall\_height

`number`

##### aux

[`Felt`](Felt.md)

#### Returns

`Note`
