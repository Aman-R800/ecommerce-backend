-- Your SQL goes here
CREATE TABLE users(
    user_id uuid PRIMARY KEY,
    name text NOT NULL,
    email text UNIQUE NOT NULL,
    password text NOT NULL
);
