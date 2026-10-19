ALTER TABLE websocket_requests
    ADD COLUMN protocol TEXT DEFAULT 'websocket' NOT NULL;
ALTER TABLE websocket_requests
    ADD COLUMN stomp_destination TEXT DEFAULT '' NOT NULL;
ALTER TABLE websocket_requests
    ADD COLUMN stomp_subscriptions TEXT DEFAULT '[]' NOT NULL;
