#!/bin/bash

# Wait for bootnode initialization to complete
while [ ! -f /bootnode_data/init_complete ]; do
  echo 'Waiting for bootnode initialization to complete...'
  sleep 5
done

# If validator binary exists move it to bin
if [ -f ./validator ]; then
	mv ./validator /usr/local/bin/validator
fi

# Set the bootnode peer ID
BOOTNODE_PEERID=$(cat /bootnode_data/peer_id)
MONITOR_PORT=8080

echo "About to run this command: validator -d /arch_data/leader -n ${NETWORK_MODE:-localnet} -b "/ip4/172.30.0.250/tcp/${BOOTNODE_P2P_PORT}/p2p/$BOOTNODE_PEERID" --rpc-bind-port $RPC_BIND_PORT --p2p-bind-port $P2P_BIND_PORT --monitor-bind-ip 127.0.0.1 --monitor-bind-port $MONITOR_PORT"

# Run the leader node with the bootnode peer ID
validator -d /arch_data/leader -n ${NETWORK_MODE:-localnet} -b "/ip4/172.30.0.250/tcp/${BOOTNODE_P2P_PORT}/p2p/$BOOTNODE_PEERID"  --rpc-bind-port $RPC_BIND_PORT --p2p-bind-port $P2P_BIND_PORT --monitor-bind-ip 127.0.0.1 --monitor-bind-port $MONITOR_PORT
