`ord` - BRC20 & BRC2.0 Enhanced
=================================

> **This is an enhanced fork of the original [ord](https://github.com/ordinals/ord) project with full BRC20 and BRC2.0 protocol support.**

`ord` is an index, block explorer, and command-line wallet for Bitcoin ordinals. This fork extends the original functionality with comprehensive BRC20 and BRC2.0 indexing capabilities.

## What's New in This Fork

- **🎯 BRC20 & BRC2.0 Protocol Support**: Full indexing and querying of BRC20 tokens and BRC2.0 programmable events
- **🔍 OPI Validation**: Built-in validation against the Ordinals Protocol Index (OPI) specification
- **📊 Extended Indexing**: Support for Bitmap, BTC Domain, and inscription receipts
- **🌐 EVM JSON-RPC**: Separate RPC server for BRC2.0 EVM endpoints
- **🔗 External Integration**: Seamless integration with `brc20-prog` server for programmable token logic

For detailed BRC20/BRC2.0 documentation, see [README.brc20.md](README.brc20.md).

## Table of Contents

- [Quick Start](#quick-start)
- [BRC20 & BRC2.0 Setup](#brc20--brc20-setup)
- [Installation](#installation)
- [Building from Source](#building-from-source)
- [Configuration](#configuration)
- [Extended Indexing Options](#extended-indexing-options)
- [Wallet](#wallet)
- [Contributing](#contributing)
- [Documentation](#documentation)

## Quick Start

### Prerequisites

Before using this fork, ensure you have:
- A synced Bitcoin Core node with `-txindex` enabled
- The `brc20-prog` server for BRC20/BRC2.0 functionality

### Basic Usage

```bash
# Start ord with BRC20 indexing
./ord \
  --index-brc20 \
  --index-addresses \
  --brc20-prog-url http://127.0.0.1:18545 \
  --brc20-prog-username user \
  --brc20-prog-password password \
  server
```

## BRC20 & BRC2.0 Setup

### 1. Install brc20-prog Server

The BRC20 functionality requires an external `brc20-prog` server.

**Option A: Install via Cargo**
```bash
cargo install brc20-prog
```

**Option B: Build from Source**
```bash
git clone https://github.com/bestinslot-xyz/brc20-programmable-module.git
cd brc20-programmable-module
cargo build --release
```

### 2. Configure and Start brc20-prog

Set up environment variables:

```bash
export BITCOIN_RPC_URL="http://127.0.0.1:8332"
export BITCOIN_RPC_USER="your_username"
export BITCOIN_RPC_PASSWORD="your_password"
export BITCOIN_RPC_NETWORK="mainnet"  # or "signet"

export BRC20_PROG_RPC_SERVER_URL=127.0.0.1:18545
export BRC20_PROG_RPC_SERVER_ENABLE_AUTH=true
export BRC20_PROG_RPC_SERVER_USER=user
export BRC20_PROG_RPC_SERVER_PASSWORD=password
```

Start the server:
```bash
brc20-prog -l info
```

For detailed configuration options, see the [brc20-prog documentation](https://github.com/bestinslot-xyz/brc20-programmable-module).

### 3. Enable BRC20 Indexing in ord

**Using Command Line Flags:**

```bash
./ord \
  --index-brc20 \
  --index-addresses \
  --brc20-prog-url http://127.0.0.1:18545 \
  --brc20-prog-username user \
  --brc20-prog-password password \
  server
```

**Using Environment Variables:**

```bash
export ORD_INDEX_BRC20=true
export ORD_INDEX_ADDRESSES=true
export ORD_BRC20_PROG_URL=http://127.0.0.1:18545
export ORD_BRC20_PROG_USERNAME=user
export ORD_BRC20_PROG_PASSWORD=password
ord server
```

**Using Config File (config.yaml):**

```yaml
index_brc20: true
index_addresses: true
brc20_prog_url: http://127.0.0.1:18545
brc20_prog_username: user
brc20_prog_password: password
```

### 4. OPI Validation

Enable validation against the OPI specification:

**Standard Validation (logs mismatches):**
```bash
./ord \
  --index-brc20 \
  --opi-validation \
  ...
```

**Strict Validation (panics on mismatch):**
```bash
./ord \
  --index-brc20 \
  --opi-validation \
  --opi-validation-strict \
  ...
```

#### OPI Validation Configuration

You can configure OPI validation behavior using environment variables:

**`CHECKPOINT_INTERVAL`** - Controls validation frequency for historical blocks (default: `1000`)
- Recent blocks (within 24 hours): Every block is validated
- Historical blocks (older than 24 hours): Validated every N blocks as checkpoints
- Lower values provide more frequent validation but slower sync
- Higher values provide faster sync but less frequent validation

**`OPI_API_URL`** - Specifies the OPI API endpoint (default: `https://api.opi.network`)
- Use this to connect to a custom OPI API server
- Useful for testing or using alternative OPI implementations

**Example:**
```bash
export CHECKPOINT_INTERVAL=500          # Validate every 500 blocks for historical data
export OPI_API_URL=https://custom.opi.server

./ord \
  --index-brc20 \
  --opi-validation \
  --brc20-prog-url http://127.0.0.1:18545 \
  --brc20-prog-username user \
  --brc20-prog-password password \
  server
```

**Validation Strategy:**
- **Recent blocks** (< 24 hours old): Validated at every height
- **Historical blocks** (> 24 hours old): Validated at checkpoint intervals
- **Checkpoint height**: `height % CHECKPOINT_INTERVAL == 0`

### 5. Accessing BRC20 Data

Once the indexer is running, you can retrieve BRC20 event data through the API endpoints. BRC2.0 EVM endpoints are accessible via a separate RPC server that supports standard EVM JSON-RPC.

For detailed API documentation, see [README.brc20.md](README.brc20.md).

## Installation

### Pre-built Binaries

Download pre-built binaries from the [releases page](https://github.com/okx/ord/releases).

Or install via command line:

```sh
curl --proto '=https' --tlsv1.2 -fsLS https://ordinals.com/install.sh | bash -s
```

Verify installation:
```sh
ord --version
```

### Homebrew

```sh
brew install ord
```

### Docker

Build the Docker image:

```sh
docker build -t okx/ord .
```

### Debian Package

Build a `.deb` package:

```sh
cargo install cargo-deb
cargo deb
```

## Building from Source

### System Requirements

**On Debian/Ubuntu:**
```sh
sudo apt-get install pkg-config libssl-dev build-essential
```

**On Red Hat/CentOS:**
```sh
yum install -y pkgconfig openssl-devel
yum groupinstall "Development Tools"
```

### Install Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Requires `rustc` version 1.79.0 or later. Check your version:
```sh
rustc --version
```

Update if needed:
```sh
rustup update
```

### Build ord

Clone the repository:
```sh
git clone https://github.com/okx/ord.git
cd ord
```

To build a specific version:
```sh
git checkout <VERSION>
```

Build the release binary:
```sh
cargo build --release
```

The binary will be available at `./target/release/ord`.

### Docker Build

```sh
docker build -t okx/ord .
```

## Configuration

### Bitcoin Core Setup

`ord` requires a synced `bitcoind` node with `-txindex` enabled. If `bitcoind` is running locally with default settings, `ord` will auto-detect it.

For non-default configurations, use the appropriate flags. See `ord --help` for details.

### RPC Authentication

`ord` communicates with `bitcoind` via RPC and requires authentication.

**Using Cookie File (Default):**
```sh
ord --cookie-file /path/to/cookie/file server
```

**Using Username/Password:**
```sh
ord --bitcoin-rpc-username foo --bitcoin-rpc-password bar server
```

**Using Environment Variables:**
```sh
export ORD_BITCOIN_RPC_USERNAME=foo
export ORD_BITCOIN_RPC_PASSWORD=bar
ord server
```

**Using Config File:**
```yaml
bitcoin_rpc_username: foo
bitcoin_rpc_password: bar
```

## Extended Indexing Options

This fork supports several extended indexing options for protocol-specific inscriptions.

**⚠️ Important:** All extended indexing options require `--index-addresses` to be enabled.

### `--index-brc20`

Index BRC-20 protocol inscriptions, including BRC20 token operations and BRC2.0 programmable events.

**Requires:** `--index-addresses` and external `brc20-prog` server

```bash
./ord \
  --index-addresses \
  --index-brc20 \
  --brc20-prog-url http://127.0.0.1:18545 \
  --brc20-prog-username user \
  --brc20-prog-password password \
  server
```

### `--index-bitmap`

Index Bitmap protocol inscriptions for on-chain bitmap images.

**Requires:** `--index-addresses` | **Mainnet only**

```bash
./ord \
  --index-addresses \
  --index-bitmap \
  server
```

### `--index-btc-domain`

Index BTC Domain protocol inscriptions and registrations.

**Requires:** `--index-addresses` | **Mainnet only**

```bash
./ord \
  --index-addresses \
  --index-btc-domain \
  server
```

### `--save-inscription-receipts`

Store detailed inscription receipt data for protocol-specific queries and verification.

**Requires:** `--index-addresses`

```bash
./ord \
  --index-addresses \
  --save-inscription-receipts \
  server
```

### Combining Multiple Options

All extended indexing options can be combined:

```bash
./ord \
  --index-addresses \
  --index-brc20 \
  --index-bitmap \
  --index-btc-domain \
  --save-inscription-receipts \
  --brc20-prog-url http://127.0.0.1:18545 \
  --brc20-prog-username user \
  --brc20-prog-password password \
  server
```

**Environment Variables:**
```bash
export ORD_INDEX_ADDRESSES=true
export ORD_INDEX_BRC20=true
export ORD_INDEX_BITMAP=true
export ORD_INDEX_BTC_DOMAIN=true
export ORD_SAVE_INSCRIPTION_RECEIPTS=true
export ORD_BRC20_PROG_URL=http://127.0.0.1:18545
export ORD_BRC20_PROG_USERNAME=user
export ORD_BRC20_PROG_PASSWORD=password
ord server
```

**Config File:**
```yaml
index_addresses: true
index_brc20: true
index_bitmap: true
index_btc_domain: true
save_inscription_receipts: true
brc20_prog_url: http://127.0.0.1:18545
brc20_prog_username: user
brc20_prog_password: password
```

## Wallet

`ord` relies on Bitcoin Core for private key management and transaction signing.

### ⚠️ Important Wallet Considerations

- **Bitcoin Core is not ordinal-aware**: Using `bitcoin-cli` commands directly may lead to loss of inscriptions
- **Automatic wallet loading**: `ord wallet` commands automatically load the wallet specified by `--name` (default: 'ord')
- **Segregate funds**: Do not use `ord` with wallets containing significant funds. Keep ordinal and cardinal wallets separate

### Pre-alpha Wallet Migration

Alpha `ord` wallets are incompatible with previous versions. To migrate:

```sh
# From old wallet, send to new addresses
ord wallet send <amount> <address>

# Generate new addresses in new wallet
ord wallet receive
```

## Logging

`ord` uses [env_logger](https://docs.rs/env_logger/latest/env_logger/) for logging.

**Enable logging:**
```sh
RUST_LOG=info cargo run server
```

**Enable debug logging with backtrace:**
```sh
RUST_BACKTRACE=1 RUST_LOG=debug ord server
```

## Contributing

Contributions are welcome! This project emphasizes proper testing with three categories:

- **Unit tests**: Located at the bottom of files in `tests` mod blocks
- **Integration tests**: End-to-end functionality tests in the [tests](tests) directory
- **Fuzz tests**: Fuzzing structure in the [fuzz](fuzz) directory

### Running Tests

We recommend installing [just](https://github.com/casey/just) for easier test execution:

```sh
# Run full CI test suite
just ci
```

This runs:
```sh
cargo fmt -- --check
cargo test --all
cargo test --all -- --ignored
```

### Useful Commands

```sh
just fmt                    # Format code
just fuzz                   # Run fuzz tests
just doc                    # Build documentation
just watch ltest --all      # Watch mode for tests
```

### Test-Driven Development

We follow TDD practices using a mocked Bitcoin Core instance ([mockcore](./crates/mockcore)) for fast feedback loops.

### Increasing File Limits

If tests are failing or hanging, you may need to increase open file limits:

```sh
ulimit -n 1024
```

## Documentation

- **Official Ordinals Docs**: [docs.ordinals.com](https://docs.ordinals.com)
- **BIP**: See [bip.mediawiki](bip.mediawiki) for technical specification
- **BRC20 Details**: See [README.brc20.md](README.brc20.md) for BRC20/BRC2.0 specific documentation
- **Project Board**: [GitHub Projects](https://github.com/orgs/ordinals/projects/1)
- **Discord**: Join [the Discord server](https://discord.gg/87cjuz4FYg)

## Translations

Documentation translations use [mdBook i18n helper](https://github.com/google/mdbook-i18n-helpers).

See the [usage guide](https://github.com/google/mdbook-i18n-helpers/blob/main/i18n-helpers/USAGE.md) for details.

### Starting a New Translation

1. Install tools:
   ```sh
   cargo install mdbook mdbook-i18n-helpers mdbook-linkcheck
   ```

2. Generate POT file:
   ```sh
   MDBOOK_OUTPUT='{"xgettext": {"pot-file": "messages.pot"}}'
   mdbook build -d po
   ```

3. Update translation file (replace `XX` with your language code):
   ```sh
   msgmerge --update po/XX.po po/messages.pot
   ```

4. Edit `msgstr` strings in `XX.po` to add translations

5. Build translated docs:
   ```sh
   mdbook build docs -d build
   MDBOOK_BOOK__LANGUAGE=XX mdbook build docs -d build/XX
   mv docs/build/XX/html docs/build/html/XX
   python3 -m http.server --directory docs/build/html --bind 127.0.0.1 8080
   ```

6. Commit `XX.po` and open a pull request

See [this example commit](https://github.com/ordinals/ord/commit/329f31bf6dac207dad001507dd6f18c87fdef355) for reference.

## Release Process

Release commit messages follow this template:

```
Release x.y.z

- Bump version: x.y.z → x.y.z
- Update changelog
- Update changelog contributor credits
- Update dependencies
```

## About Ordinal Theory

Ordinal theory imbues satoshis with numismatic value, allowing them to be collected and traded as curios.

Ordinal numbers are serial numbers for satoshis, assigned in the order in which they are mined, and preserved across transactions.

## Original Project

This is a fork of the original [ord](https://github.com/ordinals/ord) project by [raphjaph](https://github.com/raphjaph/) and the ordinals community.

For information about the original project and its development, visit:
- Original Repository: https://github.com/ordinals/ord
- Official Site: https://ordinals.com

## License

See [LICENSE](LICENSE) for details. This is experimental software with no warranty.
