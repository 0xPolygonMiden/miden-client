---
comments: true
---

The following document lists the commands that the CLI currently supports. 

!!! note
    Use `--help` as a flag on any command for more information.

## Usage

Call a command on the `miden-client` like this:

```sh
miden-client <command> <sub-command>
```

Optionally, you can include the `--debug` flag to run the command with debug mode, which enables debug output logs from scripts that were compiled in this mode:

```sh
miden-client --debug <command> <sub-command>
```

Note that the debug flag overrides the `MIDEN_DEBUG` environment variable.

## Commands

### `account` 

Create accounts and inspect account details.

#### Sub-commands

| Sub-command | Description                                         | Aliases |
|---------|-----------------------------------------------------|---------|
| `list`    | List all accounts monitored by this client         | -l      |
| `show`    | Show details of the account for the specified ID   | -s      |
| `new <ACCOUNT TYPE>`  | Create new account and store it locally  | -n      |
| `import`  | Import accounts from binary files | -i      |
| `default`  | Manage the setting for the default account | -d      |

After creating an account with the `new` command, it is automatically stored and tracked by the client. This means the client can execute transactions that modify the state of accounts and track related changes by synchronizing with the Miden node.

The `show` subcommand also accepts a partial ID instead of the full ID. For example, instead of:

```sh
miden-client account show 0x8fd4b86a6387f8d8
```

You can call:

```sh
miden-client account show 0x8fd4b86
```

### `info`

View a summary of the current client state.

### `input-notes` 

View and manage input notes. 

#### Sub-commands

| Command           | Description                                                 | Aliases |
|-------------------|-------------------------------------------------------------|---------|
| `list`            | List input notes                                            | -l      |
| `show`            | Show details of the input note for the specified note ID    | -s      |
| `export`          | Export input note data to a binary file                     | -e      |
| `import`          | Import input note data from a binary file                   | -i      |
| `list-consumables`| List consumable notes by tracked accounts                   | -c      |

The `show` subcommand also accepts a partial ID instead of the full ID. For example, instead of:

```sh
miden-client input-notes show 0x70b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0 
```

You can call:

```sh
miden-client input-notes show 0x70b7ec
```

### `sync`

Sync the client with the latest state of the Miden network.

### `tags`

View and add tags.

#### Sub-commands

| Command | Description                                              | Aliases |
|---------|----------------------------------------------------------|---------|
| `list`    | List all tags monitored by this client                   | -l      |
| `add`     | Add a new tag to the list of tags monitored by this client | -a      |
| `remove`  | Remove a tag from the list of tags monitored by this client | -r      |

### `tx` or `transaction`

Execute and view transactions.

#### Sub-commands

| Command | Description                                              | Aliases |
|---------|----------------------------------------------------------|---------|
| `list`  | List tracked transactions                                | -l      |
| `new  <TX TYPE>` | Execute a transaction, prove and submit it to the node. Once submitted, it gets tracked by the client.   | -n      |

After a transaction gets executed, two entities start being tracked:

- The transaction itself: It follows a lifecycle from `pending` (initial state) and `committed` (after the node receives it).
- Output notes that might have been created as part of the transaction (for example, when executing a pay-to-id transaction).

#### Types of transaction

| Command         | Explanation                                                                                                       |
|-----------------|-------------------------------------------------------------------------------------------------------------------|
| `p2id --sender <SENDER ACCOUNT ID> --target <TARGET ACCOUNT ID> --faucet <FAUCET ID> <AMOUNT> --note-type <NOTE_TYPE>`            | Pay-to-id transaction. Sender Account creates a note that a target Account ID can consume. The asset is identifed by the tuple `(FAUCET ID, AMOUNT)`. |
| `p2idr --sender <SENDER ACCOUNT ID> --target <TARGET ACCOUNT ID> --faucet <FAUCET ID> <AMOUNT> <RECALL_HEIGHT> --note-type <NOTE_TYPE>`            | Pay-to-id With Recall transaction. Sender Account creates a note that a target Account ID can consume, but the Sender will also be able to consume it after `<RECALL_HEIGHT>` is reached. The asset is identifed by the tuple `(FAUCET ID, AMOUNT)`. |
| `mint --target <TARGET ACCOUNT ID> --faucet <FAUCET ID> <AMOUNT> --note-type <NOTE_TYPE>`           | Creates a note that contains a specific amount tokens minted by a faucet, that the target Account ID can consume|
| `consume-notes  --account <ACCOUNT ID> [NOTES]`  | Account ID consumes a list of notes, specified by their Note ID |

`<NOTE_TYPE>` can be either `public` or `private`.

For `consume-notes` subcommand, you can also provide a partial ID instead of the full ID for each note. So instead of 

```sh
miden-client consume-notes --account <some-account-id> 0x70b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0 0x80b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0
``` 

You can do: 

```sh
miden-client consume-notes --account <some-account-id> 0x70b7ecb 0x80b7ecb
```

Also, for `p2id`, `p2idr` and `consume-notes`, you can omit the `--sender` and `--account` flags to use the default account defined in the [config](./cli-config.md). If you omit the flag but have no default account defined in the config, you'll get an error instead.

For every command which needs an account ID (either wallet or faucet), you can also provide a partial ID instead of the full ID for each account. So instead of

```sh
miden-client tx new p2id --sender 0x80519a1c5e3680fc --target 0x8fd4b86a6387f8d8 --faucet 0xa99c5c8764d4e011 100
```

You can do:

```sh
miden-client tx new p2id --sender 0x80519 --target 0x8fd4b --faucet 0xa99c5 100
```
