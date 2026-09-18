# Discord Bot

## Background

Provided a reason to learn discord library in Python and get introduced to Rust for the first time. By first prototyping in Python then translating to Rust. Adding features by just thinking of certain tools I want to learn about, e.g. ffmpeg and steamapi.

Now rust version is up-to-date with python all new updates will only be in Rust.

## Running the Bot

### Setting of the environment
1. Make copy of .example.env called .env and fill in all the variables according to the name.
2. Download both `ffmpeg` and `yt-dlp`

### Discord

1. Create new Python environment by `python -m venv .discord`
2. Install dependencies `pip install -r requirements.txt`
2. Use `run.sh` or `python main.py` 

### Rust

1. Download rust
2. head to /rust/discord_bot/
3. Run command `cargo run`

### Notes

- Must be in directory to run program. If outside env variables will not be found.
- If getting yt-dlp errors make sure it is updated to the most recent version.