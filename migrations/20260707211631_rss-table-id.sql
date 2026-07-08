-- Add migration script here

ALTER TABLE rss DROP COLUMN id;
ALTER TABLE rss ADD id uuid not null primary key;
