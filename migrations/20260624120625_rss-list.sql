-- Add migration script here

ALTER TABLE item DROP COLUMN link;
ALTER TABLE item DROP COLUMN link_type;

ALTER TABLE item ADD added_through int not null default 0;
ALTER TABLE list ADD rss_source text;


