[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / OutputNote

# Class: OutputNote

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### full()

> `static` **full**(`note`): `OutputNote`

#### Parameters

##### note

[`Note`](Note.md)

#### Returns

`OutputNote`

***

### partial()

> `static` **partial**(`partial_note`): `OutputNote`

#### Parameters

##### partial\_note

[`PartialNote`](PartialNote.md)

#### Returns

`OutputNote`

***

### header()

> `static` **header**(`note_header`): `OutputNote`

#### Parameters

##### note\_header

[`NoteHeader`](NoteHeader.md)

#### Returns

`OutputNote`

***

### assets()

> **assets**(): [`NoteAssets`](NoteAssets.md)

#### Returns

[`NoteAssets`](NoteAssets.md)

***

### id()

> **id**(): [`NoteId`](NoteId.md)

#### Returns

[`NoteId`](NoteId.md)

***

### recipientDigest()

> **recipientDigest**(): [`RpoDigest`](RpoDigest.md)

#### Returns

[`RpoDigest`](RpoDigest.md)

***

### metadata()

> **metadata**(): [`NoteMetadata`](NoteMetadata.md)

#### Returns

[`NoteMetadata`](NoteMetadata.md)

***

### shrink()

> **shrink**(): `OutputNote`

#### Returns

`OutputNote`

***

### intoFull()

> **intoFull**(): [`Note`](Note.md)

#### Returns

[`Note`](Note.md)
