import { defineConfig } from 'astro/config';
import expressiveCode from "astro-xpressive-code";
import { pluginLineNumbers } from '@expressive-code/plugin-line-numbers';

import mdx from "@astrojs/mdx";


// https://astro.build/config
export default defineConfig({
    output: 'static',
    site: 'https://donsz.nl/',
    integrations: [expressiveCode({
        plugins: [pluginLineNumbers()],
        themes: ['github-dark'],
        styleOverrides: {
            codeFontFamily: "JBM"
        }
    }), mdx()]
});
