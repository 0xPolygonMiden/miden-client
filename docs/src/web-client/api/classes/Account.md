[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / Account

# Class: Account

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### id()

> **id**(): [`AccountId`](AccountId.md)

#### Returns

[`AccountId`](AccountId.md)

***

### commitment()

> **commitment**(): [`RpoDigest`](RpoDigest.md)

#### Returns

[`RpoDigest`](RpoDigest.md)

***

### nonce()

> **nonce**(): [`Felt`](Felt.md)

#### Returns

[`Felt`](Felt.md)

***

### vault()

> **vault**(): [`AssetVault`](AssetVault.md)

#### Returns

[`AssetVault`](AssetVault.md)

***

### storage()

> **storage**(): [`AccountStorage`](AccountStorage.md)

#### Returns

[`AccountStorage`](AccountStorage.md)

***

### code()

> **code**(): [`AccountCode`](AccountCode.md)

#### Returns

[`AccountCode`](AccountCode.md)

***

### isFaucet()

> **isFaucet**(): `boolean`

#### Returns

`boolean`

***

### isRegularAccount()

> **isRegularAccount**(): `boolean`

#### Returns

`boolean`

***

### isUpdatable()

> **isUpdatable**(): `boolean`

#### Returns

`boolean`

***

### isPublic()

> **isPublic**(): `boolean`

#### Returns

`boolean`

***

### isNew()

> **isNew**(): `boolean`

#### Returns

`boolean`

***

### serialize()

> **serialize**(): `Uint8Array`

#### Returns

`Uint8Array`

***

### deserialize()

> `static` **deserialize**(`bytes`): `Account`

#### Parameters

##### bytes

`Uint8Array`

#### Returns

`Account`
