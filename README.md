# Arch-CLI

Arch-CLI is a command-line interface tool designed to streamline the development process for Arch Network applications. It provides a set of commands to initialize, manage, and deploy Arch Network programs efficiently.

## Features

- Easy project initialization
- Development server management
- Simplified deployment process
- Integration with the Arch Network ecosystem

## Installation

To install Arch-CLI, make sure you have Rust, Solana-CLI and Cargo installed on your system. Then, run:

```sh
git clone https://github.com/hoffmabc/arch-cli.git
cd arch-cli
cargo install --path .
```

TODO:

```sh
cargo install --git https://github.com/hoffmabc/arch-cli.git
```

## Usage

Here are the main commands available in Arch-CLI:

### Initialize a new project

```sh
arch-cli init
```

This command sets up a new Arch Network project with the necessary folder structure and boilerplate code. 

### Start the development server

For now just start the servers on your own. Still working on this.

```sh
arch-cli start-server
```

Launches the development environment using the `start-server.sh` script.

### Deploy a program

This will compile your program and deploy it to the Arch Network network.

```sh
arch-cli deploy
```

Compiles and deploys the specified program to the Arch Network network.

### Stop the development server

```sh
arch-cli stop-server
```

Gracefully shuts down the development server.

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
│       └── frontend/
│       |   ├── index.html
│       |   ├── index.js
│       |   └── package.json
|       |── keys/
└── Cargo.toml
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


## Acknowledgments

- Arch Network team for the core infrastructure
- The Rust community for excellent tools and libraries

## Support

If you encounter any problems or have any questions, please open an issue in the GitHub repository.
