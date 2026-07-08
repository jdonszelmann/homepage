use std::time::Duration;

use axum_oidc::openidconnect::reqwest;
use eyre::{Context, ContextCompat};
use rss::Channel;
use time::{UtcDateTime, format_description::well_known::Rfc2822};

use crate::{
    pages::lists::{
        item::{self, raw::AddedThrough},
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

pub fn format_item(item: &rss::Item, link: &str) -> String {
    let author = item
        .author()
        .map(|a| format!("\nby {a}"))
        .unwrap_or_default();
    let title = item.title().unwrap_or(link);
    let description = item
        .description()
        .map(|d| format!("\n{d}"))
        .unwrap_or_default();

    let note = format!(
        "
# [{title}]({link})

{description}
{author}
"
    );

    note
}

pub async fn check_item_update(
    state: &ArcRouteState,
    rss: &Rss,
    item: &rss::Item,
) -> eyre::Result<()> {
    let link = item.link().wrap_err("item has no link")?;
    let guid = item.guid().map(|i| i.value()).unwrap_or(link);

    {
        let mut conn = state.db.begin().await.wrap_err("aqcuire")?;
        if !item_exists(&mut conn, rss.list.0, guid)
            .await
            .wrap_err("item exists")?
        {
            let time_added = item
                .pub_date()
                .and_then(|i| UtcDateTime::parse(i, &Rfc2822).ok())
                .unwrap_or_else(UtcDateTime::now);

            tracing::info!("adding new item with guid {guid} to {}", rss.url);
            let _item = item::raw::create_item(
                &mut conn,
                rss.list.0,
                &format_item(item, link),
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

    let channel = Channel::read_from(&content[..]).wrap_err("read channel")?;

    for item in channel.items() {
        check_item_update(state, rss, item).await?
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
