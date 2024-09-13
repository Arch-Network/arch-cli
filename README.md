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

## Installation

To install Arch-CLI, make sure you have met all the prerequisites mentioned above. Then, run:

```sh
git clone https://github.com/hoffmabc/arch-cli.git
cd arch-cli
cargo install --path .
```

## Configuration

Before using Arch-CLI, you need to set up a `config.toml` file in your project root. Here's an example configuration:

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

[electrs]
rest_api_port = "3000"
electrum_port = "50001"

[btc_rpc_explorer]
port = "3002"

[ord]
port = "3003"
```

Adjust these values according to your setup.

## Usage

Here are the main commands available in Arch-CLI:

### Initialize a new project

```sh
arch-cli init
```

This command sets up a new Arch Network project with the necessary folder structure, boilerplate code, and Docker configurations.

### Start the development server

```sh
arch-cli start-server
```

This command starts the development environment by:
1. Setting up a Bitcoin regtest network using Docker
2. Starting the Arch Network nodes
3. Creating and loading a Bitcoin wallet for testing

### Deploy a program

```sh
arch-cli deploy [--directory <path>] [--program-key <path>]
```

Compiles and deploys the specified program to the Arch Network. In regtest mode, it automatically:
- Ensures the Bitcoin wallet has funds
- Sends the required transaction
- Confirms the transaction by generating a new block

### Stop the development server

```sh
arch-cli stop-server
```

Gracefully shuts down the development server and Docker containers.

### Clean up the environment

```sh
arch-cli clean
```

Removes generated files and Docker volumes for a fresh start.

### Start Distributed Key Generation (DKG) process

```sh
arch-cli start-dkg
```

Initiates the Distributed Key Generation process on the Arch Network.

### Send coins (for testing)

```sh
arch-cli send-coins --address <address> --amount <amount>
```

Sends the specified amount of coins to the given address on the Bitcoin Regtest network.

### Start the frontend application

```sh
arch-cli start-app
```

This command prepares and starts the frontend application:
1. Copies the `.env.example` file to `.env` in the frontend directory
2. Installs all npm packages in the frontend directory
3. Builds and starts the Vite development server
4. Opens the application in the user's default web browser

The server will continue running until stopped with Ctrl+C.

## Getting Started with the Demo App

To quickly set up and run the demo application, follow these steps:

1. Initialize the project:
   ```
   arch-cli init
   ```
   This command sets up the basic project structure, creating necessary directories and files for your Arch Network application.

2. Start the development server:
   ```
   arch-cli start-server
   ```
   This starts the local development environment, including a Bitcoin regtest network and Arch Network nodes. It sets up the necessary blockchain infrastructure for your application.

3. Start the Distributed Key Generation (DKG) process:
   ```
   arch-cli start-dkg
   ```
   This initiates the DKG process on the Arch Network, which is crucial for setting up the decentralized key management system used by your application.

4. Deploy your application:
   ```
   arch-cli deploy
   ```
   This command compiles your Arch Network program and deploys it to the local Arch Network. It handles the necessary transactions and confirmations on the regtest network.

5. Start the frontend application:
   ```
   arch-cli start-app
   ```
   This prepares and launches the frontend of your application. It copies the environment file, installs dependencies, starts the development server, and opens the application in your default web browser.

By following these steps in order, you'll have a fully functional demo Arch Network application running locally. This setup allows you to interact with your application, test its functionality, and make changes as needed during development.

Remember to keep the terminal windows for steps 2 and 5 open, as they run ongoing processes (the blockchain environment and the frontend server, respectively). You can stop these processes using Ctrl+C when you're done working on your application.

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

- If you encounter issues with Docker networks, try running `arch-cli clean` to remove existing volumes and networks.
- Ensure your Docker daemon is running before using `arch-cli start-server`.
- Check the `config.toml` file for correct configuration of RPC endpoints and credentials.
- If you encounter issues with the DKG process, ensure that all nodes are properly configured and connected.
- For frontend issues, make sure all npm dependencies are correctly installed and that the `.env` file is properly set up.

## Support

If you encounter any problems or have any questions, please open an issue in the GitHub repository.

## Acknowledgments

- Arch Network team for the core infrastructure
- The Rust community for excellent tools and libraries
- Bitcoin Core and Electrs projects for Bitcoin node and Electrum server implementations