import rss, { pagesGlobToRssItems } from '@astrojs/rss';
import { getCollection } from 'astro:content';

export async function GET(context) {
    const blog = await getCollection('blog');
    const items = blog.filter((post) => !post.data.draft).map((post) => ({
        title: post.data.title,
        pubDate: post.data.pubDate,
        description: post.data.description,
        categories: post.data.tags,
        link: `/blog/${post.slug}/`,
    }));

    return rss({
        title: 'Jana\'s Blog',
        description: 'Talking about random things, often Rust related',
        site: context.site,
        items,
        customData: `<language>en-us</language>`,
    });
}
