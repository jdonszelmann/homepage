import { APIRoute, Astro } from "astro";
import { removeLink } from "../../../lib/urls.ts";
import { loggedIn } from "./index.ts";

export const POST = (async ({ request, locals, redirect }) => {
    if (!loggedIn(locals)) {
        return redirect("/login")
    }

    const data = await request.formData();
    const id = data.get("id");
    if (!id) {
        console.error("no id")
        return redirect("/dashboard?error=noid")
    }

    removeLink(id);
    return redirect("/addlink")
}) satisfies APIRoute;
