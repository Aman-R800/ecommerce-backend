-- This file should undo anything in `up.sql`
ALTER TABLE users
DROP COLUMN phone_number,
DROP COLUMN address; 
