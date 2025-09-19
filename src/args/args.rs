use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about=None)]
pub struct Args {
    #[arg(short, long)]
    pub server: String,
    #[arg(short, long)]
    pub username: String,
    #[arg(short, long)]
    pub password: String,
    #[arg(
        short,
        long,
        help = "Append .ts to stream URLs",
		num_args = 0..=1,
        default_value = "",
        default_missing_value = ".ts",
    )]
    pub ts: String,
    #[arg(short, long, help = "Create a M3U for each VOD category")]
    pub vod: bool,
    #[arg(long, help = "Create a M3U for Series")]
    pub series: bool,
    #[arg(short = 'T', long, help = "Modify the stream URL for use in TVHeadend")]
    pub tvheadend_remux: bool,
    #[arg(short, long, help = "Do not add a header to the M3U files")]
    pub no_header: bool,
    #[arg(short, long, help = "Create M3U/Diff for live channels")]
    pub live: bool,
    #[arg(short, long)]
    pub account_info: bool,
    #[arg(short, long)]
    pub diff: bool,
    #[arg(short, long, help = "Create M3U files")]
    pub m3u: bool,
    #[arg(short = 'S', long, help = "Create a single M3U file")]
    pub single_m3u: bool,
    #[arg(
        short,
        long,
        help = "Where to save M3U/Diff files",
        default_value = "."
    )]
    pub output_dir: String,
}

pub fn verify_args(args: &Args) -> bool {
    if (args.live || args.vod) && (!args.m3u && !args.diff) {
        eprintln!("You must use -m/--m3u and/or -d/--diff");
        false
    } else { true }
}