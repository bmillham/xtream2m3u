# xtream2m3u
A simple program to create a m3u with the given xtream credentials

# Installing
To install rust:\
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh\
This will install everything you need to use Rust

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
+ -t, --ts: Append a .ts to the stream location in the generated m3u
+ -m, --m3u-file: The name of the generated m3u file

cargo run -- options

The first time you run you will notice a lot of packages being downloaded and compiled.
This is normal.

# Building
If you want to run this from a cron job, etc you need to build the project. To do this just run

cargo build --release

And you will find xtream2n3u in target/release

Enjoy! And feedback is welcome!
