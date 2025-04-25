-- Your SQL goes here
CREATE TABLE `channels` (
  `id` INTEGER NOT NULL PRIMARY KEY,
  `categories_id` INTEGER NOT NULL REFERENCES categories(id),
  `name` TEXT NOT NULL UNIQUE,
  `added` DATETIME DEFAULT current_timestamp,
  `deleted` DATETIME
)
