-- Your SQL goes here
CREATE TABLE inventory(
    item_id uuid,
    name text NOT NULL,
    amount integer,
    price float,
    PRIMARY KEY(item_id),
    CHECK (amount >= 0)
);
