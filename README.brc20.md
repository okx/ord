# Guide for indexing BRC20 and BRC2.0 events using OKX Ordinals Indexer

This document provides instructions on how to index BRC20 and BRC2.0 events using the OKX Ordinals Indexer. It covers the necessary steps to set up the environment, configure the indexer, and retrieve event data.

## Prerequisites

- Ensure you install brc20-prog using Cargo, or build it from source:

```bash
cargo install brc20-prog
```

```bash
git clone https://github.com/bestinslot-xyz/brc20-programmable-module.git
cd brc20-programmable-module
cargo build --release
```

## Starting up the BRC2.0 server

To start brc20-prog, use the following command:

```bash
brc20-prog -l info
```

Modify environment variables as needed to point to your Bitcoin node or other configurations.

```bash
BITCOIN_RPC_URL="..."
BITCOIN_RPC_USER="..."
BITCOIN_RPC_PASSWORD="..."
BITCOIN_RPC_NETWORK="signet"  # or mainnet

BRC20_PROG_RPC_SERVER_URL=127.0.0.1:18545 # Set your desired server address and port
BRC20_PROG_RPC_SERVER_ENABLE_AUTH=true # Enable authentication if needed (this will not authenticate eth_ endpoints, only brc20_ endpoints that are intended for indexers)
BRC20_PROG_RPC_SERVER_USER=user # Set your desired username
BRC20_PROG_RPC_SERVER_PASSWORD=password # Set your desired password
```

For more configuration options, refer to the [brc20-prog documentation](https://github.com/bestinslot-xyz/brc20-programmable-module).

## Configuring the OKX Ordinals Indexer to index BRC20 and BRC2.0 events

In addition to the standard configuration for the OKX Ordinals Indexer, you need to enable BRC20 and BRC2.0 indexing. This can be done by adding the `--index-brc20` flag when starting the indexer.

To point to the BRC2.0 server, use the following flags:

```bash
./ord \
  --index-brc20 \
  --index-addresses \
  --brc20-prog-url <BRC20_PROG_RPC_SERVER_URL> \
  --brc20-prog-username <BRC20_PROG_RPC_USER> \
  --brc20-prog-password <BRC20_PROG_RPC_PASSWORD> \
  ...
```

## Retrieving BRC2.0 event data

Once the indexer is running with BRC20 and BRC2.0 indexing enabled, you can retrieve event data through the API endpoints provided by the OKX Ordinals Indexer.

BRC2.0 EVM endpoint can be accessed in its own RPC server, separate from the main Ordinals Indexer server, and it supports standard EVM JSON-RPC.

## Validating BRC20 events against OPI

The indexer includes functionality to validate BRC20 events against the OPI specification. This ensures that the indexed events conform to the expected standards.

To enable OPI validation, use the following flags when starting the indexer:

```bash
./ord \
 ... \
 --opi-validation # Enable OPI validation
 --opi-validation-strict # Enable strict OPI validation (panics on hash mismatch)
```

The `--opi-validation` flag enables validation of BRC20 events against OPI, while the `--opi-validation-strict` flag will cause the indexer to panic if there is any mismatch in the event hashes.
