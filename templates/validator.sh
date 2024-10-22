#!/bin/bash
# Wait for bootnode initialization to complete
while [ ! -f /bootnode_data/init_complete ]; do
  echo 'Waiting for bootnode initialization to complete...'
  sleep 5
done

sleep $((15 + RANDOM % 20 + 1))

# If validator binary exists move it to bin
if [ -f ./validator ]; then
    mv ./validator /usr/local/bin/validator
fi
# Set the bootnode peer ID
BOOTNODE_PEERID=$(cat /bootnode_data/peer_id)
echo "Bootnode p2p port: $BOOTNODE_P2P_PORT"
# Run the validator with the bootnode peer ID and Bitcoin RPC details
echo "Will run this command: validator -d /arch_data/validator_${VALIDATOR_NUMBER} -n ${NETWORK_MODE:-localnet} --boot-node-endpoint "/ip4/172.30.0.250/tcp/${BOOTNODE_P2P_PORT}/p2p/$BOOTNODE_PEERID" --rpc-bind-ip 127.0.0.1 --rpc-bind-port $RPC_BIND_PORT --p2p-bind-port $P2P_BIND_PORT --bitcoin-rpc-endpoint $BITCOIN_RPC_HOSTNAME --bitcoin-rpc-port $BITCOIN_RPC_PORT --bitcoin-rpc-username $BITCOIN_RPC_USERNAME --bitcoin-rpc-password $BITCOIN_RPC_PASSWORD"
validator -d /arch_data/validator_${VALIDATOR_NUMBER} -n ${NETWORK_MODE:-localnet} -b "/ip4/172.30.0.250/tcp/${BOOTNODE_P2P_PORT}/p2p/$BOOTNODE_PEERID" --rpc-bind-ip 127.0.0.1 --rpc-bind-port $RPC_BIND_PORT --p2p-bind-port $P2P_BIND_PORT --bitcoin-rpc-endpoint "${BITCOIN_RPC_ENDPOINT}" --bitcoin-rpc-port "${BITCOIN_RPC_PORT}" --bitcoin-rpc-username "${BITCOIN_RPC_USERNAME}" --bitcoin-rpc-password "${BITCOIN_RPC_PASSWORD}"