-- Add migration script here

create table if not exists session
(
    id text primary key not null,
    data jsonb not null,
    expiry_date timestamptz not null
)
