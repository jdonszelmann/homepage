import { betterAuth } from "better-auth";
import { genericOAuth } from "better-auth/plugins"

import Database from "better-sqlite3";

export const database = new Database(process.env.DATABASE_LOCATION || "./homepage.db");
database.pragma("foreign_keys = ON");

export const auth = betterAuth({
    database: database,
    plugins: [
        genericOAuth({
            config: [
                {
                    providerId: "auth.donsz.nl",
                    clientId: process.env.CLIENT_ID as string,
                    clientSecret: process.env.CLIENT_SECRET as string,
                    discoveryUrl: "https://auth.donsz.nl/.well-known/openid-configuration",
                    scopes: ['openid', 'profile', 'email', 'groups']
                },
            ]
        })
    ],
})
