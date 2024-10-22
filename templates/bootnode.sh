#!/bin/bash

sleep 5;
while [ ! -f /arch_data/init_complete ]; do
  echo 'Waiting for init to complete...';
  sleep 5;
done;

echo 'Contents of validator_whitelist:';
cat /arch_data/validator_whitelist;
WHITELIST=$(cat /arch_data/validator_whitelist | tr '\n' ',' | sed 's/,$//');
LEADER_PEERID=$(cat /arch_data/leader_peer_id);

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

echo 'Bootnode command:';
echo RUSTLOG=debug bootnode -n ${NETWORK_MODE:-localnet} --leader-peer-id $LEADER_PEERID --validator-whitelist=$WHITELIST;

RUST_LOG=debug bootnode --p2p-bind-port $BOOTNODE_P2P_PORT -n ${NETWORK_MODE:-localnet} --leader-peer-id $LEADER_PEERID --validator-whitelist=$WHITELIST;
