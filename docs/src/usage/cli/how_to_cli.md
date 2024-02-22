# How to use

After installing the client, you can use it by running `miden-client`. In order to get more information about available CLI commands you can run `miden-client --help`.

## Configuration

The CLI can be configured through a TOML file ([`miden-client.toml`](https://github.com/0xPolygonMiden/miden-client/blob/main/miden-client.toml)). This file is expected to be located in the directory from where you are running the CLI. This is useful for connecting to a specific node when developing with the client, for example.

In the configuration file, you will find a section for defining the node's endpoint and the store's filename. By default, the node will run on `localhost:57291`, so the linked example file specifies it as the node's endpoint. 

Note that running the node locally for development is encouraged, but the endpoint can be set to point to any remote node's IP as well.
