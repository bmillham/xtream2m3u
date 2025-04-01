use chrono::DateTime;
use clap::Parser;
use reqwest;
use serde_json;
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
    #[arg(short = 'S', long, help = "Include Series streams")]
    series: bool,
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
    let series_categories_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series_categories",
        args.server, args.username, args.password
    );
    let vod_streams_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_vod_streams",
        args.server, args.username, args.password
    );
    let series_streams_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series",
        args.server, args.username, args.password
    );
    let series_info_url = format!(
        "{}/player_api.php?username={}&password={}&action=get_series_info&series_id=",
        args.server, args.username, args.password
    );

    let stream_ext = match args.ts {
        true => ".ts",
        false => "",
    };

    let mut total_streams = 0;
    let a_json: serde_json::Value;
    match reqwest::get(account_url).await {
        Ok(resp) => {
            if resp.status() != 200 {
                println!("Error {} getting account information", resp.status());
                println!("Verify that your username and password are correct");
                std::process::exit(1);
            }
            let txt = match resp.text().await {
                Ok(t) => t,
                Err(e) => panic!("Error: {e:?}"),
            };

            a_json = match serde_json::from_str(&txt) {
                Ok(j) => j,
                Err(e) => panic!("Error getting json: {e:?}"),
            };
            let expires: i64 = match a_json["user_info"]["exp_date"].as_str() {
                Some(s) => s.parse().unwrap(),
                _ => match a_json["user_info"]["exp_date"].as_i64() {
                    Some(n) => n,
                    _ => 0,
                },
            };
            let created: i64 = match a_json["user_info"]["created_at"].as_str() {
                Some(s) => s.parse().unwrap(),
                _ => match a_json["user_info"]["created_at"].as_i64() {
                    Some(n) => n,
                    _ => 0,
                },
            };
            let is_trial: bool = match a_json["user_info"]["is_trial"].is_boolean() {
                true => a_json["user_info"]["is_trial"].as_bool().unwrap(),
                false => match a_json["user_info"]["is_trial"].as_str() {
                    Some("1") => true,
                    _ => false,
                },
            };

            println!("Account Information:");
            println!(
                " Created: {}",
                DateTime::from_timestamp(created, 0)
                    .expect("Invalid Timestamp")
                    .to_string()
            );
            println!(
                " Expires: {}",
                DateTime::from_timestamp(expires, 0)
                    .expect("Invalid Timestamp")
                    .to_string()
            );
            println!(" Status: {}", a_json["user_info"]["status"]);
            println!(
                " Active Connections: {}",
                a_json["user_info"]["active_cons"]
            );
            println!(
                " Max Connections: {}",
                a_json["user_info"]["max_connections"]
            );
            println!(" Trial: {is_trial}");
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
    let c_json: Vec<serde_json::Value>;
    println!("Getting categories");
    match reqwest::get(category_url).await {
        Ok(resp) => {
            let txt = resp.text().await?;
            c_json = serde_json::from_str(&txt).expect("NONE");
            println!("Found {} categories", c_json.len());
            for c in &c_json {
                let id = match c["category_id"].as_str() {
                    Some(s) => s,
                    _ => "",
                };
                let name = match c["category_name"].as_str() {
                    Some(s) => s,
                    _ => "",
                };
                categories.insert(id, name);
            }
        }
        Err(err) => println!("Error {err:?}"),
    }

    println!("Getting streams");
    writeln!(output, "#EXTM3U").expect("ERROR");
    match reqwest::get(stream_url).await {
        Ok(resp) => {
            let txt = resp.text().await?;
            let json: Vec<serde_json::Value> = serde_json::from_str(&txt).expect("NONE");
            total_streams += json.len();
            println!("Found {} streams", json.len());
            println!("Creating m3u file {}", m3u_file);
            for c in json {
                let c_name = match c["name"].as_str() {
                    Some(s) => s,
                    _ => &String::new(),
                };
                let c_id = match c["category_id"].as_str() {
                    Some(s) => s,
                    _ => &String::new(),
                };
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
        let c_json: Vec<serde_json::Value>;
        println!("Getting VOD categories");
        match reqwest::get(vod_categories_url).await {
            Ok(resp) => {
                let txt = resp.text().await?;
                c_json = serde_json::from_str(&txt).expect("NONE");
                println!("Found {} VOD categories", c_json.len());
                for c in &c_json {
                    let id = match c["category_id"].as_str() {
                        Some(s) => s,
                        _ => "",
                    };
                    let name = match c["category_name"].as_str() {
                        Some(s) => s,
                        _ => "",
                    };
                    categories.insert(id, name);
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
        println!("Getting VOD streams");
        match reqwest::get(vod_streams_url).await {
            Ok(resp) => {
                let txt = resp.text().await?;
                let json: Vec<serde_json::Value> = serde_json::from_str(&txt).expect("NONE");
                total_streams += json.len();
                println!("Found VOD {} streams", json.len());
                println!("Adding to m3u file {}", m3u_file);
                for c in json {
                    let c_name = match c["name"].as_str() {
                        Some(s) => s,
                        _ => &String::new(),
                    };
                    let c_id = match c["category_id"].as_str() {
                        Some(s) => s,
                        _ => &String::new(),
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
    if args.series {
        let mut categories = HashMap::new();
        let c_json: Vec<serde_json::Value>;
        println!("Getting Series categories");
        match reqwest::get(series_categories_url).await {
            Ok(resp) => {
                let txt = match resp.text().await {
                    Ok(s) => s,
                    _ => String::new(),
                };
                println!("txt {txt:?}");
                c_json = serde_json::from_str(&txt).expect("NONE");
                println!("Found {} Series categories", c_json.len());
                for c in &c_json {
                    let id = match c["category_id"].as_str() {
                        Some(s) => s,
                        _ => "",
                    };
                    let name = match c["category_name"].as_str() {
                        Some(s) => s,
                        _ => "",
                    };
                    categories.insert(id, name);
                }
            }
            Err(err) => println!("Error {err:?}"),
        }
        println!("Getting Series streams");
        match reqwest::get(series_streams_url).await {
            Ok(resp) => {
                let txt = resp.text().await?;
                let json: Vec<serde_json::Value> = serde_json::from_str(&txt).expect("NONE");
                total_streams += json.len();
                println!("Found Series {} streams", json.len());
                println!("Adding to m3u file {}", m3u_file);
                for c in json {
                    //println!("{c:?}");
                    let c_name = match c["name"].as_str() {
                        Some(s) => s,
                        _ => &String::new(),
                    };
                    let c_id = match c["category_id"].as_str() {
                        Some(s) => s,
                        _ => &String::new(),
                    };
                    println!("Series id {:?}", c["series_id"]);
                    //let sinf_url = format!("{}{}", series_info_url, c["series_id"]);
                    let sinf_url = format!("{}{}", series_info_url, 9193);
                    //let series_info_json: Vec<serde_json::Value>;
                    match reqwest::get(sinf_url.clone()).await {
                        Ok(resp) => {
                            let itxt = resp.text().await?;
                            let series_info_json: serde_json::Value =
                                serde_json::from_str(&itxt).expect("NONE");
                            //println!("Series info json {:?}", series_info_json);
                            /*match series_info_json["episodes"].is_object() {
                                true => continue,
                                false => {
                                    println!("This is not an object! {}", series_info_json["episodes"].is_array());
                                    //std::process::exit(0)
                                },
                            };*/
                            let mut hm: HashMap<String, serde_json::Value> = HashMap::new();

                            let eps: HashMap<String, serde_json::Value> =
                                match series_info_json["episodes"].is_object() {
                                    true => {
                                        for e in series_info_json["episodes"].as_object().unwrap() {
                                            let (key, v) = e;
                                            hm.insert(key.to_string(), v.clone());
                                        }
                                        hm
                                    }
                                    false => {
                                        hm.insert(
                                            "1".to_string(),
                                            vec![series_info_json["episodes"].clone()].into(),
                                        );
                                        hm
                                    }
                                };
                            /*for e in series_info_json["episodes"].as_object().unwrap() {*/
                            //for e in series_info_json["episodes"].as_array().unwrap() {
                            for e in eps {
                                //println!("e {e:?}");
                                //let sarr: Vec<serde_json::Value> = serde_json::from_str(&e.to_string()).expect("NONE");
                                let (key, v) = e;
                                println!("key {key}");
                                let sarr: Vec<serde_json::Value> = match v.is_array() {
                                    true => vec![v.clone()],
                                    //let sarr: Vec<serde_json::Value> = serde_json::from_str(&v.to_string()).expect("NONE");
                                    false => serde_json::from_str(&v.to_string()).expect("NONE"),
                                };
                                println!("Found {} episodes in {} {:?}", sarr.len(), key, v);
                                //println!("Found {} episodes in {:?}", sarr.len(), e);*/
                                for si in sarr {
                                    //println!("{si:?}");
                                    /*let title = match Some(si["title"]) {
                                            Some(ref t) => t.as_str().unwrap(),
                                            _ => &String::new(),
                                    };*/
                                    println!("si {:?}", si.is_array());
                                    match si.is_array() {
                                        true => {
                                            for sii in si.as_array() {
                                                //while let Some(sii) = si.as_array() {
                                                println!("sii len {:?} {}", sii, sii.len());
                                                //std::process::exit(0);
                                                /*for x in sii.clone() {
                                                    //println!("len x {:?}", x);
                                                    //println!("len x {:?}", x[0]);
                                                    for y in vec![x[0].clone()] {
                                                        println!("{:?}", y["title"]);
                                                    };
                                                };*/
                                                /*println!("{sii:?}");
                                                for x1 in vec![sii] {
                                                    println!("LEn x1 {:?} ", x1);
                                                    for x in x1.as_array() {
                                                        //println!("{}.{} {}",  x["id"].as_str().unwrap(), x["container_extension"].as_str().unwrap(), x["title"].as_str().unwrap());
                                                        //println!("{} {:?}", sii.len(), sii[0][0]);
                                                        //println!("x {:?}, {} {:?}", x[0], x.is_array(), x[0]["id"]);
                                                        println!("x  {:?} {:?}", x, x.as_array());
                                                    };
                                                };*/
                                            }
                                        }
                                        false => println!(
                                            "{}.{} {}",
                                            si["id"].as_str().unwrap(),
                                            si["container_extension"].as_str().unwrap(),
                                            si["title"].as_str().unwrap()
                                        ),
                                    };
                                }
                            }
                            std::process::exit(0);
                            /*series_info_json =  serde_json::from_str(&itxt).expect("NONE");*/
                            /*for s in series_info_json["episodes"]["1"] {
                                println!("{s:?}");
                            }*/
                        }
                        Err(err) => {
                            println!("{err:?}")
                        }
                    };

                    /*println!("{sinf_url}");
                    writeln!(output, "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}", c["name"], c["stream_icon"], categories[c_id], c_name ).expect("ERROR");
                    writeln!(output, "{}/{}/{}/{}{}",
                             args.server,
                             args.username,
                             args.password,
                             c["stream_id"],
                             stream_ext).expect("ERROR");*/
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
