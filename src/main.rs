use serde_json;
use std::{fs::File, io::Write};
use clap::Parser;
use reqwest;
use std::collections::HashMap;

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
	#[arg(short, long)]
	ts: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();
	
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
	
	let mut output = match File::create(&args.m3u_file) {
                        Ok(f) => f,
                        Err(e) => {
                            panic!("Error creating {:?}: {e:?}", args.m3u_file);
                        }
    };

	let mut categories = HashMap::new();
	let c_json: Vec<serde_json::Value>;
	println!("Getting categories");
	match reqwest::get(category_url).await {
		Ok(resp) => {
			let txt = resp.text().await?;
			c_json =  serde_json::from_str(&txt).expect("NONE");
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
	match reqwest::get(stream_url).await {
		Ok(resp) => {
			println!("Creating m3u file {}", args.m3u_file);
			let txt = resp.text().await?;
			let json: Vec<serde_json::Value>  = serde_json::from_str(&txt).expect("NONE");
			for c in json {
				let c_name = match c["name"].as_str() {
					Some(s) => s,
					_ => &String::new(),
				};
				let c_id = match c["category_id"].as_str() {
					Some(s) => s,
					_ => &String::new(),
				};
				writeln!(output, "EXTINF:-1 tvg-name={} tgv-logo={} group-title=\"{}\",{}", c["name"], c["stream_icon"], categories[c_id], c_name ).expect("ERROR");
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
	Ok(())
}
