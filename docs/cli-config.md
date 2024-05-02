---
comments: true
---

After [installation](install-and-run.md#install-the-client), use the client by running the following and adding the [relevant commands](cli-reference.md#commands):

```sh
miden-client
```

!!! info "Help" 
    Run `miden-client --help` for information on `miden-client` commands.

## Client Configuration

We configure the client using a [TOML](https://en.wikipedia.org/wiki/TOML) file ([`miden-client.toml`](https://github.com/0xPolygonMiden/miden-client/blob/main/miden-client.toml)). 

```sh
[rpc]
endpoint = { protocol = "http", host = "localhost", port = 57291 }
timeout_ms = 10000

[store]
database_filepath = "store.sqlite3"

[cli]
default_account_id = "0x012345678"
```

The TOML file should reside in same the directory from which you run the CLI.

In the configuration file, you will find a section for defining the node's rpc `endpoint` and timeout and the store's filename `database_filepath`. 

By default, the node is set up to run on `localhost:57291`.

!!! note
    - Running the node locally for development is encouraged. 
    - However, the endpoint can point to any remote node.

There's an additional **optional** section used for CLI configuration. It
currently contains the default account ID, which is used to execute
transactions against it when the account flag is not provided.

By default none is set, but you can set and unset it with:

```sh
miden-client account default set <ACCOUNT_ID>`
miden-client account default unset
```

### Environment variables

- `MIDEN_DEBUG`: When set to `true`, enables debug mode on the transaction executor and the script compiler. For any script that has been compiled and executed in this mode, debug logs will be output in order to facilitate MASM debugging ([these instructions](https://0xpolygonmiden.github.io/miden-vm/user_docs/assembly/debugging.html) can be used to do so). This variable can be overriden by the `--debug` CLI flag. 
