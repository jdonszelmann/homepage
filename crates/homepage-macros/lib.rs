use homepage_markdown::{BlogPost, Preamble, Variant};
use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::collections::{HashSet, VecDeque};
use std::fs::read_dir;
use std::path::{Path, PathBuf};

fn collect_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, String> {
    let mut res = Vec::new();

    let mut todo = VecDeque::new();
    todo.push_back(root.as_ref().to_path_buf());

    while let Some(i) = todo.pop_front() {
        for entry in read_dir(&i).map_err(|e| format!("error reading {i:?}: {e:?}"))? {
            let entry = entry.map_err(|e| format!("error reading entry in {i:?}: {e:?}"))?;
            let file_type = entry.file_type().map_err(|e| {
                format!(
                    "error getting file type for direntry {}: {e:?}",
                    entry.path().display()
                )
            })?;

            if file_type.is_dir() {
                todo.push_back(entry.path().to_path_buf())
            } else if entry
                .path()
                .extension()
                .is_some_and(|i| i == "mdx" || i == "md")
            {
                res.push(entry.path().to_path_buf());
            }
        }
    }

    Ok(res)
}

fn serialize_blog(post: &BlogPost) -> TokenStream2 {
    let BlogPost {
        preamble:
            Preamble {
                title,
                publication_date,
                description,
                authors,
                reviewers,
                tags,
                draft,
                time,
                variant,
                ligatures,
            },
        slug,
        templatable_source,
        templatable_description,
        filepath,
    } = post;

    let title = title.as_ref();
    let publication_date = publication_date.as_ref();
    let description = description.as_ref();
    let authors = authors
        .iter()
        .map(|i| {
            let i = i.as_ref();
            quote!(Cow::Borrowed(#i),)
        })
        .collect::<Vec<_>>();
    let reviewers = reviewers
        .iter()
        .map(|i| {
            let i = i.as_ref();
            quote!(Cow::Borrowed(#i),)
        })
        .collect::<Vec<_>>();
    let tags = tags
        .iter()
        .map(|i| {
            let i = i.as_ref();
            quote!(Cow::Borrowed(#i),)
        })
        .collect::<Vec<_>>();
    let draft = *draft;
    let time = time
        .as_ref()
        .map(|i| {
            let i = i.as_ref();
            quote! {
                Some(Cow::Borrowed(#i))
            }
        })
        .unwrap_or(quote! {None});

    let slug = slug.as_ref();
    let templatable_source = templatable_source.as_ref();
    let templatable_description = templatable_description.as_ref();
    let filepath = filepath.as_ref();
    let variant = match variant {
        Variant::Normal => quote!(Variant::Normal),
        Variant::Music => quote!(Variant::Music),
    };
    let ligatures = *ligatures;

    quote! {
        BlogPost {
            preamble: Preamble {
                title: Cow::Borrowed(#title),
                publication_date: Cow::Borrowed(#publication_date),
                description: Cow::Borrowed(#description),
                authors: Cow::Borrowed(&[#(#authors)*]),
                reviewers: Cow::Borrowed(&[#(#reviewers)*]),
                tags: Cow::Borrowed(&[#(#tags)*]),
                draft: #draft,
                time: #time,
                variant: #variant,
                ligatures: #ligatures,
            },
            slug: Cow::Borrowed(#slug),
            filepath: Cow::Borrowed(#filepath),
            templatable_source: Cow::Borrowed(#templatable_source),
            templatable_description: Cow::Borrowed(#templatable_description),
        }
    }
}

fn post_url(post: &BlogPost) -> String {
    format!("/blog/{}", post.slug)
}

fn posts_with_tag<'a>(
    posts: &'a [BlogPost],
    tag: Option<&str>,
    include_drafts: bool,
) -> Vec<&'a BlogPost> {
    posts
        .iter()
        // only non drafts, unless we're supposed to include drafts
        .filter(|i| include_drafts || !i.draft)
        // only posts with the requested tags, unless all tags are requested
        .filter(|i| tag.is_none_or(|requested_tag| i.tags.iter().any(|i| i == requested_tag)))
        .collect()
}

fn make_blogpost_set(
    name: Ident,
    overview_route: &str,
    posts: &[&BlogPost],
) -> (TokenStream2, TokenStream2) {
    let data = posts.iter().map(|post| {
        let url = post_url(post);
        let serialized = serialize_blog(post);
        quote! {
            (#url, #serialized),
        }
    });

    (
        quote! {
            const #name: &[(&str, BlogPost)] = &[#(#data)*];
        },
        quote! {
          .route(#overview_route, overview_route(#name))
        },
    )
}

fn all_tags(posts: &[BlogPost]) -> HashSet<String> {
    posts
        .iter()
        .flat_map(|i| i.tags.as_ref().to_vec())
        .map(|i| i.into_owned())
        .collect()
}

fn convert_tag(tag: &str) -> String {
    tag.to_uppercase().replace("-", "_").replace(" ", "_")
}

#[proc_macro]
pub fn collect_blog_posts(ts: TokenStream) -> TokenStream {
    let input: TokenStream2 = ts.into();

    let root = "./templates/blog";
    let sources = match collect_files(root) {
        Ok(i) => i,
        Err(e) => return (quote! { compile_error!(#e) }).into(),
    };

    let mut posts = Vec::<BlogPost>::new();
    for i in sources {
        match BlogPost::from_file(root, i) {
            Ok(i) => posts.push(i),
            Err(e) => {
                let e = format!("{e:?}");
                return (quote! { compile_error!(#e) }).into();
            }
        }
    }

    posts.sort_by_cached_key(|i| i.publication_date.clone());

    let all_blogposts_name = format_ident!("ALL_POSTS_DRAFTS");

    let (blog_data, overview_routes): (Vec<_>, Vec<_>) = [
        make_blogpost_set(
            format_ident!("ALL_POSTS"),
            "/blog",
            &posts_with_tag(&posts, None, false),
        ),
        make_blogpost_set(
            all_blogposts_name.clone(),
            "/blog/drafts",
            &posts_with_tag(&posts, None, true),
        ),
    ]
    .into_iter()
    .chain(all_tags(&posts).into_iter().flat_map(|tag| {
        let normal = format_ident!("TAG_{}_POSTS", convert_tag(&tag));
        let drafts = format_ident!("TAG_{}_POSTS_DRAFTS", convert_tag(&tag));

        [
            make_blogpost_set(
                normal,
                &format!("/blog/tag/{tag}"),
                &posts_with_tag(&posts, Some(&tag), false),
            ),
            make_blogpost_set(
                drafts,
                &format!("/blog/tag/{tag}/drafts"),
                &posts_with_tag(&posts, Some(&tag), true),
            ),
        ]
    }))
    .unzip();

    let blogpost_routes = posts
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let url = post_url(post);
            let templatable_source = format!(
                r#"{{% extends "layouts/blog.html" %}} {{% block contents %}} {} {{% endblock %}}"#,
                post.templatable_source.as_ref()
            );
            quote! {
                  .route(#url, #input !(#templatable_source, #all_blogposts_name[#idx].1))

            }
        })
        .collect::<Vec<_>>();

    (quote! {
        #(#blog_data)*

        pub fn routes<S: Clone + Send + Sync + 'static>(r: Router<S>) -> Router<S> {
            r
            #(#blogpost_routes)*
            #(#overview_routes)*
        }
    })
    .into()
}
