use dotenv_codegen::dotenv;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::spotify::client::Spotify;
use rspotify::spotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::spotify::util::get_token;
use serenity::{
    model::{channel::Message, gateway::Ready, gateway::Activity},
    prelude::*,

};
use std::collections::HashSet;

lazy_static! {
    static ref SPOTIFY_TRACK_REGEX: Regex =
        Regex::new(r"https://open.spotify.com/track/([a-zA-Z0-9]{22})").unwrap();
}

lazy_static! {
    static ref SPOTIFY: Spotify = {
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
                Spotify::default()
                    .client_credentials_manager(client_credential)
                    .build()
            }
            None => {
                println!("auth failed");
                panic!("Unable to conect to Spotify");
            }
        }
    };
}

enum PlaylistResult {
    Ok,
    SemiOk,
    Err(String),
}

struct Handler;
impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.channel_id != dotenv!("CHANNEL").parse::<u64>().unwrap() {
            // My god this needs to be improved
            return;
        }
        if msg.content.starts_with("!playlist"){
            if let Err(why) = msg.channel_id.say(&ctx.http, format!("Playlist is here: https://open.spotify.com/playlist/{}", &dotenv!("PLAYLIST")[17..])) {
                println!("Error sending message: {:?}", why);
            }
            return;
        }
        if msg.content.contains("spotify") {
            let mut ids: HashSet<String> = HashSet::new();
            for c in SPOTIFY_TRACK_REGEX.captures_iter(&msg.content) {
                ids.insert(c.get(1).unwrap().as_str().to_string());
            }
            if ids.len()==0 {
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
    // Configure the client with your Discord bot token in the environment.
    let token = dotenv!("DISCORD_TOKEN");
    let mut client = Client::new(&token, Handler).expect("Err creating client");

    if let Err(why) = client.start() {
        println!("Client error: {:?}", why);
    }
}

fn add_to_playlist(tracks_to_add: HashSet<String>) -> PlaylistResult {
    let mut tracks_to_add = tracks_to_add.clone();
    let playlist_id = String::from(dotenv!("PLAYLIST"));
    let duplicates = filter_duplicates(&playlist_id, &mut tracks_to_add);

    match SPOTIFY.user_playlist_add_tracks(
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

fn filter_duplicates(playlist_id: &str, tracks_to_check: &mut HashSet<String>) -> bool {
    let amount = 100;
    let mut current = 0;
    let mut filtered = false;
    while let Ok(tracklist) = SPOTIFY.user_playlist_tracks(
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
