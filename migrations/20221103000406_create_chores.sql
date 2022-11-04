CREATE TABLE chores (
  title TEXT NOT NULL,
  expected_completion_time TIMESTAMP NOT NULL,
  status TEXT CHECK(status IN ('assigned', 'completed', 'missed')) NOT NULL DEFAULT 'assigned',
  created_at TIMESTAMP NOT NULL DEFAULT (STRFTIME('%s', 'now', 'localtime')),
  overdue_time TIMESTAMP NOT NULL,
  PRIMARY KEY (title, expected_completion_time)
);
