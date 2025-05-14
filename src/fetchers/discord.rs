use std::{
  collections::HashMap,
  panic::AssertUnwindSafe,
  sync::{LazyLock, RwLock},
  time::Duration,
};

use futures::FutureExt;
use serde::Serialize;
use serenity::all::{
  ActivityEmoji, ActivityType, CacheHttp, ChunkGuildFilter, ClientStatus, Context, EventHandler,
  GatewayIntents, GuildMembersChunkEvent, OnlineStatus, Presence, Ready,
};
use tracing::info;
use ts_rs::TS;

use crate::config::Config;

pub async fn run_discord_bot(config: &Config) -> anyhow::Result<()> {
  let Some(token) = config.discord_bot_token.as_ref().map(String::as_str) else {
    return Ok(());
  };

  let mut client = serenity::Client::builder(
    token,
    GatewayIntents::GUILD_PRESENCES | GatewayIntents::GUILD_MEMBERS,
  )
  .event_handler(Handler(config.discord_initial_search_guilds.clone()))
  .await?;

  tokio::spawn(async move {
    loop {
      if let Err(error) = AssertUnwindSafe(client.start()).catch_unwind().await {
        eprintln!("serenity client crashed: {error:#?}");
      }

      tokio::time::sleep(Duration::from_secs(30)).await
    }
  });

  tracing::info!("started discord fetcher");

  Ok(())
}

static USERS: LazyLock<RwLock<HashMap<u64, DiscordUserInfo>>> = LazyLock::new(Default::default);

#[derive(Clone, Serialize, TS)]
#[ts(rename = "DiscordEmoji")]
pub enum Emoji {
  Official {
    name: String,
  },
  Unofficial {
    name: String,
    id: u64,
    animated: bool,
    url: String,
  },
  Unknown {
    name: String,
    id: Option<u64>,
    animated: Option<bool>,
  },
}

#[derive(Clone, Serialize, TS)]
#[ts(rename = "DiscordCustomStatus")]
pub struct CustomStatus {
  emoji: Option<Emoji>,
  text: Option<String>,
}

#[allow(unused)]
#[derive(Serialize, TS)]
#[ts(rename = "DiscordOnlineStatus")]
pub enum TypescriptOnlineStatus {
  #[serde(rename = "dnd")]
  DoNotDisturb,
  #[serde(rename = "idle")]
  Idle,
  #[serde(rename = "invisible")]
  Invisible,
  #[serde(rename = "offline")]
  Offline,
  #[serde(rename = "online")]
  Online,
}

#[derive(Clone, Serialize, TS)]
pub struct DiscordUserInfo {
  display_name: String,
  #[ts(as = "TypescriptOnlineStatus")]
  status: OnlineStatus,
  #[ts(as = "Option<TypescriptOnlineStatus>")]
  client_status: Option<ClientStatus>,
  custom_status: Option<CustomStatus>,
}

pub fn fetch_user_info(user_id: u64) -> Option<DiscordUserInfo> {
  USERS.read().unwrap().get(&user_id).cloned()
}

struct Handler(Vec<u64>);

async fn build_user_info(ctx: &impl CacheHttp, presence: Presence) -> Option<DiscordUserInfo> {
  let display_name = if let Some(user) = ctx.cache().and_then(|cache| cache.user(presence.user.id))
  {
    user.display_name().to_owned()
  } else {
    let Ok(user) = ctx.http().get_user(presence.user.id).await else {
      return None;
    };
    user.display_name().to_owned()
  };

  let custom_status = presence.activities.into_iter().find_map(|activity| {
    if activity.kind != ActivityType::Custom {
      return None;
    }

    Some(CustomStatus {
      emoji: activity.emoji.map(|emoji| match emoji {
        ActivityEmoji {
          name,
          id: None,
          animated: None,
          ..
        } => Emoji::Official { name },
        ActivityEmoji {
          name,
          id: Some(id),
          animated: Some(animated),
          ..
        } => Emoji::Unofficial {
          name,
          id: id.get(),
          animated,
          url: format!(
            "https://cdn.discordapp.com/emojis/691095252458537011.webp?size=160&animated={animated}"
          ),
        },
        emoji => {
          tracing::error!("bad emoji: {emoji:?}");
          Emoji::Unknown {
            name: emoji.name,
            id: emoji.id.map(Into::into),
            animated: emoji.animated,
          }
        }
      }),
      text: activity.state,
    })
  });

  Some(DiscordUserInfo {
    display_name: display_name.to_owned(),
    status: presence.status,
    client_status: presence.client_status,
    custom_status,
  })
}

#[async_trait::async_trait]
impl EventHandler for Handler {
  async fn ready(&self, ctx: Context, _: Ready) {
    for guild in self.0.iter().cloned() {
      info!("checking guild {guild}");
      ctx
        .shard
        .chunk_guild(guild.into(), None, true, ChunkGuildFilter::None, None)
    }
  }

  async fn guild_members_chunk(&self, ctx: Context, chunk: GuildMembersChunkEvent) {
    info!("got guild chunk");
    for presence in chunk.presences.into_iter().flatten() {
      self.presence_update(ctx.clone(), presence).await;
    }
  }

  async fn presence_update(&self, ctx: Context, presence: Presence) {
    let user_id = presence.user.id;
    if let Some(presence) = build_user_info(&ctx, presence).await {
      USERS.write().unwrap().insert(user_id.into(), presence);
    }
  }
}
