[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / TransactionStatus

# Class: TransactionStatus

Defined in: miden\_client\_web.d.ts:543

## Methods

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:545

#### Returns

`void`

***

### getBlockNum()

> **getBlockNum**(): `number`

Defined in: miden\_client\_web.d.ts:552

#### Returns

`number`

***

### isCommitted()

> **isCommitted**(): `boolean`

Defined in: miden\_client\_web.d.ts:550

#### Returns

`boolean`

***

### isDiscarded()

> **isDiscarded**(): `boolean`

Defined in: miden\_client\_web.d.ts:551

#### Returns

`boolean`

***

### isPending()

> **isPending**(): `boolean`

Defined in: miden\_client\_web.d.ts:549

#### Returns

`boolean`

***

### committed()

> `static` **committed**(`block_num`): `TransactionStatus`

Defined in: miden\_client\_web.d.ts:547

#### Parameters

##### block\_num

`number`

#### Returns

`TransactionStatus`

***

### discarded()

> `static` **discarded**(): `TransactionStatus`

Defined in: miden\_client\_web.d.ts:548

#### Returns

`TransactionStatus`

***

### pending()

> `static` **pending**(): `TransactionStatus`

Defined in: miden\_client\_web.d.ts:546

#### Returns

`TransactionStatus`
