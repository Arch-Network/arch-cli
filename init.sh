#!/bin/bash
REPLICA_COUNT=${REPLICA_COUNT:-2}
echo "Generating validator PeerIDs..."
echo "Replicas: ${REPLICA_COUNT}"

# Clear the whitelist before writing to it
true >/bootnode_data/validator_whitelist

# If validator binary exists move it to bin
if [ -f ./validator ]; then
	mv ./validator /usr/local/bin/validator
fi

for i in $(seq 1 "${REPLICA_COUNT}"); do
	VALIDATOR_PATH="/arch_data/validators/validator_${i}"
	mkdir -p "${VALIDATOR_PATH}"	
	PEER_ID=$(validator -d "${VALIDATOR_PATH}" --generate-peer-id)
	echo "${PEER_ID}" | tee -a /bootnode_data/validator_whitelist
	echo "${PEER_ID}" >"/arch_data/validators/validator_${i}/peer_id"
done

LEADER_PATH="/arch_data/leader"
mkdir -p "$LEADER_PATH"
LEADER_PEER_ID=$(validator -d "$LEADER_PATH" --generate-peer-id)
echo "$LEADER_PEER_ID" | tee /bootnode_data/leader_peer_id
echo "$LEADER_PEER_ID" | tee -a /bootnode_data/validator_whitelist
echo "$LEADER_PEER_ID" >"/arch_data/leader/peer_id"

BOOTNODE_PATH="/bootnode_data"
mkdir -p "$BOOTNODE_PATH"
BOOTNODE_PEER_ID=$(validator -d "$BOOTNODE_PATH" --generate-peer-id)
echo "$BOOTNODE_PEER_ID" | tee /bootnode_data/bootnode_peer_id
echo 'Init complete' >/bootnode_data/init_complete
