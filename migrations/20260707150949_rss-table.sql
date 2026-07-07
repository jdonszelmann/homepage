-- Add migration script here

ALTER TABLE list DROP COLUMN rss_source;

create table if not exists rss
(
    "id" text primary key not null,
    "url" text not null,
    "list" uuid not null references list (id),

    "added" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updated" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "deleted" TIMESTAMP
);

ALTER TABLE item ADD rss_guid text;
