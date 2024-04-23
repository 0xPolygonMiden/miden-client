---
comments: true
---

The following document lists the commands that the CLI currently supports. 

!!! note
    Use `--help` as a flag on any command for more information.

## Usage

Call a command on the `miden-client` like this:

```sh
miden-client <command> <sub-command> <--flag>
```

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

After creating an account with the `new` command, it is automatically stored and tracked by the client. This means the client can execute transactions that modify the state of accounts and track related changes by synchronizing with the Miden node.

### `info`

View a summary of the current client state.

### `input-notes` 

View and manage input notes. 

#### Sub-commands

| Command | Description                                                 | Aliases |
|---------|-------------------------------------------------------------|---------|
| `list`    | List input notes                                            | -l      |
| `show`    | Show details of the input note for the specified note ID   | -s      |
| `export`  | Export input note data to a binary file                    | -e      |
| `import`  | Import input note data from a binary file                  | -i      |

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
| `p2id <SENDER ACCOUNT ID> <TARGET ACCOUNT ID> <FAUCET ID> <AMOUNT>`            | Pay-to-id transaction. Sender Account creates a note that a target Account ID can consume. The asset is identifed by the tuple `(FAUCET ID, AMOUNT)`. |
| `mint <TARGET ACCOUNT ID> <FAUCET ID> <AMOUNT>`           | Creates a note that contains a specific amount tokens minted by a faucet, that the target Account ID can consume|
| `consume-notes  <ACCOUNT ID> [NOTES]`  | Account ID consumes a list of notes, specified by their Note ID |

For `consume-notes` subcommand, you can also provide a partial ID instead of the full ID for each note. So instead of 

```sh
miden-client consume-notes <some-account-id> 0x70b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0 0x80b7ecba1db44c3aa75e87a3394de95463cc094d7794b706e02a9228342faeb0
``` 

You can do: 

```sh
miden-client consume-notes <some-account-id> 0x70b7ecb 0x80b7ecb
```