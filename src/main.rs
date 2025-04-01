use chrono::DateTime;
use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::{fs::File, io::Write};

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    server: String,
    #[arg(short, long)]
    username: String,
    #[arg(short, long)]
    password: String,
    #[arg(short, long, help = "Append .ts to stream URLs")]
    ts: bool,
    #[arg(short, long, help = "Include VOD streams")]
    vod: bool,
    #[arg(short, long, group = "g")]
    m3u_file: Option<String>,
    #[arg(short, long, group = "g")]
    account_info: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let account_url = format!(
        "{}/player_api.php?username={}&password={}",
        args.server, args.username, args.password
    );
    let category_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_live_categories",
        args.server, args.username, args.password
    );
    let stream_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_live_streams",
        args.server, args.username, args.password
    );
    let vod_categories_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_vod_categories",
        args.server, args.username, args.password
    );
    let vod_streams_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_vod_streams",
        args.server, args.username, args.password
    );
    let stream_ext = match args.ts {
        true => ".ts",
        false => "",
    };

    let mut total_streams = 0;

    match reqwest::get(account_url).await {
        Ok(resp) => {
            if resp.status() != 200 {
                println!("Error {} getting account information", resp.status());
                println!("Verify that your username and password are correct");
                std::process::exit(1);
            }
            let a_json = resp.json::<Value>().await?;
            let expires: i64 = match a_json["user_info"]["exp_date"].as_str() {
                Some(s) => s.parse().unwrap(),
                _ => a_json["user_info"]["exp_date"].as_i64().unwrap_or_default(),
            };
            let created: i64 = match a_json["user_info"]["created_at"].as_str() {
                Some(s) => s.parse().unwrap(),
                _ => a_json["user_info"]["created_at"]
                    .as_i64()
                    .unwrap_or_default(),
            };
            let max_connections: i64 = match a_json["user_info"]["max_connections"].as_str() {
                Some(s) => s.parse().unwrap(),
                _ => a_json["user_info"]["max_connections"]
                    .as_i64()
                    .unwrap_or_default(),
            };
            let is_trial: bool = match a_json["user_info"]["is_trial"].is_boolean() {
                true => a_json["user_info"]["is_trial"].as_bool().unwrap(),
                false => matches!(a_json["user_info"]["is_trial"].as_str(), Some("1")),
            };

            println!("Account Information:");
            println!(
                " Created: {}",
                DateTime::from_timestamp(created, 0).expect("Invalid Timestamp")
            );
            println!(
                " Expires: {}",
                DateTime::from_timestamp(expires, 0).expect("Invalid Timestamp")
            );
            println!(
                " Status: {}",
                a_json["user_info"]["status"].as_str().unwrap_or_default()
            );
            println!(
                " Active Connections: {}",
                a_json["user_info"]["active_cons"]
                    .as_str()
                    .unwrap_or_default()
            );
            println!(" Max Connections: {max_connections}");
            println!(" Trial: {is_trial}",);
        }
        Err(err) => println!("Error: {err:?}"),
    }
    if args.account_info {
        std::process::exit(0);
    }
    let m3u_file = match args.m3u_file {
        Some(f) => f,
        _ => {
            println!("No m3u file supplied");
            std::process::exit(0)
        }
    };
    let mut output = match File::create(&m3u_file) {
        Ok(f) => f,
        Err(e) => {
            panic!("Error creating {:?}: {e:?}", m3u_file);
        }
    };
    let mut categories = HashMap::new();
    let c_json: Vec<Value>;
    println!("Getting categories");
    match reqwest::get(category_url).await {
        Ok(resp) => {
            c_json = resp.json::<Vec<Value>>().await?;
            println!("Found {} categories", c_json.len());
            for c in &c_json {
                let id = c["category_id"].as_str().unwrap_or_default();
                let name = c["category_name"].as_str().unwrap_or_default();
                categories.insert(id, name);
            }
        }
        Err(err) => println!("Error {err:?}"),
    }

    println!("Getting streams");
    writeln!(output, "#EXTM3U").expect("ERROR");
    match reqwest::get(stream_url).await {
        Ok(resp) => {
            let json = resp.json::<Vec<Value>>().await?;
            total_streams += json.len();
            println!("Found {} streams", json.len());
            println!("Creating m3u file {}", m3u_file);
            for c in json {
                let c_name = c["name"].as_str().unwrap_or_default();
                let c_id = c["category_id"].as_str().unwrap_or_default();
                let stream_id = match c["stream_id"].is_string() {
                    true => c["stream_id"].as_str().unwrap(),
                    false => &c["stream_id"].as_i64().unwrap().to_string(),
                };
                writeln!(
                    output,
                    "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}",
                    c["name"], c["stream_icon"], categories[c_id], c_name
                )
                .expect("ERROR");
                writeln!(
                    output,
                    "{}/{}/{}/{}{}",
                    args.server, args.username, args.password, stream_id, stream_ext
                )
                .expect("ERROR");
            }
        }
        Err(err) => {
            println!("Error {err:?}")
        }
    }

    if args.vod {
        let mut categories = HashMap::new();
        let c_json: Vec<Value>;
        println!("Getting VOD categories");
        match reqwest::get(vod_categories_url).await {
            Ok(resp) => {
                c_json = resp.json::<Vec<Value>>().await?;
                println!("Found {} VOD categories", c_json.len());
                for c in &c_json {
                    let id = c["category_id"].as_str().unwrap_or_default();
                    let name = c["category_name"].as_str().unwrap_or_default();
                    categories.insert(id, name);
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
        println!("Getting VOD streams");
        match reqwest::get(vod_streams_url).await {
            Ok(resp) => {
                let json = resp.json::<Vec<Value>>().await?;
                total_streams += json.len();
                println!("Found VOD {} streams", json.len());
                println!("Adding to m3u file {}", m3u_file);
                for c in json {
                    let c_name = c["name"].as_str().unwrap_or_default();
                    let c_id = c["category_id"].as_str().unwrap_or_default();
                    writeln!(
                        output,
                        "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}",
                        c["name"], c["stream_icon"], categories[c_id], c_name
                    )
                    .expect("ERROR");
                    writeln!(
                        output,
                        "{}/{}/{}/{}{}",
                        args.server, args.username, args.password, c["stream_id"], stream_ext
                    )
                    .expect("ERROR");
                }
            }
            Err(err) => {
                println!("Error {err:?}")
            }
        }
    }
    println!("Found {total_streams} streams");
    Ok(())
}
