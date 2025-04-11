[**@demox-labs/miden-sdk**](../README.md)

***

[@demox-labs/miden-sdk](../README.md) / MerklePath

# Class: MerklePath

## Methods

### free()

> **free**(): `void`

#### Returns

`void`

***

### depth()

> **depth**(): `number`

#### Returns

`number`

***

### nodes()

> **nodes**(): [`RpoDigest`](RpoDigest.md)[]

#### Returns

[`RpoDigest`](RpoDigest.md)[]

***

### computeRoot()

> **computeRoot**(`index`, `node`): [`RpoDigest`](RpoDigest.md)

#### Parameters

##### index

`bigint`

##### node

[`RpoDigest`](RpoDigest.md)

#### Returns

[`RpoDigest`](RpoDigest.md)

***

### verify()

> **verify**(`index`, `node`, `root`): `boolean`

#### Parameters

##### index

`bigint`

##### node

[`RpoDigest`](RpoDigest.md)

##### root

[`RpoDigest`](RpoDigest.md)

#### Returns

`boolean`
