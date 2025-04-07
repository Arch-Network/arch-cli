THIS REPO IS NO LONGER BEING DEVELOPED. USE FOR EDUCATIONAL PURPOSES ONLY. PLEASE SEE HERE FOR LATEST CLI VERSION: https://github.com/Arch-Network/arch-node/releases


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
git clone https://github.com/Arch-Network/arch-cli.git
cd arch-cli
cargo install --path .
```

## Getting Started with the Demo App

To quickly set up and run the demo application, follow these steps:

1. Initialize the project:
   ```
   arch-cli init
   ```

2. Start the development server:
   ```
   arch-cli validator start
   ```

3. Start the demo application:
   ```
   arch-cli demo start
   ```

By following these steps in order, you'll have a fully functional demo Arch Network application running locally.



## Configuration

Before using Arch-CLI, you need to set up a `config.toml` file. By default, the CLI will look for this file in the following locations:

- **Linux**: `~/.config/arch-cli/config.toml`
- **macOS**: `~/Library/Application Support/arch-cli/config.toml`
- **Windows**: `C:\Users\<User>\AppData\Roaming\arch-cli\config.toml`

If the configuration file is not found, a default configuration file will be created automatically using the `config.default.toml` template.


## Usage

Here are the main commands available in Arch-CLI:

### Initialize Arch Network

```sh
arch-cli init
```

This command sets up a new Arch Network project with the necessary folder structure, boilerplate code, and Docker configurations.

**You MUST run this command before using any other Arch-CLI commands.**

### Run a Local Validator

For quick development and testing, you can run a single local validator node using the following command:

```sh
arch-cli validator start [--network <network>]
```

This command starts a lightweight local validator that serves as an RPC endpoint, allowing you to develop and test your Arch Network applications with minimal setup.

- `--network <network>`: Specify the network to connect to (e.g., 'development', 'testnet', 'mainnet'). Default is 'development'.

To stop the local validator, use:

```sh
arch-cli validator stop
```

Running a local validator is an easy way to get started with development, as it provides a single node that you can interact with for testing your applications. This is particularly useful when you don't need the full complexity of a multi-node setup provided by the `server start` command.


### Manage a self-contained Arch Network locally (ADVANCED)

This set of commands allow developers to create a fully self-contained Arch Network environment that does not rely on third-party hosted servers, meaning you will have your own local leader node, several validator nodes, and the regtest Bitcoin infrastructure all hosted on Docker. Managing your own full network is not necessary for developing Arch Network programs or decentralized applications on top of those programs. You should avoid deploying these containers unless you are working on core Arch Network components or would like to understand better how Arch validators communicated and operate with each other.

```sh
arch-cli server start
arch-cli server stop
arch-cli server status
arch-cli server logs [--service <service_name>]
arch-cli server clean
```

These commands start, stop, check the status of, view logs for, and clean up the development environment, including the Bitcoin regtest network and Arch Network nodes.

- `--service <service_name>`: Specify which service to show logs for (e.g., 'bitcoin', 'arch', 'bootnode', 'leader', 'validator-1', 'validator-2')

### Deploy a program

```sh
arch-cli deploy --elf-path <path> [--program-key <path>] [--rpc-url <url>]
```

Deploys a compiled program to the Arch Network. The deploy command handles the complete deployment process, including:
- Verifying the ELF binary
- Managing program keys
- Setting up Bitcoin RPC client
- Funding the program account
- Deploying program transactions
- Making the program executable

#### Arguments:

- `--elf-path <path>` (Required): Path to the compiled ELF binary file
- `--program-key <path>` (Optional): Path to a file containing hex-encoded private key for deployment
  - If not provided, you'll be prompted to either:
    - Choose from existing keys in your keys.json
    - Create a new program key
- `--rpc-url <url>` (Optional): RPC URL for connecting to the Arch Network
  - Defaults to the configured leader_rpc_endpoint or NODE1_ADDRESS

#### Example Usage:

```sh
# Deploy with a specific program key
arch-cli deploy --elf-path target/deploy/myprogram.so --program-key keys/program.key

# Deploy using interactive key selection
arch-cli deploy --elf-path target/deploy/myprogram.so

# Deploy to a specific RPC endpoint
arch-cli deploy --elf-path target/deploy/myprogram.so --rpc-url http://localhost:9002
```

The deployment process will display progress information and the final Program ID upon successful completion.

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

Starts or stops the demo application.

### Manage accounts

```sh
arch-cli account create [--program-id <program_id>] --name <account_name>
arch-cli account list
arch-cli account delete <account_id_or_name>
arch-cli account transfer-ownership <account_id_or_name> <new_owner_id_or_name>
arch-cli account update <account_id_or_name> --data-file <path_to_data_file> [--rpc-url <rpc_url>]
```

Creates, lists, or deletes accounts for your dapps.

- `create`: Creates a new account with an optional program ID for ownership.
- `list`: Lists all accounts stored in the accounts file.
- `delete`: Deletes an account by its ID or name.
- `transfer-ownership`: Transfers ownership of an account to a specified program.
- `update`: Updates the account data from a specified file. You need to provide the path to the data file and optionally the RPC URL for the Arch Network node.

### Manage configuration

```sh
arch-cli config view
arch-cli config edit
arch-cli config reset
```

These commands allow you to view, edit, and reset the configuration file.

### Manage the indexer

```sh
arch-cli indexer start [--arch-node-url <url>]
arch-cli indexer stop
arch-cli indexer clean
```

Starts, stops, or cleans the arch-indexer using Docker Compose.

- `--arch-node-url <url>`: Specify the URL of the Arch node to connect to



## Project Structure

After you run `arch-cli project create`, your project will have the following structure:

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
