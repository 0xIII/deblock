use std::env;

use egg_mode::{Token, media::{media_types, MediaHandle}};

pub async fn auth() -> Token {
    println!("Authenticating with twitter");
    dotenv::dotenv().expect(".env not found");
    let con_token = egg_mode::KeyPair::new(env::var("TAPI_KEY").unwrap(), env::var("TAPI_SEC").unwrap());
    let token = egg_mode::auth::bearer_token(&con_token)
        .await
        .unwrap();
    
    token
}

pub async fn nft_tweet(nft_name: String, nft_uri: String, token: &Token) {
    let handle = add_nft_media(token, nft_uri.clone()).await;
    match handle {
        Some(handle) => {
            let mut draft = egg_mode::tweet::DraftTweet::new(
                format!("{}\n{}\n", nft_name, nft_uri)
            );
            draft.add_media(handle.id);
            let res = draft.send(token)
                .await;
        },
        _ => {
            // TODO: log
            println!("Error adding media handle");
        }
    }
}

async fn add_nft_media(token: &Token, media_uri: String) -> Option<MediaHandle> {
    let nft_media = reqwest::get(&media_uri)
        .await
        .expect("URI unavailable") // FIXME: should prbbly handle, but new nfts will prbbly have working uris
        .bytes()
        .await
        .expect("Unable to load bytes");

    let med_type = match &media_uri[media_uri.len()-4..] {
        ".mp4" => {
            Ok(media_types::video_mp4())
        },
        ".png" => {
            Ok(media_types::image_png())
        },
        ".gif" => {
            Ok(media_types::image_gif())
        },
        ".jpg" => {
            Ok(media_types::image_jpg())
        },
        "webp" => {
            Ok(media_types::image_webp())
        },
        _ => {
            Err(())
        }
    };

    match med_type {
        Ok(mimetype) => {
            let handle = egg_mode::media::upload_media(&nft_media[..], &mimetype, token)
                .await
                .unwrap();
            
            Some(handle)
        },
        Err (_) => {
            // TODO: Log unexpected media type with uri
            None
        },
    }
}