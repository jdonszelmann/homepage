import { defineConfig } from "astro/config";
import expressiveCode from "astro-expressive-code";
import { pluginLineNumbers } from "@expressive-code/plugin-line-numbers";

import mdx from "@astrojs/mdx";

import node from "@astrojs/node";

// https://astro.build/config
export default defineConfig({
  output: "server",
  site: "https://donsz.nl/",

  integrations: [
    expressiveCode({
      // plugins: [pluginLineNumbers()],
      themes: ["github-dark"],
      styleOverrides: {
        codeFontFamily: "JBM",
      },
    }),
    mdx(),
  ],

  redirects: {
     "/eii": "/blog/externally-implementable-items",
     "/blog/eii": "/blog/externally-implementable-items"
   },

  adapter: node({
    mode: "standalone",
  }),
});
