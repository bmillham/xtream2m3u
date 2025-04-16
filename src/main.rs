use chrono::DateTime;
use clap::Parser;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use static_str_ops::static_format;
use std::collections::HashMap;
use std::{
    cell::Cell,
    fs::{File, read_to_string},
    io::Write,
};

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
    #[arg(short, long, help = "Create a M3U for each VOD category")]
    vod: bool,
    #[arg(short = 'T', long, help = "Modify the stream URL for use in TVHeadend")]
    tvheadend_remux: bool,
    #[arg(short, long, help = "Do not add a header to the VOD M3U files")]
    no_vodm3u_header: bool,
    #[arg(short, long, group = "g")]
    m3u_file: Option<String>,
    #[arg(short, long, group = "g")]
    account_info: bool,
    #[arg(short, long)]
    diff: bool,
}

#[derive(Debug)]
struct VCat {
    cat_name: String,
    file_name: String,
    file_handle: File,
    stream_count: Cell<i32>,
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
            let mut all_chans: Vec<String> = Vec::new();
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
                all_chans.push(c_name.to_string());
            }
            if args.diff {
                let original_contents = read_to_string("all_channels.txt").unwrap_or_default();
                let mut output = match File::create("all_channels.txt") {
                    Ok(f) => f,
                    Err(e) => panic!("Error creating file: {e:?}"),
                };
                all_chans.sort();
                for c in all_chans {
                    writeln!(output, "{c}").expect("ERROR");
                }
                let new_contents = read_to_string("all_channels.txt").unwrap_or_default();
                let cdiff = TextDiff::from_lines(&original_contents, &new_contents);
                let mut diff_output = match File::create("all_channels_diff.txt") {
                    Ok(f) => f,
                    Err(e) => panic!("Error creating diff file: {e:?}"),
                };
                let mut changes = 0;
                let mut inserted = 0;
                let mut deleted = 0;
                for change in cdiff.iter_all_changes() {
                    let sign = match change.tag() {
                        ChangeTag::Delete => {
                            deleted += 1;
                            "-"
                        }
                        ChangeTag::Insert => {
                            inserted += 1;
                            "+"
                        }
                        ChangeTag::Equal => " ",
                    };
                    if sign != " " {
                        write!(diff_output, "{sign} {change}").expect("ERROR");
                        changes += 1;
                    }
                }
                println!("Added {inserted}, Deleted {deleted}, Total {changes}");
            }
        }
        Err(err) => {
            println!("Error {err:?}")
        }
    }

    if args.vod {
        let mut vod_cats = HashMap::new();
        let c_json: Vec<Value>;
        println!("Getting VOD categories");
        match reqwest::get(vod_categories_url).await {
            Ok(resp) => {
                c_json = resp.json::<Vec<Value>>().await?;
                println!("Found {} VOD categories", c_json.len());
                for c in &c_json {
                    let cat_name = c["category_name"].as_str().unwrap_or_default();
                    let vcat = create_category_file(cat_name.to_string(), args.no_vodm3u_header);
                    vod_cats.insert(c["category_id"].as_str().unwrap_or_default(), vcat);
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
                println!("Adding to VOD m3u files");
                for c in &json {
                    let c_name = c["name"].as_str().unwrap_or_default();
                    let c_id = c["category_id"].as_str().unwrap_or("-1");
                    let stream_id = match c["stream_id"].is_string() {
                        true => c["stream_id"].as_str().unwrap(),
                        false => &c["stream_id"].as_i64().unwrap().to_string(),
                    };
                    if !vod_cats.contains_key(c_id) {
                        let vcat =
                            create_category_file("No_Category".to_string(), args.no_vodm3u_header);
                        vod_cats.insert(c_id, vcat);
                    }
                    vod_cats[c_id]
                        .stream_count
                        .set(vod_cats[c_id].stream_count.get() + 1);
                    writeln!(
                        &vod_cats[c_id].file_handle,
                        "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}",
                        c["name"], c["stream_icon"], vod_cats[c_id].cat_name, c_name
                    )
                    .expect("ERROR");
                    let url = format!(
                        "{}/movie/{}/{}/{}.{}",
                        args.server,
                        args.username,
                        args.password,
                        stream_id,
                        c["container_extension"].as_str().unwrap_or_default()
                    );
                    if args.tvheadend_remux {
                        writeln!(
							&vod_cats[c_id].file_handle,
							"pipe:///usr/bin/ffmpeg -loglevel 0 -re -i  {url} -c copy -flags +global_headers -f mpegts pipe:1",
						).expect("ERROR");
                    } else {
                        writeln!(&vod_cats[c_id].file_handle, "{url}").expect("ERROR");
                    }
                }
                let mut sorted: Vec<_> = vod_cats.iter().collect();
                sorted.sort_by_key(|a| &a.1.cat_name);
                for c in sorted {
                    println!(
                        "{} ({}): has {} streams",
                        c.1.cat_name,
                        c.1.file_name,
                        c.1.stream_count.get(),
                    );
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

fn create_category_file(cat_name: String, header: bool) -> VCat {
    let f_name = sanitise_file_name::sanitise(static_format!("vod-{cat_name}.m3u"));
    let mut cat_output = match File::create(&f_name) {
        Ok(f) => f,
        Err(e) => {
            panic!("Error creating {f_name:?}: {e:?}");
        }
    };
    if !header {
        writeln!(cat_output, "#EXTM3U").expect("ERROR");
    }
    VCat {
        cat_name: cat_name.to_string(),
        file_name: f_name,
        file_handle: cat_output,
        stream_count: Cell::new(0),
    }
}
