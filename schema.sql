CREATE TABLE users (
    discord_id TEXT PRIMARY KEY,
    steam_id TEXT NOT NULL
) IF NOT EXISTS users;
