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
	#[arg(short, long)]
	m3u_file: String,
	#[arg(short, long,  help = "Append .ts to stream URLs")]
	ts: bool,
	#[arg(short, long, help = "Include VOD streams")]
	vod: bool,
	#[arg(short = 'S', long, help = "Include Series streams")]
	series: bool,
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
	let vod_categories_url = format!("{}/player_api.php?username={}&password={}&action=get_vod_categories",
							 args.server,
							 args.username,
							 args.password
	);
	let series_categories_url = format!("{}/player_api.php?username={}&password={}&action=get_series_categories",
							 args.server,
							 args.username,
							 args.password
	);
	let vod_streams_url = format!("{}/player_api.php?username={}&password={}&action=get_vod_streams",
							 args.server,
							 args.username,
							 args.password
	);
	let series_streams_url = format!("{}/player_api.php?username={}&password={}&action=get_series",
							 args.server,
							 args.username,
							 args.password
	);
	let stream_ext = match args.ts {
		true => ".ts",
		false => "",
	};
	
	let mut output = match File::create(&args.m3u_file) {
                        Ok(f) => f,
                        Err(e) => {
                            panic!("Error creating {:?}: {e:?}", args.m3u_file);
                        }
    };

	let mut total_streams = 0;
	let a_json: serde_json::Value;
	match reqwest::get(account_url).await {
		Ok(resp) => {
			let txt = resp.text().await?;
			a_json = serde_json::from_str(&txt).expect("NONE");
			let ts: i64 = match a_json["user_info"]["exp_date"].as_str() {
				Some(s) => s.parse().unwrap(),
				_ => 0,
			};
			let created: i64 = match a_json["user_info"]["created_at"].as_str() {
				Some(s) => s.parse().unwrap(),
				_ => 0,
			};
			println!("Account Information:");
			println!(" Created: {}", DateTime::from_timestamp(created, 0).expect("Invalid Timestamp").to_string());
			println!(" Expires: {}", DateTime::from_timestamp(ts, 0).expect("Invalid Timestamp").to_string());
			println!(" Status: {}", a_json["user_info"]["status"]);
			println!(" Active Connections: {}", a_json["user_info"]["active_cons"]);
			println!(" Max Connections: {}", a_json["user_info"]["max_connections"]);
			println!(" Trial: {}", a_json["user_info"]["is_trial"]);
				
		},
		Err(err) => println!("Error: {err:?}")
	}
	

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
			total_streams += json.len();
			println!("Found {} streams", json.len());
			println!("Creating m3u file {}", args.m3u_file);
			for c in json {
				let c_name = match c["name"].as_str() {
					Some(s) => s,
					_ => &String::new(),
				};
				let c_id = match c["category_id"].as_str() {
					Some(s) => s,
					_ => &String::new(),
				};
				writeln!(output, "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}", c["name"], c["stream_icon"], categories[c_id], c_name ).expect("ERROR");
				writeln!(output, "{}/{}/{}/{}{}",
						 args.server,
						 args.username,
						 args.password,
						 c["stream_id"],
						 stream_ext).expect("ERROR");
			}
		},
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
				c_json =  serde_json::from_str(&txt).expect("NONE");
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
			},
			Err(err) => println!("Error {err:?}")
		}
		println!("Getting VOD streams");
		match reqwest::get(vod_streams_url).await {
			Ok(resp) => {
				let txt = resp.text().await?;
				let json: Vec<serde_json::Value>  = serde_json::from_str(&txt).expect("NONE");
				total_streams += json.len();
				println!("Found VOD {} streams", json.len());
				println!("Adding to m3u file {}", args.m3u_file);
				for c in json {
					let c_name = match c["name"].as_str() {
						Some(s) => s,
						_ => &String::new(),
					};
					let c_id = match c["category_id"].as_str() {
						Some(s) => s,
						_ => &String::new(),
					};
					writeln!(output, "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}", c["name"], c["stream_icon"], categories[c_id], c_name ).expect("ERROR");
					writeln!(output, "{}/{}/{}/{}{}",
							 args.server,
							 args.username,
							 args.password,
							 c["stream_id"],
							 stream_ext).expect("ERROR");
				}
			},
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
				let txt = resp.text().await?;
				c_json =  serde_json::from_str(&txt).expect("NONE");
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
			},
			Err(err) => println!("Error {err:?}")
		}
		println!("Getting Series streams");
		match reqwest::get(series_streams_url).await {
			Ok(resp) => {
				let txt = resp.text().await?;
				let json: Vec<serde_json::Value>  = serde_json::from_str(&txt).expect("NONE");
				total_streams += json.len();
				println!("Found Series {} streams", json.len());
				println!("Adding to m3u file {}", args.m3u_file);
				for c in json {
					let c_name = match c["name"].as_str() {
						Some(s) => s,
						_ => &String::new(),
					};
					let c_id = match c["category_id"].as_str() {
						Some(s) => s,
						_ => &String::new(),
					};
					writeln!(output, "#EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}", c["name"], c["stream_icon"], categories[c_id], c_name ).expect("ERROR");
					writeln!(output, "{}/{}/{}/{}{}",
							 args.server,
							 args.username,
							 args.password,
							 c["stream_id"],
							 stream_ext).expect("ERROR");
				}
			},
			Err(err) => {
				println!("Error {err:?}")
			}
		}
	}
	println!("Found {total_streams} streams");
	Ok(())
}
