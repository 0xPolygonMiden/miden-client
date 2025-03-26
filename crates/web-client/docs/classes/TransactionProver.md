[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / TransactionProver

# Class: TransactionProver

Defined in: miden\_client\_web.d.ts:476

## Methods

### endpoint()

> **endpoint**(): `string`

Defined in: miden\_client\_web.d.ts:483

#### Returns

`string`

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:478

#### Returns

`void`

***

### serialize()

> **serialize**(): `string`

Defined in: miden\_client\_web.d.ts:481

#### Returns

`string`

***

### deserialize()

> `static` **deserialize**(`prover_type`, `endpoint`?): `TransactionProver`

Defined in: miden\_client\_web.d.ts:482

#### Parameters

##### prover\_type

`string`

##### endpoint?

`string`

#### Returns

`TransactionProver`

***

### newLocalProver()

> `static` **newLocalProver**(): `TransactionProver`

Defined in: miden\_client\_web.d.ts:479

#### Returns

`TransactionProver`

***

### newRemoteProver()

> `static` **newRemoteProver**(`endpoint`): `TransactionProver`

Defined in: miden\_client\_web.d.ts:480

#### Parameters

##### endpoint

`string`

#### Returns

`TransactionProver`
