use std::time::Duration;

use axum_oidc::openidconnect::reqwest;
use eyre::{Context, ContextCompat};
use time::UtcDateTime;

use crate::{
    pages::lists::{
        item::{
            self,
            raw::{AddedThrough, set_item_note, undelete_item},
        },
        rss::{
            hl::Rss,
            raw::{self, append_error, clear_error, item_exists, update_rss},
        },
    },
    state::ArcRouteState,
};

pub async fn all_rss_sources(state: &ArcRouteState) -> eyre::Result<Vec<Rss>> {
    let mut conn = state.db.acquire().await.context("aqcuire")?;
    let res = raw::all_rss_sources(&mut conn).await?;

    res.into_iter().map(Rss::from_raw).collect::<Result<_, _>>()
}

pub fn format_item(
    rss: &Rss,
    item: &feed_rs::model::Entry,
    name: Option<&str>,
    link: &str,
) -> String {
    let author = item
        .authors
        .first()
        .map(|a| format!("\nby {}", a.name))
        .unwrap_or_default();
    let title = item
        .title
        .as_ref()
        .map(|i| i.content.clone())
        .unwrap_or(link.to_string());
    let description = item
        .summary
        .as_ref()
        .map(|d| format!("\n{}", d.content))
        .unwrap_or_default();
    let name = name.unwrap_or(&rss.url);

    let note = format!(
        "
[{title}]({link})

{description}
{author}
from {name}
"
    );

    note
}

pub async fn check_item_update(
    state: &ArcRouteState,
    rss: &Rss,
    name: Option<&str>,
    item: &feed_rs::model::Entry,
) -> eyre::Result<()> {
    let link = item
        .links
        .first()
        .map(|i| &i.href)
        .wrap_err("item has no link")?;
    let guid = &item.id;

    {
        let mut conn = state.db.begin().await.wrap_err("aqcuire")?;
        let formatted = format_item(rss, item, name, link);
        if let Some(item) = item_exists(&mut conn, rss.list.0, guid)
            .await
            .wrap_err("item exists")?
        {
            set_item_note(&mut conn, item, &formatted)
                .await
                .wrap_err("set item note")?;
            undelete_item(&mut conn, item)
                .await
                .wrap_err("undelete item")?;
        } else {
            let time_added = item
                .published
                .and_then(|i| UtcDateTime::from_unix_timestamp(i.timestamp()).ok())
                .unwrap_or_else(UtcDateTime::now);

            tracing::info!("adding new item with guid {guid} to {}", rss.url);
            let _item = item::raw::create_item(
                &mut conn,
                rss.list.0,
                &formatted,
                Some(guid),
                AddedThrough::Rss,
                Some(time_added),
            )
            .await
            .wrap_err("add item from rss")?;
        }

        conn.commit().await.context("commit tx")?;
    }

    Ok(())
}

fn should_update(rss: &Rss, now: UtcDateTime) -> bool {
    let duration_since_last_update = Duration::from_secs(
        now.unix_timestamp()
            .checked_sub(rss.updated.unix_timestamp())
            .unwrap_or_default() as u64,
    );
    tracing::debug!(
        "deciding whether to update {}: last update {duration_since_last_update:?} ago",
        rss.url
    );

    // update once every 10 minutes
    if duration_since_last_update < Duration::from_mins(10) {
        return false;
    }

    true
}

async fn update_feed(state: &ArcRouteState, rss: &Rss, force: bool) -> eyre::Result<()> {
    let now = UtcDateTime::now();
    if !should_update(rss, now) && !force {
        return Ok(());
    }

    tracing::info!("updating {}", rss.url);
    {
        let mut conn = state.db.acquire().await.wrap_err("aqcuire")?;
        update_rss(&mut conn, rss.id.0, now)
            .await
            .wrap_err("set updated timestamp")?;
    }

    let content = reqwest::get(&rss.url)
        .await
        .wrap_err("get request from feed")?
        .bytes()
        .await
        .wrap_err("read response as bytes")?;

    // TODO: hash content to see if anything changed
    // TODO: see if channel has an update frequency
    // TODO: dedupe rss feeds for more than one list
    // TODO: undelete deleted items
    let feed = {
        let parser = feed_rs::parser::Builder::new()
            .sanitize_content(true)
            .build();
        parser.parse(&content[..]).wrap_err("parse feed")?
    };
    let title = feed.title.map(|i| i.content);

    for item in &feed.entries {
        check_item_update(state, rss, title.as_deref(), item).await?;
    }

    Ok(())
}

pub async fn try_update_feed(state: &ArcRouteState, rss: &Rss, force: bool) -> eyre::Result<()> {
    {
        let mut conn = state.db.acquire().await.wrap_err("aqcuire")?;
        clear_error(&mut conn, rss.id.0)
            .await
            .wrap_err("clear errors")?
    }

    if let Err(e) = tokio::spawn({
        let state = state.clone();
        let rss = rss.clone();
        async move { update_feed(&state, &rss, force).await }
    })
    .await
    .map_err(|e| eyre::eyre!("{e:?}"))
    .flatten()
    .wrap_err_with(|| format!("update {}", rss.url))
    {
        tracing::error!("{e:?}");
        {
            let mut conn = state.db.acquire().await.wrap_err("aqcuire")?;
            append_error(&mut conn, rss.id.0, &format!("{e:?}"))
                .await
                .wrap_err("add errors")?;
        }
    }

    Ok(())
}

pub async fn try_update_all_feeds(state: &ArcRouteState) -> eyre::Result<()> {
    for rss in all_rss_sources(state).await? {
        try_update_feed(state, &rss, false).await?
    }

    Ok(())
}

pub async fn start_periodic_feed_updates(state: &ArcRouteState) {
    let state = state.clone();
    tokio::spawn(async move {
        // every minute
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            tokio::spawn({
                let state = state.clone();
                async move {
                    // try updating feeds. Only when their 'updated' is old enough.
                    if let Err(e) = try_update_all_feeds(&state).await {
                        tracing::error!("{e}");
                    }
                }
            });
        }
    });
}
