# Arch-CLI

Arch-CLI is a command-line interface tool designed to streamline the development process for Arch Network applications. It provides a set of commands to initialize, manage, and deploy Arch Network programs efficiently.

## Prerequisites

Before using Arch-CLI, ensure you have the following installed on your system:

- Docker (latest stable version)
- Docker Compose (latest stable version)
- Node.js (version 19 or higher)
- Solana CLI (latest stable version)
- Rust and Cargo (latest stable version)

These tools are essential for running the development environment and building Arch Network applications.

## Features

- Easy project initialization
- Development server management with Docker integration
- Simplified deployment process for both regtest and mainnet environments
- Integration with the Arch Network ecosystem
- Bitcoin regtest network setup for local development
- Distributed Key Generation (DKG) process initiation
- Send coins functionality for testing
- Frontend application management and launching
- Account creation and management
- Configuration viewing and editing
- Indexer management
- Validator management

## Installation

To install Arch-CLI, make sure you have met all the prerequisites mentioned above. Then, run:

```sh
git clone https://github.com/hoffmabc/arch-cli.git
cd arch-cli
cargo install --path .
```

## Configuration

Before using Arch-CLI, you need to set up a `config.toml` file. By default, the CLI will look for this file in the following locations:

- **Linux**: `~/.config/arch-cli/config.toml`
- **macOS**: `~/Library/Application Support/arch-cli/config.toml`
- **Windows**: `C:\Users\<User>\AppData\Roaming\arch-cli\config.toml`

If the configuration file is not found, a default configuration file will be created automatically using the `config.default.toml` template.

You can also specify a custom configuration file location by setting the `ARCH_CLI_CONFIG` environment variable:

```sh
export ARCH_CLI_CONFIG=/path/to/your/config.toml
```

Here's an example configuration:

```toml
[network]
type = "development"  # Options: development, testnet, mainnet

[bitcoin]
docker_compose_file = "./bitcoin-docker-compose.yml"
network = "regtest"
rpc_endpoint = "http://localhost:18443"
rpc_port = "18443"
rpc_user = "bitcoin"
rpc_password = "password"
rpc_wallet = "devwallet"

[arch]
docker_compose_file = "./arch-docker-compose.yml"
leader_rpc_endpoint = "http://localhost:8080"
network_mode = "development"
rust_log = "info"
rust_backtrace = "1"
bootnode_ip = "127.0.0.1"
leader_p2p_port = "9000"
leader_rpc_port = "8080"
validator1_p2p_port = "9001"
validator1_rpc_port = "8081"
validator2_p2p_port = "9002"
validator2_rpc_port = "8082"
bitcoin_rpc_endpoint = "http://localhost:18443"
bitcoin_rpc_wallet = "devwallet"
replica_count = "3"

[program]
key_path = "src/app/keys/program.json"
```

By following these steps, you ensure that your CLI can be run from any location and still correctly locate and load its configuration files on Windows, macOS, and Linux.

## Usage

Here are the main commands available in Arch-CLI:

### Initialize Arch Network

```sh
arch-cli init
```

This command sets up a new Arch Network project with the necessary folder structure, boilerplate code, and Docker configurations.

### Manage the local development server (Docker)

```sh
arch-cli server start
arch-cli server stop
arch-cli server status
arch-cli server logs [<service>]
arch-cli server clean
```

These commands start, stop, check the status of, view logs for, and clean up the development environment, including the Bitcoin regtest network and Arch Network nodes.

### Deploy a program

```sh
arch-cli deploy [--directory <path>] [--program-key <path>] [--folder <folder>]
```

Compiles and deploys the specified program to the Arch Network.

### Manage a project

```sh
arch-cli project create [--name <project_name>]
```

Creates a new project with the specified name.

### Start Distributed Key Generation (DKG) process

```sh
arch-cli dkg start
```

Initiates the Distributed Key Generation process on the Arch Network.

### Send coins (for testing)

```sh
arch-cli bitcoin send-coins --address <address> --amount <amount>
```

Sends the specified amount of coins to the given address on the Bitcoin Regtest network.

### Manage the demo application

Arch Network comes bundled with a block explorer and graffiti wall demonstration application. These commands manage that application.

```sh
arch-cli demo start
arch-cli demo stop
```

Starts or stops the demo application, including both frontend and backend services.

### Manage accounts

```sh
arch-cli account create [--program-id <program_id>] --name <account_name>
arch-cli account list
arch-cli account delete <account_id_or_name>
```

Creates, lists, or deletes accounts for your dapps.

### Manage configuration

```sh
arch-cli config view
arch-cli config edit
arch-cli config reset
```

These commands allow you to view, edit, and reset the configuration file.

### Manage the indexer

```sh
arch-cli indexer start
arch-cli indexer stop
arch-cli indexer clean
```

Starts, stops, or cleans the arch-indexer using Docker Compose.

### Manage the validator

The validator is a lightweight server that only serves as an RPC for developers to get up and running quickly with the least amount of overhead.

```sh
arch-cli validator start [options]
arch-cli validator stop
```

Starts a local validator with specified network settings or stops the local validator.

## Getting Started with the Demo App

To quickly set up and run the demo application, follow these steps:

1. Initialize the project:
   ```
   arch-cli init
   ```

2. Start the development server:
   ```
   arch-cli server start
   ```

3. Deploy your application:
   ```
   arch-cli deploy
   ```

   Choose the demo application to deploy. When asked to create a key, do so.

5. Start the demo application:
   ```
   arch-cli demo start
   ```

By following these steps in order, you'll have a fully functional demo Arch Network application running locally.

## Project Structure

[Project Structure section remains unchanged]

## Development

[Development section remains unchanged]

## Troubleshooting

- If you encounter issues with Docker networks, try running `arch-cli server clean` to remove existing volumes and networks.
- Ensure your Docker daemon is running before using `arch-cli server start`.
- Check the `config.toml` file for correct configuration of RPC endpoints and credentials.
- If you encounter issues with the DKG process, ensure that all nodes are properly configured and connected.
- For demo application issues, make sure all dependencies are correctly installed and that the necessary configuration files are properly set up.

## Support

If you encounter any problems or have any questions, please open an issue in the GitHub repository.

## Acknowledgments

- Arch Network team for the core infrastructure
- The Rust community for excellent tools and libraries
- Bitcoin Core and Electrs projects for Bitcoin node and Electrum server implementations
