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

### Initialize a new project

```sh
arch-cli init
```

This command sets up a new Arch Network project with the necessary folder structure, boilerplate code, and Docker configurations.


### Manage the development server

```sh
arch-cli server start
arch-cli server stop
arch-cli server status
arch-cli server logs [<service>]
```

These commands start, stop, check the status of, and view logs for the development environment, including the Bitcoin regtest network and Arch Network nodes.

### Deploy a program

```sh
arch-cli deploy [--directory <path>] [--program-key <path>]
```

Compiles and deploys the specified program to the Arch Network.

### Clean up the environment

```sh
arch-cli project clean
```

Removes generated files and Docker volumes for a fresh start.

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

### Manage the frontend application

```sh
arch-cli frontend start
```

Prepares and starts the frontend application, opening it in the default browser.

### Manage accounts

```sh
arch-cli account create [--program-id <program_id>]
```

Creates an account for the dApp, prompts for funding, and transfers ownership to the specified program.

### Manage configuration

```sh
arch-cli config view
arch-cli config edit
arch-cli config reset
```

These commands allow you to view, edit, and reset the configuration file.

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

3. Start the Distributed Key Generation (DKG) process:
   ```
   arch-cli dkg start
   ```

4. Deploy your application:
   ```
   arch-cli deploy
   ```

5. Create an account for your dApp:
   ```
   arch-cli account create
   ```

6. Start the frontend application:
   ```
   arch-cli frontend start
   ```

By following these steps in order, you'll have a fully functional demo Arch Network application running locally.

## Project Structure

After initialization, your project will have the following structure:

```
my-arch-project/
├── src/
│   └── app/
│       ├── program/
│       │   └── src/
│       │       └── lib.rs
│       ├── backend/
│       │   ├── index.ts
│       │   └── package.json
│       ├── frontend/
│       │   ├── index.html
│       │   ├── index.js
│       │   ├── package.json
│       │   └── .env.example
│       └── keys/
├── Cargo.toml
├── config.toml
├── bitcoin-docker-compose.yml
└── arch-docker-compose.yml
```

## Development

To set up the development environment:

1. Clone the repository:
    ```sh
    git clone https://github.com/hoffmabc/arch-cli.git
    cd arch-cli
    ```
2. Build the project:
    ```sh
    cargo build
    ```
3. Run tests:
    ```sh
    cargo test
    ```

## Troubleshooting

- If you encounter issues with Docker networks, try running `arch-cli project clean` to remove existing volumes and networks.
- Ensure your Docker daemon is running before using `arch-cli server start`.
- Check the `config.toml` file for correct configuration of RPC endpoints and credentials.
- If you encounter issues with the DKG process, ensure that all nodes are properly configured and connected.
- For frontend issues, make sure all npm dependencies are correctly installed and that the `.env` file is properly set up.

## Support

If you encounter any problems or have any questions, please open an issue in the GitHub repository.

## Acknowledgments

- Arch Network team for the core infrastructure
- The Rust community for excellent tools and libraries
- Bitcoin Core and Electrs projects for Bitcoin node and Electrum server implementations