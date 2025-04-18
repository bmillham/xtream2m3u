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
    #[arg(short, long)]
    m3u_file: Option<String>,
    #[arg(short, long)]
    account_info: bool,
    #[arg(short, long)]
    diff: bool,
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
struct VCat {
    cat_name: String,
    file_name: String,
    file_handle: File,
    stream_count: Cell<i32>,
}

#[derive(Debug)]
struct MFile {
    args: Args,
    name: String,
    all_name: Option<String>,
    handle: File,
    all_handle: Option<File>,
}

impl MFile {
    fn new(args: Args, name: String) -> MFile {
        let mut f_name = sanitise_file_name::sanitise(static_format!("{name}.m3u")).to_string();
        if args.vod {
            f_name = static_format!("vod-{f_name}").to_string();
        }
        let mut all_name = None;
        let mut all_handle: Option<File> = None;
        if args.diff {
            all_name = Some(static_format!("all-{name}.txt").to_string());
            all_handle = match File::create(all_name.as_ref().unwrap()) {
                Ok(f) => Some(f),
                Err(e) => panic!("Error creating {all_name:?}: {e:?}"),
            };
        }
        // let original_contents = read_to_string("all_channels.txt").unwrap_or_default();
        let mut handle = match File::create(&f_name) {
            Ok(f) => f,
            Err(e) => panic!("Error creating {f_name:?}: {e:?}"),
        };

        if !args.no_header {
            writeln!(handle, "#EXTM3U").expect("E");
        }
        MFile {
            args,
            name: f_name,
            all_name,
            handle,
            all_handle,
        }
    }

    fn add_channel(&mut self, gname: String, chan: Value) {
        //let c_name = chan["name"].as_str().unwrap_or_default();
        let c_name = chan.get_name();
        let stream_id = match chan["stream_id"].is_string() {
            true => chan["stream_id"].as_str().unwrap(),
            false => &chan["stream_id"].as_i64().unwrap().to_string(),
        };
        writeln!(
            self.handle,
            "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}",
            chan["name"], chan["stream_icon"], gname, c_name
        )
        .expect("ERROR");
        writeln!(
            self.handle,
            "{}/{}/{}/{}{}",
            self.args.server,
            self.args.username,
            self.args.password,
            stream_id,
            self.args.get_ext()
        )
        .expect("ERROR");
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
    let stream_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_live_streams",
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

    let mut output: Option<File> = match args.clone().m3u_file {
        Some(f) => match File::create(&f) {
            Ok(file) => {
                println!("Creating m3u file {}", f);
                Some(file)
            }
            Err(e) => {
                panic!("Error creating {f}: {e:?}");
            }
        },
        _ => None,
    };

    let mut categories = HashMap::new();
    let mut streams_in_categories = HashMap::new();
    let c_json: Vec<Value>;
    println!("Getting categories");
    match reqwest::get(category_url).await {
        Ok(resp) => {
            c_json = resp.json::<Vec<Value>>().await?;
            println!("Found {} categories", c_json.len());
            for c in &c_json {
                let id = c.get_category_id();
                //let name = c["category_name"].as_str().unwrap_or_default();
                let name = c.get_category_name();
                categories.insert(id, name);
                streams_in_categories.insert(name, vec![]);
                match reqwest::get(format!("{}{}", stream_by_category_url, id)).await {
                    Ok(s_resp) => {
                        let s_json = s_resp.json::<Vec<Value>>().await?;
                        println!("Found {} streams in {}", s_json.len(), name);
                        let mut test_file = MFile::new(args.clone(), name.to_string());
                        for stream in &s_json {
                            test_file.add_channel(name.to_string(), stream.clone());
                        }
                    }
                    Err(err) => println!("Error {err:?}"),
                };
            }
        }
        Err(err) => println!("Error {err:?}"),
    }

    println!("Getting streams");
    let mut header_written = false;

    match reqwest::get(stream_url).await {
        Ok(resp) => {
            let json = resp.json::<Vec<Value>>().await?;
            total_streams += json.len();
            println!("Found {} streams", json.len());

            let mut all_chans: Vec<String> = Vec::new();
            for c in &json {
                let c_name = c["name"].as_str().unwrap_or_default();
                let c_id = c.get_category_id();
                let stream_id = match c["stream_id"].is_string() {
                    true => c["stream_id"].as_str().unwrap(),
                    false => &c["stream_id"].as_i64().unwrap().to_string(),
                };
                if let Some(val) = streams_in_categories.get_mut(categories[c_id]) {
                    val.push(c_name);
                };
                if let Some(ref mut o) = output {
                    if !header_written {
                        writeln!(o, "#EXTM3U").expect("ERROR");
                        header_written = true;
                    }
                    write_m3u_line(args.clone(), o, c.clone());
                    writeln!(
                        o,
                        "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}",
                        c["name"], c["stream_icon"], categories[c_id], c_name
                    )
                    .expect("ERROR");
                    writeln!(
                        o,
                        "{}/{}/{}/{}{}",
                        args.server, args.username, args.password, stream_id, stream_ext
                    )
                    .expect("ERROR");
                };
                all_chans.push(c_name.to_string());
            }
            /*for s in streams_in_categories {
                println!("{:?}", s.1);
            }*/
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
                let now = chrono::offset::Local::now()
                    .format("%Y%m%d_%H%M%S")
                    .to_string();
                let diff_filename = format!("all_channels_diff_{now}.txt");
                let mut diff_output = match File::create(&diff_filename) {
                    Ok(f) => {
                        println!("Created {diff_filename}");
                        f
                    }
                    Err(e) => panic!("Error creating diff file {diff_filename}: {e:?}"),
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
                    //let cat_name = c["category_name"].as_str().unwrap_or_default();
                    let cat_name = c.get_category_name();
                    let vcat = create_category_file(cat_name.to_string(), args.no_header, true);
                    vod_cats.insert(c.get_category_id(), vcat);
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
                    let c_id = c.get_category_id();
                    let stream_id = match c["stream_id"].is_string() {
                        true => c["stream_id"].as_str().unwrap(),
                        false => &c["stream_id"].as_i64().unwrap().to_string(),
                    };
                    if !vod_cats.contains_key(c_id) {
                        let vcat =
                            create_category_file("No_Category".to_string(), args.no_header, true);
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
    println!("Found {total_streams} total streams");
    Ok(())
}

fn write_m3u_line(args: Args, file: &File, c: Value) {
    let stream_ext = match args.ts {
        true => ".ts",
        false => "",
    };
    println!("holder {stream_ext} {c:?}");
}

fn create_category_file(cat_name: String, header: bool, vod: bool) -> VCat {
    let mut f_name = sanitise_file_name::sanitise(static_format!("{cat_name}.m3u")).to_string();
    if vod {
        f_name = static_format!("vod-{f_name}").to_string();
    }
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
