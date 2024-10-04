-- Your SQL goes here
CREATE TABLE order_items (
    order_item_id uuid PRIMARY KEY,
    order_id uuid NOT NULL,
    item_id uuid NOT NULL,
    quantity integer NOT NULL CHECK (quantity > 0),
    FOREIGN KEY (order_id) REFERENCES orders(order_id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES inventory(item_id)
);
