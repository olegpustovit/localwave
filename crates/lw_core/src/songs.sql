CREATE TABLE IF NOT EXISTS songs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    songKey INTEGER NOT NULL, artist TEXT NOT NULL, title TEXT NOT NULL,
    album TEXT NOT NULL, albumArtist TEXT NOT NULL, releaseYear INTEGER,
    discNumber INTEGER, trackNumber INTEGER, coverArtPath TEXT, bookmark BLOB,
    pathHash INTEGER NOT NULL, createdAt REAL NOT NULL, updatedAt REAL,
    localFilePath TEXT, fileState INTEGER NOT NULL DEFAULT 0,
    isFavorite INTEGER NOT NULL DEFAULT 0, sourceId INTEGER, relativePath TEXT,
    duration REAL, fileSize INTEGER, fileExtension TEXT, sampleRate REAL,
    bitDepth INTEGER, sourceAvailability TEXT
);
CREATE VIRTUAL TABLE IF NOT EXISTS songs_fts USING fts5(
    songId UNINDEXED, artist, title, album, albumArtist, tokenize='unicode61'
);
