-- Your SQL goes here
CREATE TABLE `categories` (
  `id` INTEGER NOT NULL PRIMARY KEY,
  `types_id` INTEGER NOT NULL REFERENCES types(id),
  `name` TEXT NOT NULL UNIQUE,
  `added` DATETIME DEFAULT current_timestamp
)
  
