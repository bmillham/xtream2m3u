# xtream2m3u
A simple program to create a m3u with the given xtream credentials

# Installing

## Install Rust

### Linux
To install rust:\
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\
This will install everything you need to use Rust

### Windows or Mac

Go to https::/www.rust-lan.org/tools/install and follow the instructions.

#### Windows Executable for WIn10/11

If you do not want to install Rust you can download this instead:\
https://github.com/bmillham/xtream2m3u/releases/download/v0.1.0/xtream2m3u.exe

## Install xtream2m3u

To install xtream2m3u, clone this to your projects directory:\
mkdir -p projects\
cd projects\
git clone https://github.com/bmillham/xtream2m3u

The project is now in projects/extream2m3u

cd extream2m3u

Everything from here on is done in the xtream2m3u directory.

# Options
+ -s, --server: The server name
+ -u, --username: Your user name
+ -p --password: Your password
+ -t, --ts: Append a .ts to the stream URL in the generated m3u
+ -l, --live: Use live channels
+ -v, --vod: Use VOD channels
+ -d, --diff: Create a timestamped file of changes
+ -N, --no-m3u: Do not create a M3U. Useful for just getting channel changes
+ -a, --acount-info: Only show the account information
+ -T, --tvheadend: Adds a remote call to ffmpeg for use in TVHeadend (Option does not do anything at this time)
+ -n, --no-header: Does not include the normal m3u header. Useful if you want to concatinate several m3u files.
+ -o, --output-dir: Directory to save output files in. Defaults to current directory.

New changes are that M3U files are now created per category instead of one large M3U file
The old -m option is removed.

Output files are saved in live\_m3u, live\_diff, vod\_m3u and vod\_diff

# Running

cargo run -- options

The first time you run you will notice a lot of packages being downloaded and compiled.
This is normal.

# Building
If you want to run this from a cron job, etc you need to build the project. To do this just run

cargo build --release

And you will find xtream2m3u in target/release

Enjoy! And feedback is welcome!
