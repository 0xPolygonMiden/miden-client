[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / NoteExecutionHint

# Class: NoteExecutionHint

Defined in: miden\_client\_web.d.ts:293

## Methods

### canBeConsumed()

> **canBeConsumed**(`block_num`): `boolean`

Defined in: miden\_client\_web.d.ts:301

#### Parameters

##### block\_num

`number`

#### Returns

`boolean`

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:295

#### Returns

`void`

***

### afterBlock()

> `static` **afterBlock**(`block_num`): `NoteExecutionHint`

Defined in: miden\_client\_web.d.ts:298

#### Parameters

##### block\_num

`number`

#### Returns

`NoteExecutionHint`

***

### always()

> `static` **always**(): `NoteExecutionHint`

Defined in: miden\_client\_web.d.ts:297

#### Returns

`NoteExecutionHint`

***

### fromParts()

> `static` **fromParts**(`tag`, `payload`): `NoteExecutionHint`

Defined in: miden\_client\_web.d.ts:300

#### Parameters

##### tag

`number`

##### payload

`number`

#### Returns

`NoteExecutionHint`

***

### none()

> `static` **none**(): `NoteExecutionHint`

Defined in: miden\_client\_web.d.ts:296

#### Returns

`NoteExecutionHint`

***

### onBlockSlot()

> `static` **onBlockSlot**(`epoch_len`, `slot_len`, `slot_offset`): `NoteExecutionHint`

Defined in: miden\_client\_web.d.ts:299

#### Parameters

##### epoch\_len

`number`

##### slot\_len

`number`

##### slot\_offset

`number`

#### Returns

`NoteExecutionHint`
