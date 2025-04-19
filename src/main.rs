use chrono::DateTime;
use clap::Parser;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use static_str_ops::static_format;
use std::collections::HashMap;
use std::{
    fs::{File, read_to_string},
    io::Write,
};

#[derive(Parser, Debug, Clone)]
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
    #[arg(short, long, help = "Do not add a header to the M3U files")]
    no_header: bool,
    #[arg(short, long, help = "Create M3U/Diff for live channels")]
    live: bool,
    #[arg(short, long)]
    account_info: bool,
    #[arg(short, long)]
    diff: bool,
    #[arg(short = 'N', long, help = "Do not create M3U")]
    no_m3u: bool,
}

impl Args {
    fn get_ext(&self) -> String {
        match self.ts {
            true => ".ts".to_string(),
            false => "".to_string(),
        }
    }
}

trait ValueExtensions {
    fn get_name(&self) -> String;
    fn get_category_name(&self) -> &str;
    fn get_category_id(&self) -> &str;
    fn get_stream_id(&self) -> String;
    fn expires(&self) -> String;
    fn created(&self) -> String;
    fn max_connections(&self) -> i64;
    fn is_trial(&self) -> bool;
    fn status(&self) -> &str;
    fn active_cons(&self) -> &str;
}

impl ValueExtensions for Value {
    fn get_name(&self) -> String {
        self["name"].as_str().unwrap_or_default().to_string()
    }
    fn get_category_name(&self) -> &str {
        self["category_name"].as_str().unwrap_or_default()
    }
    fn get_category_id(&self) -> &str {
        self["category_id"].as_str().unwrap_or("-1")
    }
    fn get_stream_id(&self) -> String {
        match self["stream_id"].is_string() {
            true => self["stream_id"].as_str().unwrap().to_string(),
            false => self["stream_id"].as_i64().unwrap().to_string(),
        }
    }
    fn expires(&self) -> String {
        let exp_ts = match self["user_info"]["exp_date"].as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self["user_info"]["exp_date"].as_i64().unwrap_or_default(),
        };
        DateTime::from_timestamp(exp_ts, 0)
            .expect("Invalid Timestamp")
            .to_string()
    }
    fn created(&self) -> String {
        let created_ts = match self["user_info"]["created_at"].as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self["user_info"]["created_at"].as_i64().unwrap_or_default(),
        };
        DateTime::from_timestamp(created_ts, 0)
            .expect("Invalid Timestamp")
            .to_string()
    }
    fn max_connections(&self) -> i64 {
        match self["user_info"]["max_connections"].as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self["user_info"]["max_connections"]
                .as_i64()
                .unwrap_or_default(),
        }
    }
    fn is_trial(&self) -> bool {
        match self["user_info"]["is_trial"].is_boolean() {
            true => self["user_info"]["is_trial"].as_bool().unwrap(),
            false => matches!(self["user_info"]["is_trial"].as_str(), Some("1")),
        }
    }
    fn status(&self) -> &str {
        self["user_info"]["status"].as_str().unwrap_or_default()
    }
    fn active_cons(&self) -> &str {
        self["user_info"]["active_cons"]
            .as_str()
            .unwrap_or_default()
    }
}

#[derive(Debug)]
struct MFile {
    args: Args,
    //file_name: String,
    group_name: String,
    handle: Option<File>,
    vod: bool,
    all_channels: Vec<String>,
}

impl MFile {
    fn new(args: Args, group_name: String, vod: bool) -> MFile {
        let mut f_name =
            sanitise_file_name::sanitise(static_format!("{group_name}.m3u")).to_string();
        if vod {
            f_name = static_format!("vod_{f_name}").to_string();
        }
        let mut handle = None;
        if !args.no_m3u {
            handle = match File::create(&f_name) {
                Ok(f) => Some(f),
                Err(e) => panic!("Error creating {f_name:?}: {e:?}"),
            };
            if !args.no_header {
                if let Some(ref mut h) = handle {
                    writeln!(h, "#EXTM3U").expect("E")
                };
            }
        }
        MFile {
            args,
            group_name,
            handle,
            vod,
            all_channels: vec![],
        }
    }

    fn add_channel(&mut self, gname: String, chan: Value) {
        self.all_channels.push(chan.get_name());
        if !self.args.no_m3u {
            match self.handle {
                Some(ref mut h) => {
                    writeln!(
                        h,
                        "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}",
                        chan.get_name(),
                        chan["stream_icon"],
                        gname,
                        chan.get_name(),
                    )
                    .expect("ERROR");
                    writeln!(
                        h,
                        "{}/{}/{}/{}{}",
                        self.args.server,
                        self.args.username,
                        self.args.password,
                        chan.get_stream_id(),
                        self.args.get_ext()
                    )
                    .expect("ERROR");
                }
                _ => (),
            }
        }
    }
    fn make_diff_file(&mut self) -> (u32, u32) {
        let all_name: String;
        let diff_name: String;

        let now = chrono::offset::Local::now()
            .format("%Y%m%d_%H%M%S")
            .to_string();
        if self.vod {
            all_name =
                sanitise_file_name::sanitise(static_format!("all_vod_{}.txt", self.group_name))
                    .to_string();
            diff_name = sanitise_file_name::sanitise(static_format!(
                "all_vod_{}_diff_{now}.txt",
                self.group_name
            ))
            .to_string();
        } else {
            all_name = sanitise_file_name::sanitise(static_format!("all_{}.txt", self.group_name))
                .to_string();
            diff_name = sanitise_file_name::sanitise(static_format!(
                "all_{}_diff_{now}.txt",
                self.group_name
            ))
            .to_string();
        }
        let original_contents = read_to_string(&all_name).unwrap_or_default();
        let mut all_handle = match File::create(&all_name) {
            Ok(f) => f,
            Err(e) => panic!("Error creating {all_name:?}: {e:?}"),
        };
        self.all_channels.sort();
        for c in self.all_channels.clone() {
            writeln!(all_handle, "{c}").expect("E");
        }
        let new_contents = read_to_string(&all_name).unwrap_or_default();
        let cdiff = TextDiff::from_lines(&original_contents, &new_contents);

        let mut changes = 0;
        let mut inserted: u32 = 0;
        let mut deleted: u32 = 0;
        if cdiff.ratio() < 1.0 {
            let mut diff_output = match File::create(&diff_name) {
                Ok(f) => f,
                Err(e) => panic!("Error creating diff file {diff_name}: {e:?}"),
            };
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
            println!("Added {inserted}, Deleted {deleted}, Total {changes} saved to {diff_name}");
        } else {
            println!("No changes for {}", self.group_name);
        }
        (inserted, deleted)
    }
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
    let stream_by_category_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_live_streams&category_id=",
        args.server, args.username, args.password
    );
    let vod_categories_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_vod_categories",
        args.server, args.username, args.password
    );
    let vod_streams_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_vod_streams&category_id=",
        args.server, args.username, args.password
    );

    let mut live_inserted = 0;
    let mut live_deleted = 0;
    let mut vod_inserted = 0;
    let mut vod_deleted = 0;
    let mut live_streams = 0;
    let mut vod_streams = 0;

    match reqwest::get(account_url).await {
        Ok(resp) => {
            if resp.status() != 200 {
                println!("Error {} getting account information", resp.status());
                println!("Verify that your username and password are correct");
                std::process::exit(1);
            }
            let a_json = resp.json::<Value>().await?;

            println!("Account Information:");
            println!(" Created: {}", a_json.created());
            println!(" Expires: {}", a_json.expires());
            println!(" Status: {}", a_json.status());
            println!(" Active Connections: {}", a_json.active_cons());
            println!(" Max Connections: {}", a_json.max_connections());
            println!(" Trial: {}", a_json.is_trial());
        }
        Err(err) => println!("Error: {err:?}"),
    }
    if args.account_info {
        std::process::exit(0);
    }

    if args.live {
        let mut categories = HashMap::new();
        let c_json: Vec<Value>;
        println!("Getting categories");
        match reqwest::get(category_url).await {
            Ok(resp) => {
                c_json = resp.json::<Vec<Value>>().await?;
                println!("Found {} categories", c_json.len());
                for c in &c_json {
                    categories.insert(c.get_category_id(), c.get_category_name());
                    match reqwest::get(format!("{}{}", stream_by_category_url, c.get_category_id()))
                        .await
                    {
                        Ok(s_resp) => {
                            let s_json = s_resp.json::<Vec<Value>>().await?;
                            println!(
                                "Found {} streams in {}",
                                s_json.len(),
                                c.get_category_name()
                            );
                            live_streams += s_json.len();
                            let mut m3u_file =
                                MFile::new(args.clone(), c.get_category_name().to_string(), false);
                            for stream in &s_json {
                                m3u_file
                                    .add_channel(c.get_category_name().to_string(), stream.clone());
                            }
                            if args.diff {
                                let (ins, del) = m3u_file.make_diff_file();
                                live_inserted += ins;
                                live_deleted += del;
                            }
                        }
                        Err(err) => println!("Error {err:?}"),
                    };
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
    }

    if args.vod {
        let c_json: Vec<Value>;
        println!("Getting VOD categories");
        match reqwest::get(vod_categories_url).await {
            Ok(resp) => {
                c_json = resp.json::<Vec<Value>>().await?;
                println!("Found {} VOD categories", c_json.len());
                for c in &c_json {
                    match reqwest::get(format!("{}{}", vod_streams_url, c.get_category_id())).await
                    {
                        Ok(s_resp) => {
                            let s_json = s_resp.json::<Vec<Value>>().await?;
                            println!(
                                "Found {} streams in {}",
                                s_json.len(),
                                c.get_category_name()
                            );
                            vod_streams += s_json.len();
                            let mut m3u_file =
                                MFile::new(args.clone(), c.get_category_name().to_string(), true);
                            for stream in &s_json {
                                m3u_file
                                    .add_channel(c.get_category_name().to_string(), stream.clone());
                            }
                            if args.diff {
                                let (ins, del) = m3u_file.make_diff_file();
                                vod_inserted += ins;
                                vod_deleted += del;
                            }
                        }
                        Err(err) => println!("Error {err:?}"),
                    }
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
    }
    if !args.no_m3u {
        println!("Live Streams: {live_streams}");
        println!("VOD Streams: {vod_streams}");
        println!("Total Streams: {}", live_streams + vod_streams);
    }

    if args.diff {
        println!("Live channel changes: Added {live_inserted}, Deleted {live_deleted}");
        println!("VOD channel changes: Added {vod_inserted}, Deleted {vod_deleted}");
        println!(
            "Total changed: Added {}, Deleted {}",
            live_inserted + vod_inserted,
            live_deleted + vod_deleted
        );
    }
    Ok(())
}
