import { APIRoute, Astro } from "astro";
import { hide as doHide, show } from "../../../lib/urls.ts";
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
    const hide = data.get("hide");
    if (!hide) {
        console.error("no hidden boolean")
        return redirect("/dashboard?error=nohideboolean")
    }

    if (hide == "true") {
        show(id);
    } else {
        doHide(id);
    }
    return redirect("/addlink")
}) satisfies APIRoute;
