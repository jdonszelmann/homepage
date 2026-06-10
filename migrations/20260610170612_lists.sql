-- Add migration script here

create table "list" (
    "id" text not null primary key,
    "name" text not null,

    "added" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updated" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "deleted" TIMESTAMP
);

create table "item" (
    "id" text not null primary key,
    "list" text not null references list (id),

    "note" text not null,

    -- canonically associated link
    "link" text not null,
    -- the type of link, if relevant, to be displayed nicely
    "link_type" text not null,

    "added" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "updated" TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    "deleted" TIMESTAMP
);

CREATE INDEX list_id ON list("id");
CREATE INDEX item_id ON item("id");
CREATE INDEX item_list ON item("list");
