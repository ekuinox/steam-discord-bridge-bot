DROP TABLE IF EXISTS users;
CREATE TABLE users (
    discord_id TEXT PRIMARY KEY,
    steam_id TEXT NOT NULL
);