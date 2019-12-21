use lazy_static::lazy_static;
use regex::Regex;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::util::get_token;
use serenity::{
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::collections::HashSet;
use dotenv_codegen::dotenv;

struct Handler;
lazy_static! {
    static ref SPOTIFY_TRACK_REGEX: Regex =
        Regex::new(r"https://open.spotify.com/track/([a-zA-Z0-9]{22})").unwrap();
}

enum PlaylistResult {
    Ok,
    SemiOk,
    Err(String),
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id != dotenv!("CHANNEL").parse::<u64>().unwrap() {
            // My god this needs to be improved
            return;
        }
        if msg.content.contains("spotify") {
            let mut ids: HashSet<String> = HashSet::new();
            for c in SPOTIFY_TRACK_REGEX.captures_iter(&msg.content) {
                ids.insert(c.get(1).unwrap().as_str().to_string());
            }
            match add_to_playlist(ids) {
                PlaylistResult::Ok => {
                    msg.react(ctx, "🔊").ok();
                }
                PlaylistResult::SemiOk => {
                    msg.react(ctx, "⁉️").ok();
                }
                PlaylistResult::Err(e) => {
                    msg.react(ctx, "🔇").ok();
                    println!("Adding playlist error: {:?}", e);
                }
            }
        }
    }
    fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = dotenv!("DISCORD_TOKEN");

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

fn get_spotify() -> Option<Spotify> {
    let mut spotify_oauth = SpotifyOAuth::default()
        .scope("playlist-modify-private playlist-modify-public")
        .client_id(dotenv!("CLIENT_ID"))
        .client_secret(dotenv!("CLIENT_SECRET"))
        .redirect_uri("http://localhost.com")
        .build();
    match get_token(&mut spotify_oauth) {
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .build();
            Some(
                Spotify::default()
                    .client_credentials_manager(client_credential)
                    .build(),
            )
        }
        None => {
            println!("auth failed");
            None
        }
    }
}

fn add_to_playlist(tracks_to_add: HashSet<String>) -> PlaylistResult {
    let mut tracks_to_add = tracks_to_add.clone();
    match get_spotify() {
        Some(spotify) => {
            let playlist_id = String::from(dotenv!("PLAYLIST"));
            let duplicates = filter_duplicates(&spotify, &playlist_id, &mut tracks_to_add);

            match spotify.user_playlist_add_tracks(
                "spotify",
                &playlist_id,
                &tracks_to_add.into_iter().collect::<Vec<_>>(),
                None,
            ) {
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
        }
        None => return PlaylistResult::Err("Unable to connect to spotify".to_string()),
    }
}

fn filter_duplicates(
    spotify: &Spotify,
    playlist_id: &str,
    tracks_to_check: &mut HashSet<String>,
) -> bool {
    let amount = 100;
    let mut current = 0;
    let mut filtered = false;
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
    filtered
}
