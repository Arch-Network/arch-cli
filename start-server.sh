#!/bin/bash

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo "Usage: $0 <number_of_validators> [debug]"
    exit 1
fi

# rm -rf validator/.arch-data

cargo build
NUM_VALIDATORS=$1
DEBUG_MODE=$2

# Constants
PEER_ID_FILENAME="peer_id"
BOOTNODE_DATA_DIR="./.arch-bootnode-data"
LEADER_DATA_DIR="./.arch-data/arch-validator-data-0"
INITIAL_RPC_PORT=9001
INITIAL_P2P_PORT=19001
INITIAL_MONITOR_PORT=8080

if [ -z "$DEBUG_MODE" ]; then
    RUST_LOG_LEVEL="INFO"
elif [ "$DEBUG_MODE" == "debug" ]; then
    RUST_LOG_LEVEL="DEBUG"
elif [ "$DEBUG_MODE" == "info" ]; then
    RUST_LOG_LEVEL="INFO"
else
    echo "Invalid debug mode. Use 'info' or 'debug'."
    exit 1
fi

SESSION_NAME="gossip-development-server"

LEADER_PEERID=$(cd validator && cargo run -- --data-dir $LEADER_DATA_DIR --generate-peer-id)

tmux new-session -d -s $SESSION_NAME

# Bootnode
tmux send-keys -t $SESSION_NAME "cd bootnode && RUST_LOG=$RUST_LOG_LEVEL cargo run -- --data-dir $BOOTNODE_DATA_DIR --network-mode localnet --p2p-bind-port $INITIAL_P2P_PORT --leader-peer-id $LEADER_PEERID" C-m

tmux rename-window -t $SESSION_NAME:0 bootnode

while ! nc -z 127.0.0.1 $INITIAL_P2P_PORT; do
    sleep 1
done

BOOTNODE_PEERID=$(cat "./bootnode/$BOOTNODE_DATA_DIR/$PEER_ID_FILENAME")

for (( i=1; i<NUM_VALIDATORS; i++ ))
do
    P2P_PORT=$((INITIAL_P2P_PORT + 10000 + i))
    RPC_PORT=$((INITIAL_RPC_PORT + 1 + i))
    MONITOR_PORT=$((INITIAL_MONITOR_PORT + i))
    DATA_DIR="./.arch-data/arch-validator-data-$i"

    tmux new-window -t $SESSION_NAME -n "Validator-$i"
    tmux send-keys -t $SESSION_NAME "cd validator && RUST_LOG=$RUST_LOG_LEVEL cargo run -- --data-dir $DATA_DIR --boot-node-endpoint /ip4/127.0.0.1/tcp/$INITIAL_P2P_PORT/p2p/$BOOTNODE_PEERID --network-mode localnet --rpc-bind-ip 127.0.0.1 --rpc-bind-port $RPC_PORT --p2p-bind-port $P2P_PORT --monitor-bind-ip 127.0.0.1 --monitor-bind-port $MONITOR_PORT --bitcoin-rpc-endpoint bitcoin-node.dev.aws.archnetwork.xyz --bitcoin-rpc-port 18443 --bitcoin-rpc-username bitcoin --bitcoin-rpc-password 428bae8f3c94f8c39c50757fc89c39bc7e6ebc70ebf8f618" C-m
done

# This is a workaround while DKG is lacking synchronization primitives
sleep 3

tmux new-window -t $SESSION_NAME -n "LEADER"
tmux send-keys -t $SESSION_NAME "cd validator && RUST_LOG=$RUST_LOG_LEVEL cargo run -- --data-dir $LEADER_DATA_DIR --boot-node-endpoint /ip4/127.0.0.1/tcp/$INITIAL_P2P_PORT/p2p/$BOOTNODE_PEERID --network-mode localnet --rpc-bind-ip 127.0.0.1 --rpc-bind-port $((INITIAL_RPC_PORT + 1)) --p2p-bind-port $((INITIAL_P2P_PORT + 10000)) --monitor-bind-ip 127.0.0.1 --monitor-bind-port $INITIAL_MONITOR_PORT --bitcoin-rpc-endpoint bitcoin-node.dev.aws.archnetwork.xyz --bitcoin-rpc-port 18443 --bitcoin-rpc-username bitcoin --bitcoin-rpc-password 428bae8f3c94f8c39c50757fc89c39bc7e6ebc70ebf8f618" C-m

while ! nc -z 127.0.0.1 $((INITIAL_RPC_PORT + 1)); do
    sleep 1
done

tmux attach -t $SESSION_NAME
