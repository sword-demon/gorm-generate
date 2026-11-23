CREATE TABLE users (
  id BIGSERIAL PRIMARY KEY,
  username VARCHAR(64) NOT NULL,
  email VARCHAR(128) NOT NULL,
  age SMALLINT,
  balance NUMERIC(12,2) NOT NULL,
  avatar_url VARCHAR(255),
  settings JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ,
  deleted_at TIMESTAMPTZ
);
COMMENT ON TABLE users IS 'user table';
COMMENT ON COLUMN users.username IS 'login name';
