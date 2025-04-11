[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / TransactionStatus

# Class: TransactionStatus

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### pending()

> `static` **pending**(): `TransactionStatus`

#### Returns

`TransactionStatus`

***

### committed()

> `static` **committed**(`block_num`): `TransactionStatus`

#### Parameters

##### block\_num

`number`

#### Returns

`TransactionStatus`

***

### discarded()

> `static` **discarded**(): `TransactionStatus`

#### Returns

`TransactionStatus`

***

### isPending()

> **isPending**(): `boolean`

#### Returns

`boolean`

***

### isCommitted()

> **isCommitted**(): `boolean`

#### Returns

`boolean`

***

### isDiscarded()

> **isDiscarded**(): `boolean`

#### Returns

`boolean`

***

### getBlockNum()

> **getBlockNum**(): `number`

#### Returns

`number`
