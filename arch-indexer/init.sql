CREATE TABLE IF NOT EXISTS blocks (
  height INTEGER PRIMARY KEY,
  hash TEXT NOT NULL,
  timestamp BIGINT NOT NULL,
  bitcoin_block_height INTEGER
);

CREATE TABLE IF NOT EXISTS transactions (
  txid TEXT PRIMARY KEY,
  block_height INTEGER NOT NULL,
  data JSONB NOT NULL,
  status INTEGER NOT NULL DEFAULT 0,
  bitcoin_txids TEXT[] DEFAULT '{}',
  FOREIGN KEY (block_height) REFERENCES blocks(height)
);

CREATE INDEX IF NOT EXISTS idx_transactions_block_height ON transactions(block_height);
CREATE INDEX IF NOT EXISTS idx_blocks_bitcoin_block_height ON blocks(bitcoin_block_height);