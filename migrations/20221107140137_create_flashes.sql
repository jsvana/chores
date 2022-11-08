CREATE TABLE flashes (
  id INTEGER NOT NULL PRIMARY KEY,
  contents TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT (STRFTIME('%s', 'now')),
  acknowledged INTEGER NOT NULL DEFAULT 0,
  acknowledged_at TIMESTAMP
);
