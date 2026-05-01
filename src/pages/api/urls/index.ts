import { APIRoute, Astro } from "astro";
import { addLink, removeLink } from "../../../lib/urls.ts";

export function loggedIn(locals) {
    if (locals.session) {
        return true;
    } else {
        return false;
    }
}

export const POST = (async ({ request, locals, redirect }) => {
    if (!loggedIn(locals)) {
        return redirect("/login")
    }

    const data = await request.formData();
    const link = data.get("link");
    let note = data.get("note");
    let tags = [];

    if (!note) {
        note = "";
    }
    if (!tags) {
        tags = [];
    }
    if (!link) {
        console.error("no link")
        return redirect("/dashboard?error=nolink")
    }

    addLink(link as string, note as string, tags)
    return redirect("/addlink")
}) satisfies APIRoute;


