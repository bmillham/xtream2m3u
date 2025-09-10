use chrono::DateTime;
use clap::Parser;
use serde_json::Value;
use similar::{ChangeTag, TextDiff};
use static_str_ops::static_format;
use std::fmt::Write as FmtWrite;
use std::{
    fs::{File, create_dir_all, read_to_string},
    io::Write,
    path::PathBuf,
    collections::HashMap,
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
    #[arg(
        short,
        long,
        help = "Append .ts to stream URLs",
		num_args = 0..=1,
        default_value = "",
        default_missing_value = ".ts",
    )]
    ts: String,
    #[arg(short, long, help = "Create a M3U for each VOD category")]
    vod: bool,
    #[arg(long, help = "Create a M3U for Series")]
    series: bool,
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
    #[arg(short, long, help = "Create M3U files")]
    m3u: bool,
    #[arg(short = 'S', long, help = "Create a single M3U file")]
    single_m3u: bool,
    #[arg(
        short,
        long,
        help = "Where to save M3U/Diff files",
        default_value = "."
    )]
    output_dir: String,
}

trait ValueExtensions {
    fn get_name(&self) -> String;
    fn get_epg_id(&self) -> String;
    fn get_category_name(&self) -> &str;
    fn get_category_id(&self) -> &str;
    fn get_stream_id(&self) -> String;
    fn get_series_name(&self) -> &str;
    fn get_series_id(&self) -> String;
    fn get_episode_id(&self) -> String;
    fn expires(&self) -> String;

    fn created(&self) -> String;
    fn max_connections(&self) -> i64;
    fn is_trial(&self) -> bool;
    fn status(&self) -> &str;
    fn active_cons(&self) -> &str;
    fn get_ext(&self) -> String;
}

impl ValueExtensions for Value {
    fn get_name(&self) -> String {
        self["name"].as_str().unwrap_or_default().to_string()
    }
    fn get_epg_id(&self) -> String { self["epg_channel_id"].as_str().unwrap_or_default().to_string() }
    fn get_category_name(&self) -> &str {
        self["category_name"].as_str().unwrap_or_default()
    }
    fn get_series_name(&self) -> &str {self["category_name"].as_str().unwrap_or_default()}
    fn get_category_id(&self) -> &str {
        self["category_id"].as_str().unwrap_or("-1")
    }
    fn get_series_id(&self) -> String {
        //println!("{self:?}");
        //println!("Getting series id {:?}", self["id"]);
        //if self["series_id"].is_null() { return "-1".to_string() }
        if self["series_id"].is_null() { 
            return self.get_episode_id()
        }
        match self["series_id"].is_string() {
            true => self["series_id"].as_str().unwrap().to_string(),
            false => self["series_id"].as_i64().unwrap().to_string(),
        }
    }
    fn get_episode_id(&self) -> String {
        if self["id"].is_null() { return "-1".to_string() }
        match self["id"].is_string() {
            true => self["id"].as_str().unwrap().to_string(),
            false => self["id"].as_i64().unwrap().to_string(),
        }
    }

    fn get_stream_id(&self) -> String {
        match self["stream_id"].is_string() {
            true => self["stream_id"].as_str().unwrap().to_string(),
            false => self["stream_id"].as_i64().unwrap().to_string(),
        }
    }

    fn get_ext(&self) -> String {
        let x = self["container_extension"].as_str().unwrap_or_default();
        match x.is_empty() {
            true => "".to_string(),
            false => format!(".{x}"),
        }
    }
    fn expires(&self) -> String {
        let exp_ts = match self["user_info"]["exp_date"].as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self["user_info"]["exp_date"].as_i64().unwrap_or_default(),
        };
        DateTime::from_timestamp(exp_ts, 0)
            .unwrap_or_default()
            .to_string()
    }
    fn created(&self) -> String {
        let created_ts = match self["user_info"]["created_at"].as_str() {
            Some(s) => s.parse().unwrap(),
            _ => self["user_info"]["created_at"].as_i64().unwrap_or_default(),
        };
        DateTime::from_timestamp(created_ts, 0)
            .unwrap_or_default()
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
struct ChanGroup {
    args: Args,
    group_name: String,
    file_name: String,
    file_created: bool,
    m3u_dir: PathBuf,
    diff_dir: PathBuf,
    handle: Option<File>,
    all_channels: Vec<String>,
    group_type: String,
}

impl ChanGroup {
    fn new(args: Args, group_name: String, group_type: String) -> ChanGroup {
        let m3u_dir: PathBuf = [&args.output_dir, &format!("{group_type}_m3u")].iter().collect();
        let diff_dir: PathBuf = [&args.output_dir, &format!("{group_type}_m3u")].iter().collect();
        let file_name = match args.single_m3u {
            false => sanitise_file_name::sanitise(static_format!("{group_name}.m3u")).to_string(),
            true => sanitise_file_name::sanitise(static_format!("all.m3u")).to_string(),
        };

        ChanGroup {
            args,
            group_name,
            file_name,
            file_created: false,
            m3u_dir,
            diff_dir,
            handle: None,
            all_channels: vec![],
            group_type,
        }
    }

    fn create_file(&mut self) -> std::io::Result<()> {
        if self.args.m3u {
            if let Ok(false) = std::fs::exists(&self.m3u_dir) {
                println!("Creating {:?}", self.m3u_dir);
                let _ = create_dir_all(&self.m3u_dir);
            };
        }
        
        if !self.file_created {
            println!("Creating {:?}", self.file_name);
            self.handle = match File::create(self.m3u_dir.join(self.file_name.clone())) {
                Ok(f) => Some(f),
                Err(e) => panic!("Error creating : {e:?}"),
            };
            if !self.args.no_header {
                if let Some(ref mut h) = self.handle {
                    writeln!(h, "#EXTM3U")?;
                }
            }
            self.file_created = true;
        } else {
            println!("Using {:?}", self.handle);
        }
        Ok(())
    }

    fn add_channel(&mut self, gname: String, chan: Value) -> std::io::Result<()> {
        self.all_channels.push(chan.get_name());
        if self.args.m3u {
            if let Some(ref mut h) = self.handle {
                writeln!(
                    h,
                    "#EXTINF:-1 tvg-id=\"{}\" tvg-name=\"{}\" tgv-logo={} group-title=\"{}\",\
                    {}",
                    chan.get_epg_id(),
                    chan.get_name(),
                    chan["stream_icon"],
                    gname,
                    chan.get_name(),
                )?;
                let ext = match chan.get_ext().is_empty() {
                    true => self.args.ts.clone(),
                    false => chan.get_ext(),
                };
                let mut server = self.args.server.clone();
                if self.group_type != "live" {
                    server += &*format!("/{}", self.group_type);
                }
                let sid = match self.group_type.as_str() {
                    "series" => chan.get_series_id(),
                    _ => chan.get_stream_id(),
                };
                //if self.group_type != "series" {
                //    sid = chan.get_stream_id();
                //} else {
                //    sid = chan.get_series_id();
                //}
                writeln!(
                    h,
                    "{}/{}/{}/{}{}",
                    server,
                    self.args.username,
                    self.args.password,
                    sid,
                    ext
                )?;
            }
        }
        Ok(())
    }
    fn make_diff_file(&mut self) -> Result<(u32, u32), std::io::Error> {
        let mut new_contents = String::new();

        let now = chrono::offset::Local::now()
            .format("%Y%m%d_%H%M%S")
            .to_string();
        if let Ok(false) = std::fs::exists(&self.diff_dir) {
            println!("Creating {:?}", self.diff_dir);
            let _ = create_dir_all(&self.diff_dir);
        };
        let all_name = self
            .diff_dir
            .join(sanitise_file_name::sanitise(static_format!(
                "{}_all.txt",
                self.group_name
            )));
        let diff_name = self
            .diff_dir
            .join(sanitise_file_name::sanitise(static_format!(
                "{}_diff_{now}.txt",
                self.group_name
            )));
        let all_exists = std::fs::exists(&all_name)?;
        let original_contents = read_to_string(&all_name).unwrap_or_default();
        let mut all_handle = match File::create(&all_name) {
            Ok(f) => f,
            Err(e) => panic!("Error creating {all_name:?}: {e:?}"),
        };
        self.all_channels.sort();
        for c in self.all_channels.clone() {
            writeln!(all_handle, "{c}")?;
            let _ = writeln!(&mut new_contents, "{c}");
        }

        let mut changes: u32 = 0;
        let mut inserted: u32 = 0;
        let mut deleted: u32 = 0;

        if all_exists {
            let cdiff = TextDiff::from_lines(&original_contents, &new_contents);

            if cdiff.ratio() < 1.0 {
                let mut diff_output = match File::create(&diff_name) {
                    Ok(f) => f,
                    Err(e) => panic!("Error creating diff file {diff_name:?}: {e:?}"),
                };
                for change in cdiff.iter_all_changes() {
                    match change.tag() {
                        ChangeTag::Delete => {
                            deleted += 1;
                            write!(diff_output, "- {}", change.value())?;
                        }
                        ChangeTag::Insert => {
                            inserted += 1;
                            write!(diff_output, "+ {}", change.value())?;
                        }
                        _ => (),
                    };
                    changes = inserted + deleted;
                }
                println!(
                    "Added {inserted}, Deleted {deleted}, Total {changes} saved to {diff_name:?}"
                );
            } else {
                println!("No changes for {}", self.group_name);
            }
        } else {
            println!("Not creating diff file since no previous file exists");
        }
        Ok((inserted, deleted))
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if (args.live || args.vod) && (!args.m3u && !args.diff) {
        eprintln!("You must use -m/--m3u and/or -d/--diff");
        std::process::exit(1);
    }
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
    let series_categories_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series_categories",
        args.server, args.username, args.password
    );
    let series_streams_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series&category_id=",
        args.server, args.username, args.password
    );
    let series_info_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series_info&series_id=",
        args.server, args.username, args.password);

    let mut live_inserted = 0;
    let mut live_deleted = 0;
    let mut vod_inserted = 0;
    let mut vod_deleted = 0;
    let mut live_streams = 0;
    let mut vod_streams = 0;
    let mut series_inserted = 0;
    let mut series_deleted = 0;
    let mut series_streams = 0;
    let mut series_no_episodes = 0;

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
        let c_json: Vec<Value>;
        println!("Getting categories");
        
        match reqwest::get(category_url).await {
            Ok(resp) => {
                c_json = resp.json::<Vec<Value>>().await?;
                println!("Found {} categories", c_json.len());
                let mut chan_group : ChanGroup = ChanGroup::new(
                    args.clone(),
                    "ALL".to_string(),
                    "live".to_string(),
                );
                if args.single_m3u && args.m3u {
                    let _ = chan_group.create_file();
                }
                for c in &c_json {
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
                            
                            if !args.single_m3u {
                                chan_group = ChanGroup::new(
                                    args.clone(),
                                    c.get_category_name().to_string(),
                                    "live".to_string(),
                                );
                                if args.m3u {
                                    let _ = chan_group.create_file();
                                }
                            }
                            for stream in &s_json {
                                let _ = chan_group
                                    .add_channel(c.get_category_name().to_string(), stream.clone());
                            }
                            if args.diff {
                                (live_inserted, live_deleted) = match chan_group.make_diff_file() {
                                    Ok((i, d)) => (i + live_inserted, d + live_deleted),
                                    Err(_) => (live_inserted, live_deleted),
                                };
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
                let mut chan_group : ChanGroup = ChanGroup::new(
                    args.clone(),
                    "ALL".to_string(),
                    "movie".to_string(),
                );
                if args.single_m3u && args.m3u {
                    let _ = chan_group.create_file();
                }
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
                            if !args.single_m3u {
                                chan_group = ChanGroup::new(
                                    args.clone(),
                                    c.get_category_name().to_string(),
                                    "movie".to_string(),
                                );
                                if args.m3u {
                                    let _ = chan_group.create_file();
                                }
                            }

                        for stream in &s_json {
                                let _ = chan_group
                                    .add_channel(c.get_category_name().to_string(), stream.clone());
                            }
                            if args.diff {
                                (vod_inserted, vod_deleted) = match chan_group.make_diff_file() {
                                    Ok((i, d)) => (vod_inserted + i, vod_deleted + d),
                                    Err(_) => (vod_inserted, vod_deleted),
                                };
                            }
                        }
                        Err(err) => println!("Error {err:?}"),
                    }
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
    }
    if args.series {
        let series_categories_json: Vec<Value>;
        println!("Getting Series categories");
        match reqwest::get(series_categories_url).await {
            Ok(resp) => {
                series_categories_json = resp.json::<Vec<Value>>().await?;
                println!("Found {} Series categories", series_categories_json.len());
                let mut chan_group : ChanGroup = ChanGroup::new(
                    args.clone(),
                    "ALL".to_string(),
                    "series".to_string(),
                );
                if args.single_m3u && args.m3u {
                    let _ = chan_group.create_file();
                }
                for c in &series_categories_json {
                    //println!("{:?}", c);
                    match reqwest::get(format!("{}{}", series_streams_url, c.get_category_id()))
                        .await
                    {
                        Ok(series_resp) => {
                            let series_json = series_resp.json::<Vec<Value>>().await?;
                            println!(
                                "Found {} series in {}",
                                series_json.len(),
                                c.get_series_name()
                            );
                            series_streams += series_json.len();
                            if !args.single_m3u {
                                chan_group = ChanGroup::new(
                                    args.clone(),
                                    c.get_series_name().to_string(),
                                    "series".to_string(),
                                );
                                if args.m3u {
                                    let _ = chan_group.create_file();
                                }
                            }

                            for series in &series_json {
                                //let _ = chan_group
                                //    .add_channel(c.get_series_name().to_string(), stream.clone());
                                //println!("STREAM {stream:?}");
                                let url = format!("{}{}", series_info_url, series.get_series_id
                                 ());
                                //println!("URL {}", url);
                                match reqwest::get(format!("{}{}", series_info_url, series
                                    .get_series_id())).await {
                                    Ok(season_resp) => {
                                        let season_json: HashMap<String, Value> = season_resp
                                            .json()
                                            .await?;
                                        if !season_json.contains_key("episodes") {
                                            println!("No episodes in series {}",
                                                     season_json["info"]["name"]);
                                            series_no_episodes += 1;
                                        } else if season_json["episodes"].is_array() {
                                            println!("WARNING: Array found in episodes {}",
                                                     season_json["info"]["name"].as_str().unwrap());
                                            let x = season_json["episodes"].as_array().unwrap();
                                            println!("Episode len {}", x.len());
                                            for i in x {
                                                for y in i.as_array().unwrap() {
                                                    //println!("{} {}", y["season"], y["title"]);
                                                    let _ = chan_group.add_channel(y["title"]
                                                                                       .to_string
                                                                                       (), y
                                                        .clone());
                                                    }
                                                }
                                        } else {
                                            let episodes_map: HashMap<String, Value> =
                                                serde_json::from_str
                                                    (&season_json["episodes"]
                                                        .to_string())?;
                                            let mut episodes: Vec<_> = episodes_map.iter().collect();
                                            episodes.sort_by_key(|a| a.0);
                                            //println!("Found {} Seasons {} (Reported), {} (Found)",
                                            //         season_json["info"]["name"].as_str()
                                            // .unwrap(),
                                            //         season_json["seasons"].as_array()
                                            //             .unwrap()
                                            //             .len(), episodes.iter().len
                                            //    ());
                                            for (k, v) in episodes.iter() {
                                                for i in v.as_array().unwrap() {
                                                    let _ = chan_group
                                                        .add_channel(i["title"].to_string
                                                        (),
                                                                     i.clone());
                                                    //println!("{} {} {}", k, i["id"], i["title"]);
                                                }
                                            }
                                        }
                                    }
                                Err(err) => println!("Error {err:?}"),
                            }
                        }
                        if args.diff {
                        (series_inserted, series_deleted) = match chan_group.make_diff_file
                                () {
                                    Ok((i, d)) => (series_inserted + i, series_deleted + d),
                                    Err(_) => (series_inserted, series_deleted),
                                };
                            }
                        }
                        Err(err) => println!("Error {err:?}"),
                    }
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
    }
    if args.m3u {
        if args.live {
            println!("Live Streams: {live_streams}");
        }
        if args.vod {
            println!("VOD Streams: {vod_streams}");
        }
        if args.series {
            println!("Series Streams: {series_streams}");
            println!("Series with no episodes: {series_no_episodes}");
        }
        println!("Total Streams: {}", live_streams + vod_streams);
    }

    if args.diff {
        if args.live {
            println!("Live channel changes: Added {live_inserted}, Deleted {live_deleted}");
        }
        if args.vod {
            println!("VOD channel changes: Added {vod_inserted}, Deleted {vod_deleted}");
        }
        println!(
            "Total changed: Added {}, Deleted {}",
            live_inserted + vod_inserted,
            live_deleted + vod_deleted
        );
    }
    Ok(())
}
