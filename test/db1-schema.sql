CREATE TABLE users (id BIGINT, name TEXT);
CREATE TABLE posts (id BIGINT, title TEXT, author BIGINT REFERENCES users(id));
