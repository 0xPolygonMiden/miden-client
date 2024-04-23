---
comments: true
---

After [installation](install-and-run.md#install-the-client), use the client by running the following and adding the [relevant commands](cli-reference.md#commands):

```sh
miden-client
```

!!! info "Help" 
    Run `miden-client --help` for information on `miden-client` commands.

## Configuration

We configure the client using a [TOML](https://en.wikipedia.org/wiki/TOML) file ([`miden-client.toml`](https://github.com/0xPolygonMiden/miden-client/blob/main/miden-client.toml)). 

```sh
[rpc]
endpoint = { protocol = "http", host = "localhost", port = 57291 }

[store]
database_filepath = "store.sqlite3"
```

The TOML file should reside in same the directory from which you run the CLI.

In the configuration file, you will find a section for defining the node's `endpoint` and the store's filename `database_filepath`. 

By default, the node is set up to run on `localhost:57291`.

!!! note
    - Running the node locally for development is encouraged. 
    - However, the endpoint can point to any remote node.