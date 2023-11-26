CREATE TABLE IF NOT EXISTS magic_tokens (
    magic_token_hash VARCHAR(64) PRIMARY KEY NOT NULL,
    client_id VARCHAR(64) NOT NULL,
    expires DATETIME DEFAULT NULL,
    created DATETIME NOT NULL DEFAULT NOW(),
    updated DATETIME NOT NULL DEFAULT NOW()
);
