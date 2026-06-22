use homepage_markdown::BlogPost;
use homepage_traits::ReproduceTokens;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use std::cmp::Reverse;
use std::collections::{HashSet, VecDeque};
use std::fs::read_dir;
use std::path::{Path, PathBuf};
use syn::LitStr;
use syn::parse::Parse;

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
                res.push(
                    entry
                        .path()
                        .strip_prefix(root.as_ref())
                        .unwrap()
                        .to_path_buf(),
                );
            }
        }
    }

    Ok(res)
}

fn post_url(post: &BlogPost) -> String {
    format!("/blog/{}", post.slug)
}

fn posts_with_tag<'a>(
    posts: &'a [BlogPost],
    tag: Option<&str>,
    include_drafts: bool,
    derived: Option<Ident>,
) -> Vec<BlogPostOrDerived<'a>> {
    posts
        .iter()
        .enumerate()
        // only non drafts, unless we're supposed to include drafts
        .filter(|(_, i)| include_drafts || !i.draft)
        // only posts with the requested tags, unless all tags are requested
        .filter(|(_, i)| tag.is_none_or(|requested_tag| i.tags.iter().any(|i| i == requested_tag)))
        .map(|(idx, post)| {
            if let Some(d) = &derived {
                BlogPostOrDerived::Derived(quote! {
                    #d[#idx]
                })
            } else {
                BlogPostOrDerived::BlogPost(post)
            }
        })
        .collect()
}

enum BlogPostOrDerived<'a> {
    BlogPost(&'a BlogPost),
    Derived(TokenStream2),
}

fn make_blogpost_set(
    name: Ident,
    generate_overview_route_function: Ident,
    overview_route: &str,
    posts: &[BlogPostOrDerived],
    show_links: bool,
) -> (TokenStream2, TokenStream2) {
    let data = posts.iter().map(|post| match post {
        BlogPostOrDerived::BlogPost(post) => {
            let url = post_url(post);
            let serialized = post.reproduce_tokens();
            let source = format!(
                r#"{{% extends "layouts/blog.html" %}} {{% block contents %}} {} {{% endblock %}}"#,
                post.templatable_source.as_ref()
            );
            let path = post.filepath.as_ref();
            quote! {
                {
                    const SERIALIZED: BlogPost = #serialized;
                    (#url, SERIALIZED, |base: Option<Base>| {
                        #[derive(Template, LiveTemplate)]
                        #[template(source = #source, ext="html", blocks=["main"])]
                        #[template_disambiguator = #path]
                        struct BlogPostTemplate {
                            base: Base,
                            post: &'static BlogPost,
                        }

                        impl Deref for BlogPostTemplate {
                            type Target = Base;

                            fn deref(&self) -> &Self::Target {
                                &self.base
                            }
                        }

                        impl PostTemplate for BlogPostTemplate {
                            fn render_contents(&self) -> Result<String, askama::Error> {
                                self.as_main().render()
                            }
                        }

                        Box::new(BlogPostTemplate {
                            base: base.unwrap_or_default(),
                            post: &SERIALIZED
                        })
                    })
                },
            }
        }
        BlogPostOrDerived::Derived(base) => quote! {#base,},
    });

    (
        quote! {
            pub const #name: &[RouteInfo] = &[#(&#data)*];
        },
        quote! {
          .route(#overview_route, #generate_overview_route_function(#name, #show_links))
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

struct GenerateBlogRoutesInput {
    repo_root: LitStr,
    generate_route_macro: syn::Ident,
    generate_overview_route_function: syn::Ident,
}

impl Parse for GenerateBlogRoutesInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let repo_root = input.parse()?;
        let _: syn::Token![,] = input.parse()?;
        let generate_route_macro = input.parse()?;
        let _: syn::Token![,] = input.parse()?;
        let generate_overview_route_function = input.parse()?;

        Ok(Self {
            repo_root,
            generate_route_macro,
            generate_overview_route_function,
        })
    }
}

#[proc_macro]
pub fn generate_blog_routes(ts: TokenStream) -> TokenStream {
    let GenerateBlogRoutesInput {
        repo_root,
        generate_route_macro,
        generate_overview_route_function,
    } = syn::parse_macro_input!(ts as GenerateBlogRoutesInput);

    let call_path = {
        let mut call_path = Span::call_site().local_file().unwrap();
        call_path.pop();
        call_path
    };

    let call_path_to_repo_root = PathBuf::from(repo_root.value());
    let repo_root_to_blog = Path::new("templates/blog");
    // where are blog posts relative to the macro?
    let macro_base = call_path
        .join(&call_path_to_repo_root)
        .join(repo_root_to_blog);
    // where are blog posts relative to the expanded source?
    // no need to include the call path, since that's already where we're compiling
    // (in the expanded source)
    let expansion_base = call_path_to_repo_root.join(repo_root_to_blog);

    let sources = match collect_files(&macro_base) {
        Ok(i) => i,
        Err(e) => return (quote! { compile_error!(#e) }).into(),
    };

    let mut posts = Vec::<BlogPost>::new();
    for i in sources {
        match BlogPost::from_file(
            expansion_base.join(&i),
            macro_base.join(&i),
            repo_root_to_blog.join(&i),
        ) {
            Ok(i) => posts.push(i),
            Err(e) => {
                let e = format!("{e:?}");
                return (quote! { compile_error!(#e) }).into();
            }
        }
    }

    posts.sort_by_cached_key(|i| Reverse(i.publication_date.clone()));

    let all_posts_drafts_name = format_ident!("ALL_POSTS_DRAFTS");
    let all_posts_name = format_ident!("ALL_POSTS");

    let (blog_data, overview_routes): (Vec<_>, Vec<_>) = [
        make_blogpost_set(
            all_posts_name.clone(),
            generate_overview_route_function.clone(),
            "/blog",
            &posts_with_tag(&posts, None, false, Some(all_posts_drafts_name.clone())),
            true,
        ),
        make_blogpost_set(
            all_posts_drafts_name.clone(),
            generate_overview_route_function.clone(),
            "/blog/drafts",
            &posts_with_tag(&posts, None, true, None),
            false,
        ),
    ]
    .into_iter()
    .chain(all_tags(&posts).into_iter().flat_map(|tag| {
        let normal = format_ident!("TAG_{}_POSTS", convert_tag(&tag));
        let drafts = format_ident!("TAG_{}_POSTS_DRAFTS", convert_tag(&tag));

        [
            make_blogpost_set(
                normal,
                generate_overview_route_function.clone(),
                &format!("/blog/tag/{tag}"),
                &posts_with_tag(
                    &posts,
                    Some(&tag),
                    false,
                    Some(all_posts_drafts_name.clone()),
                ),
                false,
            ),
            make_blogpost_set(
                drafts,
                generate_overview_route_function.clone(),
                &format!("/blog/tag/{tag}/drafts"),
                &posts_with_tag(
                    &posts,
                    Some(&tag),
                    true,
                    Some(all_posts_drafts_name.clone()),
                ),
                false,
            ),
        ]
    }))
    .unzip();

    let blogpost_routes = posts
        .iter()
        .enumerate()
        .map(|(idx, post)| {
            let url = post_url(post);
            quote! {
                  .route(#url, #generate_route_macro !(#all_posts_drafts_name[#idx]))

            }
        })
        .collect::<Vec<_>>();

    let includes: Vec<_> = posts
        .iter()
        .map(|i| {
            let path = i.filepath.as_ref();
            quote! {
                const _: &str = include_str!(#path);
            }
        })
        .collect();

    let prelude = quote! {crate::pages::blog::prelude};

    (quote! {
        mod generated {
            use #prelude::*;

            #(#includes)*
            #(#blog_data)*

            pub fn routes(r: #prelude::Router<#prelude::ArcRouteState>) -> #prelude::Router<#prelude::ArcRouteState> {
                r
                    #(#blogpost_routes)*
                    #(#overview_routes)*
            }
        }

        pub use generated::*;
    })
    .into()
}
