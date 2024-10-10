const express = require('express');
const { Pool } = require('pg');
const { ArchRpcClient } = require('arch-typescript-sdk');
const cors = require('cors');

const app = express();
app.use(cors());

const port = process.env.INDEXER_PORT || 3003;
const MAX_RETRIES = 10;
const RETRY_DELAY = 5000; // 5 seconds

const pool = new Pool({
  user: process.env.DB_USER,
  host: process.env.DB_HOST,
  database: process.env.DB_NAME,
  password: process.env.DB_PASSWORD,
  port: process.env.DB_PORT,
});

const archClient = new ArchRpcClient(process.env.ARCH_NODE_URL);

let currentBlockHeight = 0;

let syncStartTime = Date.now();
let lastBlockTime = Date.now();
let averageBlockTime = 0;

async function syncBlocks() {
  try {
    const isReady = await archClient.isNodeReady();
    if (!isReady) {
      console.log('Node is not ready, waiting...');
      setTimeout(syncBlocks, 5000);
      return;
    }

    const latestBlockHeight = await archClient.getBlockCount();
    console.log(`Current block height: ${currentBlockHeight}, Latest block height: ${latestBlockHeight}`);

    while (currentBlockHeight <= latestBlockHeight) {
      const blockStartTime = Date.now();
      try {
        const blockHash = await archClient.getBlockHash(currentBlockHeight);
        const block = await archClient.getBlock(blockHash);
        block.height = currentBlockHeight;
        block.hash = blockHash;
        await storeBlock(block);
        currentBlockHeight++;
        console.log(`Synced block at height ${currentBlockHeight}`);

        const blockTime = Date.now() - blockStartTime;
        averageBlockTime = averageBlockTime === 0 ? blockTime : (averageBlockTime * 0.9 + blockTime * 0.1);
        lastBlockTime = Date.now();
      } catch (error) {
        console.error(`Error processing block at height ${currentBlockHeight}:`, error);
        await new Promise(resolve => setTimeout(resolve, 5000)); // Wait 5 seconds before retrying
      }
    }

    // Determine the delay for the next sync attempt
    const delay = currentBlockHeight >= latestBlockHeight ? 1000 : 10000;
    setTimeout(syncBlocks, delay);
  } catch (error) {
    console.error('Error syncing blocks:', error);
    setTimeout(syncBlocks, 10000);
  }
}

async function storeBlock(block) {
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query(
      `INSERT INTO blocks (height, hash, timestamp, bitcoin_block_height) 
       VALUES ($1, $2, $3, $4) 
       ON CONFLICT (height) DO UPDATE 
       SET hash = EXCLUDED.hash, timestamp = EXCLUDED.timestamp, 
           bitcoin_block_height = EXCLUDED.bitcoin_block_height`,
      [block.height, block.hash, block.timestamp, block.bitcoin_block_height]
    );

    // Store the block even if there are no transactions
    console.log(`Stored block at height ${block.height}`);

    if (block.transactions && block.transactions.length > 0) {
      const transactionPromises = block.transactions.map(async (txId) => {
        const tx = await archClient.getProcessedTransaction(txId);
        return client.query(
          `INSERT INTO transactions (txid, block_height, data, status, bitcoin_txids) 
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (txid) DO UPDATE
           SET block_height = EXCLUDED.block_height, data = EXCLUDED.data, 
               status = EXCLUDED.status, bitcoin_txids = EXCLUDED.bitcoin_txids`,
          [
            txId,
            block.height,
            JSON.stringify(tx.runtime_transaction),
            tx.status === 'Processing' ? 0 : 1,
            tx.bitcoin_txids && tx.bitcoin_txids.length > 0 ? tx.bitcoin_txids : '{}'
          ]
        );
      });

      await Promise.all(transactionPromises);
      console.log(`Stored ${block.transactions.length} transactions for block at height ${block.height}`);
    } else {
      console.log(`No transactions to store for block at height ${block.height}`);
    }

    await client.query('COMMIT');
  } catch (error) {
    await client.query('ROLLBACK');
    throw error;
  } finally {
    client.release();
  }
}

  app.get('/', (req, res) => {
    res.json({ message: 'Arch Indexer API is running' });
  });
  
  app.get('/api/blocks', async (req, res) => {
    try {
      const { rows } = await pool.query('SELECT * FROM blocks ORDER BY height DESC LIMIT 200');
      res.json(rows);
    } catch (error) {
      console.error('Error fetching blocks:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });

  app.get('/api/blocks/:blockhash', async (req, res) => {
    const { blockhash } = req.params;
    try {
      const blockQuery = `
        SELECT b.*, 
               (SELECT hash FROM blocks WHERE height = b.height - 1) AS previous_block_hash
        FROM blocks b 
        WHERE b.hash = $1
      `;
      const blockResult = await pool.query(blockQuery, [blockhash]);
  
      if (blockResult.rows.length === 0) {
        return res.status(404).json({ error: 'Block not found' });
      }
  
      const block = blockResult.rows[0];
  
      const transactionsQuery = `
        SELECT txid 
        FROM transactions 
        WHERE block_height = $1
      `;
      const transactionsResult = await pool.query(transactionsQuery, [block.height]);
  
      const response = {
        ...block,
        transactions: transactionsResult.rows.map(row => row.txid)
      };
  
      res.json(response);
    } catch (error) {
      console.error('Error fetching block:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });
  
  app.get('/api/blocks/height/:height', async (req, res) => {
    const { height } = req.params;
    try {
      const blockQuery = `
        SELECT b.*, 
               (SELECT hash FROM blocks WHERE height = b.height - 1) AS previous_block_hash
        FROM blocks b 
        WHERE b.height = $1
      `;
      const blockResult = await pool.query(blockQuery, [height]);
  
      if (blockResult.rows.length === 0) {
        return res.status(404).json({ error: 'Block not found' });
      }
  
      const block = blockResult.rows[0];
  
      const transactionsQuery = `
        SELECT txid 
        FROM transactions 
        WHERE block_height = $1
      `;
      const transactionsResult = await pool.query(transactionsQuery, [height]);
  
      const response = {
        ...block,
        transactions: transactionsResult.rows.map(row => row.txid)
      };
  
      res.json(response);
    } catch (error) {
      console.error('Error fetching block by height:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });
  
  app.get('/api/transactions', async (req, res) => {
    try {
      const { rows } = await pool.query('SELECT * FROM transactions ORDER BY block_height DESC LIMIT 20');
      res.json(rows);
    } catch (error) {
      console.error('Error fetching transactions:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });

  app.get('/api/transactions/:txid', async (req, res) => {
    const { txid } = req.params;
    try {
      const { rows } = await pool.query('SELECT * FROM transactions WHERE txid = $1', [txid]);
      if (rows.length > 0) {
        res.json(rows[0]);
      } else {
        res.status(404).json({ error: 'Transaction not found' });
      }
    } catch (error) {
      console.error('Error fetching transaction:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });
  
  
  app.get('/api/sync-status', async (req, res) => {
    try {
      const latestBlockHeight = await archClient.getBlockCount();
      const percentageComplete = ((currentBlockHeight / latestBlockHeight) * 100).toFixed(2);
      
      let estimatedTimeToCompletion = 'N/A';
      if (averageBlockTime > 0) {
        const remainingBlocks = latestBlockHeight - currentBlockHeight;
        const estimatedSeconds = (remainingBlocks * averageBlockTime) / 1000;
        estimatedTimeToCompletion = formatTime(estimatedSeconds);
      }
  
      const elapsedTime = formatTime((Date.now() - syncStartTime) / 1000);
  
      res.json({
        currentBlockHeight,
        latestBlockHeight,
        percentageComplete: `${percentageComplete}%`,
        isSynced: currentBlockHeight >= latestBlockHeight,
        estimatedTimeToCompletion,
        elapsedTime,
        averageBlockTime: `${(averageBlockTime / 1000).toFixed(2)} seconds`
      });
    } catch (error) {
      console.error('Error fetching sync status:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });
  
  function formatTime(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const remainingSeconds = Math.floor(seconds % 60);
    return `${hours}h ${minutes}m ${remainingSeconds}s`;
  }
  
  async function connectWithRetry() {
    let retries = 0;
    while (retries < MAX_RETRIES) {
      try {
        await pool.connect();
        console.log('Successfully connected to the database');
        return;
      } catch (err) {
        retries++;
        console.error(`Failed to connect to the database. Attempt ${retries}/${MAX_RETRIES}. Retrying in ${RETRY_DELAY / 1000} seconds...`);
        await new Promise(resolve => setTimeout(resolve, RETRY_DELAY));
      }
    }
    console.error('Failed to connect to the database after maximum retries. Exiting...');
    process.exit(1);
  }

  // Initialize currentBlockHeight before starting the sync process
  app.listen(port, '0.0.0.0', async () => {
    console.log(`Indexer API listening at http://localhost:${port}`);
    try {
      await connectWithRetry();
      const result = await pool.query('SELECT MAX(height) as max_height FROM blocks');
      currentBlockHeight = result.rows[0].max_height || 0;
      console.log(`Starting sync from block height: ${currentBlockHeight}`);
      syncStartTime = Date.now(); // Reset sync start time
      syncBlocks();
    } catch (error) {
      console.error('Error initializing currentBlockHeight:', error);
      process.exit(1);
    }
  });

  app.get('/api/network-stats', async (req, res) => {
    try {
      // Get total transactions
      const { rows: txRows } = await pool.query('SELECT COUNT(*) as total_transactions FROM transactions');
      const totalTransactions = parseInt(txRows[0].total_transactions);
  
      // Get latest block height
      const latestBlockHeight = await archClient.getBlockCount();
  
      // Get latest slot height (assuming it's the same as block height for now)
      const slotHeight = latestBlockHeight;
  
      // Calculate TPS (transactions per second) over the last minute
      const oneMinuteAgo = Date.now() - 60000; // 1 minute ago
      const { rows: recentTxRows } = await pool.query(`
        SELECT COUNT(*) as recent_tx_count, 
               MIN(b.timestamp) as start_time,
               MAX(b.timestamp) as end_time
        FROM transactions t
        JOIN blocks b ON t.block_height = b.height
        WHERE b.timestamp > $1
      `, [oneMinuteAgo]);
      
      const recentTxCount = parseInt(recentTxRows[0].recent_tx_count);
      const startTime = new Date(recentTxRows[0].start_time).getTime();
      const endTime = new Date(recentTxRows[0].end_time).getTime();
      const timeSpanSeconds = (endTime - startTime) / 1000;
      
      const tps = timeSpanSeconds > 0 ? recentTxCount / timeSpanSeconds : 0;
  
      // Calculate true TPS (average over the last 100 blocks)
      const { rows: last100BlocksRows } = await pool.query(`
        SELECT COUNT(*) as tx_count,
               MIN(timestamp) as start_time,
               MAX(timestamp) as end_time
        FROM (
          SELECT b.timestamp, t.txid
          FROM blocks b
          LEFT JOIN transactions t ON b.height = t.block_height
          WHERE b.height > (SELECT MAX(height) - 100 FROM blocks)
        ) as recent_data
      `);
  
      const last100BlocksTxCount = parseInt(last100BlocksRows[0].tx_count);
      const last100BlocksStartTime = new Date(last100BlocksRows[0].start_time).getTime();
      const last100BlocksEndTime = new Date(last100BlocksRows[0].end_time).getTime();
      const last100BlocksTimeSpanSeconds = (last100BlocksEndTime - last100BlocksStartTime) / 1000;
  
      const trueTps = last100BlocksTimeSpanSeconds > 0 ? last100BlocksTxCount / last100BlocksTimeSpanSeconds : 0;
  
      res.json({
        totalTransactions,
        blockHeight: latestBlockHeight,
        slotHeight,
        tps: tps.toFixed(2),
        trueTps: trueTps.toFixed(2)
      });
    } catch (error) {
      console.error('Error fetching network stats:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });

  app.get('/api/search', async (req, res) => {
    const { term } = req.query;
    try {
      // Check if the term is a transaction ID
      const { rows: txRows } = await pool.query('SELECT * FROM transactions WHERE txid = $1', [term]);
      if (txRows.length > 0) {
        res.json({ type: 'transaction', data: txRows[0] });
        return;
      }
  
      // Check if the term is a block hash
      const { rows: blockRows } = await pool.query('SELECT * FROM blocks WHERE hash = $1', [term]);
      if (blockRows.length > 0) {
        res.json({ type: 'block', data: blockRows[0] });
        return;
      }
  
      // Check if the term is a block height
      const height = parseInt(term);
      if (!isNaN(height)) {
        const { rows: heightRows } = await pool.query('SELECT * FROM blocks WHERE height = $1', [height]);
        if (heightRows.length > 0) {
          res.json({ type: 'block', data: heightRows[0] });
          return;
        }
      }
  
      res.status(404).json({ error: 'No matching transaction or block found' });
    } catch (error) {
      console.error('Error searching:', error);
      res.status(500).json({ error: 'Internal server error' });
    }
  });