-- Your SQL goes here
CREATE TABLE orders(
    order_id uuid,
    user_id uuid,
    order_date timestamptz,
    status text NOT NULL DEFAULT 'pending',
    PRIMARY KEY(order_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id),
    CHECK (status IN ('pending', 'shipped', 'delivered'))
);
