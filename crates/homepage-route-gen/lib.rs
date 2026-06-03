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
    generate_overview_route_function: Ident,
    overview_route: &str,
    posts: &[&BlogPost],
) -> (TokenStream2, TokenStream2) {
    let data = posts.iter().map(|post| {
        let url = post_url(post);
        let serialized = post.reproduce_tokens();
        quote! {
            (#url, #serialized),
        }
    });

    (
        quote! {
            const #name: &[(&str, BlogPost)] = &[#(#data)*];
        },
        quote! {
          .route(#overview_route, #generate_overview_route_function(#name))
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
        .join(&repo_root_to_blog);
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
        match BlogPost::from_file(expansion_base.join(&i), macro_base.join(&i)) {
            Ok(i) => posts.push(i),
            Err(e) => {
                let e = format!("{e:?}");
                return (quote! { compile_error!(#e) }).into();
            }
        }
    }

    posts.sort_by_cached_key(|i| Reverse(i.publication_date.clone()));

    let all_blogposts_name = format_ident!("ALL_POSTS_DRAFTS");

    let (blog_data, overview_routes): (Vec<_>, Vec<_>) = [
        make_blogpost_set(
            format_ident!("ALL_POSTS"),
            generate_overview_route_function.clone(),
            "/blog",
            &posts_with_tag(&posts, None, false),
        ),
        make_blogpost_set(
            all_blogposts_name.clone(),
            generate_overview_route_function.clone(),
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
                generate_overview_route_function.clone(),
                &format!("/blog/tag/{tag}"),
                &posts_with_tag(&posts, Some(&tag), false),
            ),
            make_blogpost_set(
                drafts,
                generate_overview_route_function.clone(),
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
                  .route(#url, #generate_route_macro !(#templatable_source, #all_blogposts_name[#idx].1))

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
        pub fn routes(r: #prelude::Router<#prelude::ArcRouteState>) -> #prelude::Router<#prelude::ArcRouteState> {
            use #prelude::*;

            #(#includes)*
            #(#blog_data)*

            r
                #(#blogpost_routes)*
                #(#overview_routes)*
        }
    })
    .into()
}
