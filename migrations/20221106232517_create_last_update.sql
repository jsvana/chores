CREATE TABLE updates (
  update_timestamp TIMESTAMP NOT NULL DEFAULT (STRFTIME('%s', 'now', 'localtime')),
  PRIMARY KEY (update_timestamp)
);
