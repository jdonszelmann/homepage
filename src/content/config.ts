import { z, defineCollection } from 'astro:content';

const blogCollection = defineCollection({
    type: 'content',
    schema: z.object({
        title: z.string(),
        tags: z.array(z.string()),
        reviewers: z.array(z.string()),
        authors: z.array(z.string()),
        description: z.string(),
        draft: z.boolean(),
        time: z.string().optional(),
        pubDate: z.date().optional(),
    }),
});

export const collections = {
    'blog': blogCollection,
};
