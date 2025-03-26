[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / Account

# Class: Account

Defined in: miden\_client\_web.d.ts:31

## Methods

### code()

> **code**(): [`AccountCode`](AccountCode.md)

Defined in: miden\_client\_web.d.ts:39

#### Returns

[`AccountCode`](AccountCode.md)

***

### commitment()

> **commitment**(): [`RpoDigest`](RpoDigest.md)

Defined in: miden\_client\_web.d.ts:35

#### Returns

[`RpoDigest`](RpoDigest.md)

***

### free()

> **free**(): `void`

Defined in: miden\_client\_web.d.ts:33

#### Returns

`void`

***

### id()

> **id**(): [`AccountId`](AccountId.md)

Defined in: miden\_client\_web.d.ts:34

#### Returns

[`AccountId`](AccountId.md)

***

### isFaucet()

> **isFaucet**(): `boolean`

Defined in: miden\_client\_web.d.ts:40

#### Returns

`boolean`

***

### isNew()

> **isNew**(): `boolean`

Defined in: miden\_client\_web.d.ts:44

#### Returns

`boolean`

***

### isPublic()

> **isPublic**(): `boolean`

Defined in: miden\_client\_web.d.ts:43

#### Returns

`boolean`

***

### isRegularAccount()

> **isRegularAccount**(): `boolean`

Defined in: miden\_client\_web.d.ts:41

#### Returns

`boolean`

***

### isUpdatable()

> **isUpdatable**(): `boolean`

Defined in: miden\_client\_web.d.ts:42

#### Returns

`boolean`

***

### nonce()

> **nonce**(): [`Felt`](Felt.md)

Defined in: miden\_client\_web.d.ts:36

#### Returns

[`Felt`](Felt.md)

***

### serialize()

> **serialize**(): `Uint8Array`

Defined in: miden\_client\_web.d.ts:45

#### Returns

`Uint8Array`

***

### storage()

> **storage**(): [`AccountStorage`](AccountStorage.md)

Defined in: miden\_client\_web.d.ts:38

#### Returns

[`AccountStorage`](AccountStorage.md)

***

### vault()

> **vault**(): [`AssetVault`](AssetVault.md)

Defined in: miden\_client\_web.d.ts:37

#### Returns

[`AssetVault`](AssetVault.md)

***

### deserialize()

> `static` **deserialize**(`bytes`): `Account`

Defined in: miden\_client\_web.d.ts:46

#### Parameters

##### bytes

`Uint8Array`

#### Returns

`Account`
