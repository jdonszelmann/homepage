use axum::{extract::State, response::IntoResponse};
use eyre::Context;
use rss::{Channel, ChannelBuilder, GuidBuilder, ItemBuilder};
use time::format_description::well_known::Rfc2822;

use crate::pages::blog::BLOGPOST_INFO;
use crate::{pages::error::RequestError, state::ArcRouteState};

fn generate_channel(base_url: &str) -> eyre::Result<Channel> {
    let mut items = Vec::new();
    for (_, post, render) in BLOGPOST_INFO {
        let url = format!("{base_url}/blog/{}", post.slug);
        items.push(
            ItemBuilder::default()
                .title(post.title.as_ref().to_string())
                .link(url.clone())
                .description(post.preamble.description.as_ref().to_string())
                .pub_date(
                    post.publication_date
                        .with_hms(0, 0, 0)
                        .context("add time")?
                        .assume_utc()
                        .format(&Rfc2822)
                        .context("format time")?,
                )
                .guid(GuidBuilder::default().value(url).build())
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
