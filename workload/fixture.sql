.bail on

PRAGMA page_size = 4096;
PRAGMA journal_mode = OFF;
PRAGMA synchronous = OFF;
PRAGMA temp_store = MEMORY;

CREATE TABLE customer (
  id INTEGER PRIMARY KEY,
  region TEXT NOT NULL,
  status TEXT NOT NULL,
  name TEXT NOT NULL
);

CREATE TABLE purchase (
  id INTEGER PRIMARY KEY,
  customer_id INTEGER NOT NULL REFERENCES customer(id),
  day INTEGER NOT NULL,
  category TEXT NOT NULL,
  amount_cents INTEGER NOT NULL,
  payload TEXT NOT NULL
);

WITH RECURSIVE seq(n) AS (
  VALUES(1)
  UNION ALL
  SELECT n + 1 FROM seq WHERE n < 5000
)
INSERT INTO customer(id, region, status, name)
SELECT
  n,
  CASE n % 5
    WHEN 0 THEN 'north'
    WHEN 1 THEN 'south'
    WHEN 2 THEN 'east'
    WHEN 3 THEN 'west'
    ELSE 'central'
  END,
  CASE WHEN n % 7 = 0 THEN 'inactive' ELSE 'active' END,
  printf('customer-%05d', n)
FROM seq;

WITH RECURSIVE seq(n) AS (
  VALUES(1)
  UNION ALL
  SELECT n + 1 FROM seq WHERE n < 50000
)
INSERT INTO purchase(id, customer_id, day, category, amount_cents, payload)
SELECT
  n,
  ((n * 7919) % 5000) + 1,
  (n * 37) % 730,
  CASE n % 6
    WHEN 0 THEN 'books'
    WHEN 1 THEN 'food'
    WHEN 2 THEN 'hardware'
    WHEN 3 THEN 'music'
    WHEN 4 THEN 'software'
    ELSE 'travel'
  END,
  ((n * 104729) % 250000) + 99,
  printf('order=%06d;bucket=%03d;deterministic-payload', n, n % 257)
FROM seq;

CREATE INDEX purchase_customer_day ON purchase(customer_id, day);
CREATE INDEX purchase_day_category ON purchase(day, category);
CREATE INDEX purchase_category_amount ON purchase(category, amount_cents);

ANALYZE;
VACUUM;
PRAGMA integrity_check;
