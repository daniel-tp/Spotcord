use dotenv::dotenv;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::util::get_token;
use serenity::{
    model::{channel::Message, gateway::Activity, gateway::Ready},
    prelude::*,
};
use std::collections::HashSet;
use std::env;

lazy_static! {
    static ref SPOTIFY_TRACK_REGEX: Regex =
        Regex::new(r"https://open.spotify.com/track/([a-zA-Z0-9]{22})").unwrap();
}

enum PlaylistResult {
    Ok,
    SemiOk,
    Err(String),
}

struct Handler;
impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id != env::var("CHANNEL").unwrap().parse::<u64>().unwrap() {
            // My god this needs to be improved
            return;
        }
        if msg.content.starts_with("!playlist") {
            if let Err(why) = msg.channel_id.say(
                &ctx.http,
                format!(
                    "Playlist is here: https://open.spotify.com/playlist/{}",
                    &env::var("PLAYLIST").unwrap().as_str()[17..]
                ),
            ) {
                println!("Error sending message: {:?}", why);
            }
            return;
        }
        if msg.content.contains("spotify") {
            let mut ids: HashSet<String> = HashSet::new();
            for c in SPOTIFY_TRACK_REGEX.captures_iter(&msg.content) {
                ids.insert(c.get(1).unwrap().as_str().to_string());
            }
            if ids.len() == 0 {
                return;
            }
            match add_to_playlist(ids) {
                PlaylistResult::Ok => {
                    msg.react(&ctx, "🔊").ok();
                }
                PlaylistResult::SemiOk => {
                    msg.react(&ctx, "⁉️").ok();
                }
                PlaylistResult::Err(e) => {
                    msg.react(&ctx, "🔇").ok();
                    println!("Adding playlist error: {:?}", e);
                }
            }
        }
    }
    fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        let activity = Activity::listening("Your Music");
        ctx.set_activity(activity);
    }
}

fn main() {
    dotenv().ok();

    let mut client = Client::new(&env::var("DISCORD_TOKEN").unwrap().as_str(), Handler).expect("Err creating client");

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

fn get_spotify() -> Result<Spotify, String>
{
    let mut spotify_oauth = SpotifyOAuth::default()
        .scope("playlist-modify-private playlist-modify-public")
        .client_id(env::var("CLIENT_ID").unwrap().as_str())
        .client_secret(env::var("CLIENT_SECRET").unwrap().as_str())
        .redirect_uri("http://localhost.com")
        .build();
    match get_token(&mut spotify_oauth) {
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .build();
            Ok(Spotify::default()
                .client_credentials_manager(client_credential)
                .build())
        }
        None => {
            Err(String::from("Failed Auth, unable to get token"))
        }
    }
}

fn add_to_playlist(tracks_to_add: HashSet<String>) -> PlaylistResult {
    let mut tracks_to_add = tracks_to_add.clone();
    let playlist_id = String::from(env::var("PLAYLIST").unwrap().as_str());
    let duplicates = filter_duplicates(&playlist_id, &mut tracks_to_add);

    if tracks_to_add.is_empty() {
        return PlaylistResult::Err("No tracks to add".to_string());
    }

    match get_spotify() {
        Ok(spotify) => {
        match spotify.user_playlist_add_tracks(
            "spotify",
            &playlist_id,
            &tracks_to_add.into_iter().collect::<Vec<_>>(),
            None) {
                Err(e) => {
                    println!("Adding playlist error: {:?}", e);
                    return PlaylistResult::Err("Failed to add to playlist".to_string());
                }
                _ => {
                    if duplicates {
                        return PlaylistResult::SemiOk;
                    } else {
                        return PlaylistResult::Ok;
                    }
                }
            }
        },
        Err(msg) => return PlaylistResult::Err(msg),
    }
}

fn filter_duplicates(playlist_id: &str, tracks_to_check: &mut HashSet<String>) -> bool {
    let amount = 100;
    let mut current = 0;
    let mut filtered = false;
    match get_spotify() {
        Ok(spotify) => {
            while let Ok(tracklist) = spotify.user_playlist_tracks(
                "spotify",
                &playlist_id,
                None,
                amount,
                amount * current,
                None,
            ) {
                for track in tracklist.items.into_iter() {
                    let track_id = track.track.id.unwrap_or_default();
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
        },
        Err(_) => println!("Unable to connect to Spotify")
    }
    filtered
}
