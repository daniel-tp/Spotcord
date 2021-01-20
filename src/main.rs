use dotenv::dotenv;
use lazy_static::lazy_static;
use regex::Regex;

use serenity::{
    async_trait,
    model::{
        channel::{Message, ReactionType},
        gateway::Ready,
        prelude::Activity,
    },
    prelude::*,
};
use std::collections::HashSet;
use std::env;

use rspotify::oauth2::SpotifyClientCredentials;
use rspotify::oauth2::SpotifyOAuth;
use rspotify::{client::Spotify, util::get_token};

use anyhow::{anyhow, Result};

lazy_static! {
    static ref SPOTIFY_TRACK_REGEX: Regex =
        Regex::new(r"https://open.spotify.com/track/([a-zA-Z0-9]{22})").unwrap();
}

enum PlaylistResult {
    Ok,
    SemiOk,
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id != env::var("CHANNEL").unwrap().parse::<u64>().unwrap() {
            // My god this needs to be improved
            return;
        }
        if msg.content.starts_with("!playlist") {
            if let Err(why) = msg
                .channel_id
                .say(
                    &ctx.http,
                    format!(
                        "Playlist is here: https://open.spotify.com/playlist/{}",
                        &env::var("PLAYLIST").unwrap().as_str()[17..]
                    ),
                )
                .await
            {
                println!("Error sending message: {:?}", why);
            }
            return;
        }
        if msg.content.contains("spotify") {
            let mut ids: HashSet<String> = HashSet::new();
            let mut last_track = "".to_string();
            for c in SPOTIFY_TRACK_REGEX.captures_iter(&msg.content) {
                last_track = c.get(1).unwrap().as_str().to_string();
                ids.insert(last_track.clone());
            }
            if ids.len() == 0 {
                return;
            }
            match add_to_playlist(ids).await {
                Ok(PlaylistResult::Ok) => {
                    msg.react(&ctx, ReactionType::Unicode("🔊".to_string()))
                        .await
                        .ok();
                }
                Ok(PlaylistResult::SemiOk) => {
                    msg.react(&ctx, ReactionType::Unicode("⁉️".to_string()))
                        .await
                        .ok();
                }
                Err(e) => {
                    msg.react(&ctx, ReactionType::Unicode("🔇".to_string()))
                        .await
                        .ok();
                    println!("Adding playlist error: {:?}", e);
                }
            }
            let track_name = get_track_name(last_track).await.unwrap();
            let activity = Activity::listening(&track_name);
            ctx.set_activity(activity).await;
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let activity = Activity::listening("Your Music");
        ctx.set_activity(activity).await;
    }
}
#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create a new instance of the Client, logging in as a bot. This will
    // automatically prepend your bot token with "Bot ", which is a requirement
    // by Discord for bot users.
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
    //Ok(())
}

async fn get_spotify() -> Result<Spotify> {
    let mut spotify_oauth = SpotifyOAuth::default()
        .scope("playlist-modify-private playlist-modify-public")
        .client_id(env::var("CLIENT_ID").unwrap().as_str())
        .client_secret(env::var("CLIENT_SECRET").unwrap().as_str())
        .redirect_uri("http://localhost.com")
        .build();

    match get_token(&mut spotify_oauth).await {
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .build();
            Ok(Spotify::default()
                .client_credentials_manager(client_credential)
                .build())
        }
        None => Err(anyhow!("Failed Auth, unable to get token")),
    }
}

async fn add_to_playlist(tracks_to_add: HashSet<String>) -> Result<PlaylistResult> {
    let mut tracks_to_add = tracks_to_add.clone();
    let playlist_id = String::from(env::var("PLAYLIST").unwrap().as_str());
    let duplicates = filter_duplicates(&playlist_id, &mut tracks_to_add).await;

    if tracks_to_add.is_empty() {
        return Err(anyhow!("No tracks to add"));
    }

    let spotify = get_spotify().await?;
    match spotify
        .user_playlist_add_tracks(
            "spotify",
            &playlist_id,
            &tracks_to_add.into_iter().collect::<Vec<_>>(),
            None,
        )
        .await
    {
        Err(e) => {
            println!("Adding playlist error: {:?}", e);
            return Err(anyhow!("Failed to add to playlist"));
        }
        _ => {
            if duplicates {
                return Ok(PlaylistResult::SemiOk);
            } else {
                return Ok(PlaylistResult::Ok);
            }
        }
    }
}

async fn filter_duplicates(playlist_id: &str, tracks_to_check: &mut HashSet<String>) -> bool {
    let amount = 100;
    let mut current = 0;
    let mut filtered = false;
    match get_spotify().await {
        Ok(spotify) => {
            while let Ok(tracklist) = spotify
                .user_playlist_tracks(
                    "spotify",
                    &playlist_id,
                    None,
                    amount,
                    amount * current,
                    None,
                )
                .await
            {
                for track in tracklist.items.into_iter() {
                    let track_id = track.track.unwrap().id.unwrap_or_default(); //TODO: improve this
                    if tracks_to_check.contains(&track_id) {
                        let err = tracks_to_check.remove(&track_id);
                        println!("Status: {}", err);
                        filtered = true;
                    }
                    if tracks_to_check.len() == 0 {
                        break;
                    }
                }
                if tracks_to_check.len() == 0 {
                    break;
                }
                match tracklist.next {
                    Some(_) => current += 1,
                    None => break,
                }
            }
        }
        Err(_) => println!("Unable to connect to Spotify"),
    }
    filtered
}

async fn get_track_name(id: String) -> Result<String> {
    let spotify = get_spotify().await?;
    match spotify.track(&id).await {
        Ok(track) => Ok(track.name),
        Err(_) => Err(anyhow!("Unable to get matching track"))
    }
}
