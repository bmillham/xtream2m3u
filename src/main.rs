use serde_json;
use std::{fs::File, io::Write};
use clap::Parser;
use reqwest;
use std::collections::HashMap;
use chrono::DateTime;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
	#[arg(short, long)]
	server: String,
	#[arg(short, long)]
	username: String,
	#[arg(short, long)]
	password: String,
	#[arg(short, long, group="g")]
	m3u_file: Option<String>,
	#[arg(short, long)]
	ts: bool,
	#[arg(short, long, group="g")]
	account_info: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();

	let account_url = format!("{}/player_api.php?username={}&password={}",
							   args.server,
							   args.username,
							   args.password
	);
	let category_url = format!("{}/player_api.php?username={}&password={}&action=get_live_categories",
							   args.server,
							   args.username,
							   args.password
	);
	let stream_url = format!("{}/player_api.php?username={}&password={}&action=get_live_streams",
							 args.server,
							 args.username,
							 args.password
	);
	
	let stream_ext = match args.ts {
		true => ".ts",
		false => "",
	};
	


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
				Some(s) => s.parse().unwrap() ,
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
			println!(" Created: {}", DateTime::from_timestamp(created, 0).expect("Invalid Timestamp").to_string());
			println!(" Expires: {}", DateTime::from_timestamp(expires, 0).expect("Invalid Timestamp").to_string());
			println!(" Status: {}", a_json["user_info"]["status"]);
			println!(" Active Connections: {}", a_json["user_info"]["active_cons"]);
			println!(" Max Connections: {}", a_json["user_info"]["max_connections"]);
			println!(" Trial: {is_trial}");
		},
		Err(err) => println!("Error: {err:?}")
	}
	if args.account_info {
		std::process::exit(0);
	}
	let m3u_file = match args.m3u_file {
		Some(f) => f,
		_ => {
			println!("No m3u file supplied");
			std::process::exit(0)
		},
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
			c_json =  serde_json::from_str(&txt).expect("NONE");
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
		},
		Err(err) => println!("Error {err:?}")
	}

	println!("Getting streams");
	writeln!(output, "#EXTM3U").expect("ERROR");
	match reqwest::get(stream_url).await {
		Ok(resp) => {
			let txt = resp.text().await?;
			let json: Vec<serde_json::Value>  = serde_json::from_str(&txt).expect("NONE");
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
					false => &format!("{}", c["stream_id"].as_i64().unwrap()),
				};
				writeln!(output, "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}", c["name"], c["stream_icon"], categories[c_id], c_name ).expect("ERROR");
				writeln!(output, "{}/{}/{}/{}{}",
						 args.server,
						 args.username,
						 args.password,
						 stream_id,
						 stream_ext).expect("ERROR");
			}
		},
		Err(err) => {
			println!("Error {err:?}")
		}
	}
	Ok(())
}
