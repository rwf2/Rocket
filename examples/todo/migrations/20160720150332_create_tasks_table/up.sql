CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    description VARCHAR NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT FALSE
);

INSERT INTO tasks (description) VALUES ('demo task');
INSERT INTO tasks (description) VALUES ('demo task2');
