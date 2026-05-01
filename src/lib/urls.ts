
import {database as db} from "../auth.ts";
import { v4 as uuidv4 } from "uuid";

export type LinkId = string;

export interface Link {
    id: LinkId,
    link: string,
    note: string,
    added: Date,
    updated: Date,
    tags: string[],
    hidden: boolean,
}

export function addLink(link: string, note: string, tags: string[]): LinkId {
    db.transaction(() => {
        const links_q = db.prepare('INSERT INTO links VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 0)');
        const tags_q = db.prepare('INSERT INTO tags VALUES (?, ?)');

        const id = uuidv4();

        links_q.run(id, link, note);

        for (const tag of tags) {
            tags_q.run(tag, id)
        }

        return id
    })()
}

export function removeLink(id: LinkId) {
    db.transaction(() => {
        db.prepare('DELETE FROM links where id = ?').run(id);
    })()
}

export function setNote(id: LinkId, note: string) {
    db.transaction(() => {
        db.prepare('UPDATE links SET note = ?, updated=CURRENT_TIMESTAMP WHERE id = ?').run(note, id);
    })()
}

export function hide(id: LinkId) {
    db.transaction(() => {
        db.prepare('UPDATE links SET hidden = 0 WHERE id = ?').run(id);
    })()
}

export function show(id: LinkId) {
    db.transaction(() => {
        db.prepare('UPDATE links SET hidden = 1 WHERE id = ?').run(id);
    })()
}

export function addTag(id: LinkId, tag: string) {
    db.transaction(() => {
        db.prepare('INSERT INTO tags VALUES (?, ?)').run(tag, id);
        db.prepare('UPDATE links SET updated=CURRENT_TIMESTAMP WHERE id = ?').run(id);
    })()
}


export function removeTag(id: LinkId, tag: string) {
    db.transaction(() => {
        db.prepare('DELETE FROM tags where name = ? and link = ?').run(tag, id);
        db.prepare('UPDATE links SET updated=CURRENT_TIMESTAMP WHERE id = ?').run(id);
    })()
}

export function getLinks(limit: number = 100_000, also_hidden: boolean = false): Link[] {
    return db.transaction(() => {
        let links;
        if (also_hidden) {
            links = db.prepare('SELECT * FROM links LIMIT ?');
        } else {
            links = db.prepare('SELECT * FROM links WHERE hidden is 0 LIMIT ?');
        }

        const tags = db.prepare('SELECT * FROM tags where link = ?');

        const res: Link[] = [];

        for (const link of links.iterate(limit)) {
            res.push({
                id: link.id,
                link: link.link,
                note: link.note,
                tags: tags.all(link.id).map((tag) => tag.name),
                added: Date.parse(link.added),
                updated: Date.parse(link.updated),
                hidden: link.hidden === 1,
            })
        }

        return res;
    })();
}
