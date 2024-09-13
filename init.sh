#!/bin/bash
set -e  # Exit immediately if a command exits with a non-zero status
set -x  # Print commands and their arguments as they are executed

REPLICA_COUNT=${REPLICA_COUNT:-2}
echo "Generating validator PeerIDs..."
echo "Replicas: ${REPLICA_COUNT}"

# Clear the whitelist before writing to it
true >/bootnode_data/validator_whitelist

# Check if validator binary exists and is executable
if [ -f ./validator ]; then
    echo "Moving validator binary to /usr/local/bin"
    mv ./validator /usr/local/bin/validator
    chmod +x /usr/local/bin/validator
else
    echo "Validator binary not found in current directory"
fi

# Check if validator is in PATH
if ! command -v validator &> /dev/null; then
    echo "Error: validator command not found in PATH"
    exit 1
fi

# Generate validator peer IDs
for i in $(seq 1 "${REPLICA_COUNT}"); do
    VALIDATOR_PATH="/arch_data/validators/validator_${i}"
    mkdir -p "${VALIDATOR_PATH}"
    echo "Generating peer ID for validator ${i}"
    PEER_ID=$(validator -d "${VALIDATOR_PATH}" --generate-peer-id)
    if [ $? -ne 0 ]; then
        echo "Error: Failed to generate peer ID for validator ${i}"
        exit 1
    fi
    echo "Validator ${i} peer ID: ${PEER_ID}"
    echo "${PEER_ID}" | tee -a /bootnode_data/validator_whitelist
    echo "${PEER_ID}" >"/arch_data/validators/validator_${i}/peer_id"
done

# Generate leader peer ID
LEADER_PATH="/arch_data/leader"
mkdir -p "$LEADER_PATH"
echo "Generating leader peer ID"
LEADER_PEER_ID=$(validator -d "$LEADER_PATH" --generate-peer-id)
if [ $? -ne 0 ]; then
    echo "Error: Failed to generate leader peer ID"
    exit 1
fi
echo "Leader peer ID: ${LEADER_PEER_ID}"
echo "$LEADER_PEER_ID" | tee /bootnode_data/leader_peer_id
echo "$LEADER_PEER_ID" | tee -a /bootnode_data/validator_whitelist
echo "$LEADER_PEER_ID" >"/arch_data/leader/peer_id"

# Generate bootnode peer ID
BOOTNODE_PATH="/bootnode_data"
mkdir -p "$BOOTNODE_PATH"
echo "Generating bootnode peer ID"
BOOTNODE_PEER_ID=$(validator -d "$BOOTNODE_PATH" --generate-peer-id)
if [ $? -ne 0 ]; then
    echo "Error: Failed to generate bootnode peer ID"
    exit 1
fi
echo "Bootnode peer ID: ${BOOTNODE_PEER_ID}"
echo "$BOOTNODE_PEER_ID" | tee /bootnode_data/bootnode_peer_id

# Print contents of important files for verification
echo "Contents of /bootnode_data/validator_whitelist:"
cat /bootnode_data/validator_whitelist

echo "Contents of /bootnode_data/leader_peer_id:"
cat /bootnode_data/leader_peer_id

echo "Contents of /bootnode_data/bootnode_peer_id:"
cat /bootnode_data/bootnode_peer_id

echo 'Init complete' >/bootnode_data/init_complete
echo "Initialization completed successfully"