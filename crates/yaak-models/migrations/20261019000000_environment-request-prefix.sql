ALTER TABLE environments
    ADD COLUMN request_prefix TEXT DEFAULT '' NOT NULL;
