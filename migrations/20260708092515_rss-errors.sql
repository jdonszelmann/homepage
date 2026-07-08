-- Add migration script here

CREATE INDEX rss_id ON rss("id");
CREATE INDEX rss_list ON rss("list");

ALTER TABLE rss ADD last_error text;
