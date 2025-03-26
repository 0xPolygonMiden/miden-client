[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / IntoUnderlyingByteSource

# Class: IntoUnderlyingByteSource

Defined in: miden\_client\_web.d.ts:201

## Properties

### autoAllocateChunkSize

> `readonly` **autoAllocateChunkSize**: `number`

Defined in: miden\_client\_web.d.ts:208

***

### type

> `readonly` **type**: `"bytes"`

Defined in: miden\_client\_web.d.ts:207

## Methods

### cancel()

> **cancel**(): `void`

Defined in: miden\_client\_web.d.ts:206

#### Returns

`void`

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:203

#### Returns

`void`

***

### pull()

> **pull**(`controller`): `Promise`\<`any`\>

Defined in: miden\_client\_web.d.ts:205

#### Parameters

##### controller

`ReadableByteStreamController`

#### Returns

`Promise`\<`any`\>

***

### start()

> **start**(`controller`): `void`

Defined in: miden\_client\_web.d.ts:204

#### Parameters

##### controller

`ReadableByteStreamController`

#### Returns

`void`
