-- Your SQL goes here
CREATE TABLE confirmation(
    confirmation_id uuid PRIMARY KEY,
    user_id uuid,
    FOREIGN KEY(user_id) REFERENCES users(user_id)
);
