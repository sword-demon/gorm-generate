CREATE TABLE `logs` (
  `msg` VARCHAR(200) NOT NULL,
  `created_at` DATETIME NOT NULL
) COMMENT='Append-only log table';
