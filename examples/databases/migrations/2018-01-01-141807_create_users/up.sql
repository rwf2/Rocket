CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  username VARCHAR UNIQUE NOT NULL
);

CREATE UNIQUE INDEX username_idx ON users(username);

INSERT INTO users(username) VALUES('postgresql');