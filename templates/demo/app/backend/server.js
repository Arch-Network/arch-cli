const express = require('express');
const { ArchRpcClient } = require('arch-typescript-sdk');
const cors = require('cors');
require('dotenv').config();

const app = express();
app.use(cors());
app.use(express.json());

const client = new ArchRpcClient(process.env.ARCH_NODE_URL || 'http://localhost:9002');
const PRIVATE_KEY = process.env.ARCH_PRIVATE_KEY;

app.post('/add-to-wall', async (req, res) => {
  try {
    const { programPubkey, name, message } = req.body;
    const privateKeyBytes = new Uint8Array(PRIVATE_KEY.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));

    const encoder = new TextEncoder();
    const nameBytes = encoder.encode(name.slice(0, 16).padEnd(16, '\0')).slice(0, 16);
    const messageBytes = encoder.encode(message).slice(0, 64);

    const instructionData = new Uint8Array(80);
    instructionData.set(nameBytes, 0);
    instructionData.set(messageBytes, 16);

    const result = await client.callProgram(privateKeyBytes, programPubkey, Array.from(instructionData));
    res.json({ success: true, result });
  } catch (error) {
    console.error('Error adding to wall:', error);
    res.status(500).json({ success: false, error: error.message });
  }
});

const PORT = process.env.PORT || 5174;
app.listen(PORT, () => console.log(`Server running on port ${PORT}`));