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
BOOTNODE_PEERID=$(cat /bootnode_data/bootnode_peer_id)
MONITOR_PORT=8080
echo 'Contents of validator_whitelist:';
cat /bootnode_data/validator_whitelist;
WHITELIST=$(cat /bootnode_data/validator_whitelist | tr '\n' ',' | sed 's/,$//');
echo 'WHITELIST value:';
echo $WHITELIST;
# If bootnode exists move it to bin
if [ -f ./bootnode ]; then
    mv ./bootnode /usr/local/bin/bootnode
fi
if [ -z $WHITELIST ]; then
  echo 'Error: WHITELIST is empty';
  exit 1;
fi;
echo "About to run this command: validator -d /arch_data/leader -n ${NETWORK_MODE:-localnet} -b "/ip4/172.30.0.250/tcp/${BOOTNODE_P2P_PORT}/p2p/$BOOTNODE_PEERID" --rpc-bind-port $RPC_BIND_PORT --p2p-bind-port $P2P_BIND_PORT --monitor-bind-ip 127.0.0.1 --monitor-bind-port $MONITOR_PORT --bitcoin-rpc-endpoint $BITCOIN_RPC_ENDPOINT --bitcoin-rpc-port $BITCOIN_RPC_PORT --bitcoin-rpc-username $BITCOIN_RPC_USERNAME --bitcoin-rpc-password $BITCOIN_RPC_PASSWORD"
# Run the leader node with the bootnode peer ID and Bitcoin RPC details
validator -d /arch_data/leader -n "${NETWORK_MODE:-localnet}" -b "/ip4/172.30.0.250/tcp/${BOOTNODE_P2P_PORT}/p2p/${BOOTNODE_PEERID}" --rpc-bind-port "${RPC_BIND_PORT}" --p2p-bind-port "${P2P_BIND_PORT}" --monitor-bind-ip 127.0.0.1 --monitor-bind-port "${MONITOR_PORT}" --bitcoin-rpc-endpoint "${BITCOIN_RPC_ENDPOINT}" --bitcoin-rpc-port "${BITCOIN_RPC_PORT}" --bitcoin-rpc-username "${BITCOIN_RPC_USERNAME}" --bitcoin-rpc-password "${BITCOIN_RPC_PASSWORD}"