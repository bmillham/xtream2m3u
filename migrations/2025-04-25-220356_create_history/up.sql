-- Your SQL goes here

CREATE TABLE `history` (
  `id` INTEGER NOT NULL PRIMARY KEY,
  `channels_id` INTEGER NOT NULL REFERENCES channels(id),
  `changed` DATETIME DEFAULT current_timestamp,
  `change_type` TEXT NOT NULL
)
