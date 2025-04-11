[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / NoteExecutionHint

# Class: NoteExecutionHint

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### none()

> `static` **none**(): `NoteExecutionHint`

#### Returns

`NoteExecutionHint`

***

### always()

> `static` **always**(): `NoteExecutionHint`

#### Returns

`NoteExecutionHint`

***

### afterBlock()

> `static` **afterBlock**(`block_num`): `NoteExecutionHint`

#### Parameters

##### block\_num

`number`

#### Returns

`NoteExecutionHint`

***

### onBlockSlot()

> `static` **onBlockSlot**(`epoch_len`, `slot_len`, `slot_offset`): `NoteExecutionHint`

#### Parameters

##### epoch\_len

`number`

##### slot\_len

`number`

##### slot\_offset

`number`

#### Returns

`NoteExecutionHint`

***

### fromParts()

> `static` **fromParts**(`tag`, `payload`): `NoteExecutionHint`

#### Parameters

##### tag

`number`

##### payload

`number`

#### Returns

`NoteExecutionHint`

***

### canBeConsumed()

> **canBeConsumed**(`block_num`): `boolean`

#### Parameters

##### block\_num

`number`

#### Returns

`boolean`
