use axum::{extract::State, response::IntoResponse};
use eyre::Context;
use rss::{Channel, ChannelBuilder, ItemBuilder};

use crate::pages::blog::BLOGPOST_INFO;
use crate::{pages::error::RequestError, state::ArcRouteState};

fn generate_channel(base_url: &str) -> eyre::Result<Channel> {
    let mut items = Vec::new();
    for (_, post, render) in BLOGPOST_INFO {
        items.push(
            ItemBuilder::default()
                .title(post.title.to_owned().into_owned())
                .link(format!("{base_url}/blog/{}", post.slug))
                .description(post.preamble.description.to_owned().into_owned())
                .content(
                    render(None)
                        .as_ref()
                        .render_contents()
                        .context("render contents")?,
                )
                .build(),
        );
    }

    let channel = ChannelBuilder::default()
        .title("Blog Posts".to_string())
        .language("en".to_string())
        .link(format!("{base_url}/rss.xml"))
        .description("Jana Dönszelmann's blog posts")
        .ttl("60".to_string())
        .items(items)
        .build();

    Ok(channel)
}

pub async fn rss_xml(
    State(state): State<ArcRouteState>,
) -> Result<impl IntoResponse, RequestError> {
    let channel = generate_channel(&state.args.base_url)?;

    Ok(channel.to_string())
}
