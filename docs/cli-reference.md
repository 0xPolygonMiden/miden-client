---
comments: true
---

The following document lists the commands that the CLI currently supports.

!!! note
    Use `--help` as a flag on any command for more information.

## Usage

Call a command on the `miden-client` like this:

```sh
miden <command> <flags> <arguments>
```

Optionally, you can include the `--debug` flag to run the command with debug mode, which enables debug output logs from scripts that were compiled in this mode:

```sh
miden --debug <flags> <arguments>
```

Note that the debug flag overrides the `MIDEN_DEBUG` environment variable.

## Commands

### `init`

Creates a configuration file for the client in the current directory.

```sh
# This will create a config file named `miden-client.toml` using default values
# This file contains information useful for the CLI like the RPC provider and database path
miden init

# You can use the --rpc flag to override the default rpc config
miden init --rpc 18.203.155.106
# You can specify the port
miden init --rpc 18.203.155.106:8080
# You can also specify the protocol (http/https)
miden init --rpc https://18.203.155.106
# You can specify both
miden init --rpc https://18.203.155.106:1234

# You can use the --store_path flag to override the default store config
miden init --store_path db/store.sqlite3

# You can provide both flags
miden init --rpc 18.203.155.106 --store_path db/store.sqlite3
```

### `account`

Inspect account details.

#### Action Flags

| Flags           | Description                                         | Short Flag|
|-----------------|-----------------------------------------------------|-----------|
|`--list`         | List all accounts monitored by this client          | `-l`      |
|`--show <ID>`    | Show details of the account for the specified ID    | `-s`      |
|`--default <ID>` | Manage the setting for the default account          | `-d`      |

The `--show` flag also accepts a partial ID instead of the full ID. For example, instead of:

```sh
miden account --show 0x8fd4b86a6387f8d8
```

You can call:

```sh
miden account --show 0x8fd4b86
```

For the `--default` flag, if `<ID>` is "none" then the previous default account is cleared. If no `<ID>` is specified then the default account is shown.

### `new-wallet`

Creates a new wallet account.

This command has two optional flags:
- `--storage-type <TYPE>`: Used to select the storage mode of the account (private if not specified). It may receive "private" or "public".
- `--mutable`: Makes the account code mutable (it's immutable by default).

After creating an account with the `new-wallet` command, it is automatically stored and tracked by the client. This means the client can execute transactions that modify the state of accounts and track related changes by synchronizing with the Miden node.

### `new-faucet`

Creates a new faucet account.

This command has two optional flags:
- `--storage-type <type>`: Used to select the storage mode of the account (private if not specified). It may receive "private" or "public".
- `--non-fungible`: Makes the faucet asset non-fungible (it's fungible by default).

After creating an account with the `new-faucet` command, it is automatically stored and tracked by the client. This means the client can execute transactions that modify the state of accounts and track related changes by synchronizing with the Miden node.

### `info`

View a summary of the current client state.

### `notes`

View and manage notes.

#### Action Flags

| Flags             | Description                                                 | Short Flag |
|-------------------|-------------------------------------------------------------|------------|
|`--list [<filter>]`| List input notes                                            | `-l`       |
| `--show <ID>`     | Show details of the input note for the specified note ID    | `-s`       |

The `--list` flag receives an optional filter:
    - expected: Only lists expected notes.
    - commited: Only lists commited notes.
    - consumed: Only lists consumed notes.
    - consumable: Only lists consumable notes. An additional `--account-id <ID>` flag may be added to only show notes consumable by the specified account.
If no filter is specified then all notes are listed.

The `--show` flag also accepts a partial ID instead of the full ID. For example, instead of:

```sh
miden notes --show 0x70b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0
```

You can call:

```sh
miden notes --show 0x70b7ec
```

### `sync`

Sync the client with the latest state of the Miden network. Shows a brief summary at the end.

### `tags`

View and add tags.

#### Action Flags

| Flag            | Description                                                 | Aliases |
|-----------------|-------------------------------------------------------------|---------|
| `--list`        | List all tags monitored by this client                      | `-l`    |
| `--add <tag>`   | Add a new tag to the list of tags monitored by this client  | `-a`    |
| `--remove <tag>`| Remove a tag from the list of tags monitored by this client | `-r`    |

### `tx`

View transactions.

#### Action Flags

| Command | Description                                              | Aliases |
|---------|----------------------------------------------------------|---------|
| `--list`| List tracked transactions                                | -l      |

After a transaction gets executed, two entities start being tracked:

- The transaction itself: It follows a lifecycle from `expected` (initial state) and `committed` (after the node receives it).
- Output notes that might have been created as part of the transaction (for example, when executing a pay-to-id transaction).

### Transaction creation commands
#### `mint`

Creates a note that contains a specific amount tokens minted by a faucet, that the target Account ID can consume.

Usage: `miden mint --target <TARGET ACCOUNT ID> --asset <AMOUNT>::<FAUCET ID> --note-type <NOTE_TYPE>`

#### `consume-notes`

Account ID consumes a list of notes, specified by their Note ID.

Usage: `miden consume-notes --account <ACCOUNT ID> [NOTES]`

For this command, you can also provide a partial ID instead of the full ID for each note. So instead of

```sh
miden consume-notes --account <some-account-id> 0x70b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0 0x80b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0
```

You can do:

```sh
miden consume-notes --account <some-account-id> 0x70b7ecb 0x80b7ecb
```

Additionally, you can optionally not specify note IDs, in which case any note that is known to be consumable by the executor account ID will be consumed.

Either `Expected` or `Committed` notes may be consumed by this command, changing their state to `Processing`. It's state will be updated to `Consumed` after the next sync.

#### `send`

Sends assets to another account. Sender Account creates a note that a target Account ID can consume. The asset is identified by the tuple `(FAUCET ID, AMOUNT)`. The note can be configured to be recallable making the sender able to consume it after a height is reached.

Usage: `miden send --sender <SENDER ACCOUNT ID> --target <TARGET ACCOUNT ID> --asset <AMOUNT>::<FAUCET ID> --note-type <NOTE_TYPE> <RECALL_HEIGHT>`

#### `swap`

The source account creates a Swap note that offers some asset in exchange for some other asset. When another account consumes that note, it'll receive the offered amount and it'll have the requested amount removed from its assets (and put into a new note which the first account can then consume). Consuming the note will fail if the account doesn't have enough of the requested asset.

Usage:  `miden swap --source <SOURCE ACCOUNT ID> --offered-asset <OFFERED AMOUNT>::<OFFERED FAUCET ID> --requested-asset <REQUESTED AMOUNT>::<REQUESTED FAUCET ID> --note-type <NOTE_TYPE>`

#### Tips
For `send` and `consume-notes`, you can omit the `--sender` and `--account` flags to use the default account defined in the [config](./cli-config.md). If you omit the flag but have no default account defined in the config, you'll get an error instead.

For every command which needs an account ID (either wallet or faucet), you can also provide a partial ID instead of the full ID for each account. So instead of

```sh
miden send --sender 0x80519a1c5e3680fc --target 0x8fd4b86a6387f8d8 --asset 100::0xa99c5c8764d4e011
```

You can do:

```sh
miden send --sender 0x80519 --target 0x8fd4b --asset 100::0xa99c5c8764d4e011
```

!!! note
    The only exception is for using IDs as part of the asset, those should have the full faucet's account ID.

#### Transaction confirmation

When creating a new transaction, a summary of the transaction updates will be shown and confirmation for those updates will be prompted:

```sh
miden <tx command> ...

TX Summary:

...

Continue with proving and submission? Changes will be irreversible once the proof is finalized on the rollup (Y/N)
```

This confirmation can be skipped in non-interactive environments by providing the `--force` flag (`miden send --force ...`):

### Importing and exporting

#### `export`

Export input note data to a binary file .

| Flag                           | Description                                    | Aliases |
|--------------------------------|------------------------------------------------|---------|
| `--filename <FILENAME>`        | Desired filename for the binary file.          | `-f`    |
| `--export-type <EXPORT_TYPE>`  | Exported note type.                            | `-e`    |

##### Export type

The user needs to specify how the note should be exported via the `--export-type` flag. The following options are available:

- `id`: Only the note ID is exported. When importing, if the note ID is already tracked by the client, the note will be updated with missing information fetched from the node. This works for both public and private notes. If the note isn't tracked and the note is public, the whole note is fetched from the node and is stored for later use.
- `full`: The note is exported with all of its information (metadata and inclusion proof). When importing, the note is considered committed. The note may not be consumed directly after importing as its block header will not be stored in the client. The block header will be fetched during the next sync.
- `partial`: The note is exported with minimal information and may be imported even if the note is not yet committed on chain. At the moment of importing the note, the client will check the state of the note by doing a note sync, using the note's tag. Depending on the response, the note will be either stored as "Expected" or "Committed".

#### `import`

Import entities managed by the client, such as accounts and notes. The type of entities is inferred.
