# A free reverse proxy and CLI tool for OpenAI GPT-3.5-turbo.
**[WIP]** 
It allows you to use the GPT-3.5 API without needing to sign up for an API key or pay for usage. 

## Features
- [x] REPL mode, you can input questions and get answers interactively
- [ ] Reverse proxy mode, you can use the GPT-3.5 API without needing to sign up for an API key or pay for usage
- [x] CLI mode, with shell pipe, file input, code output, etc.
- [x] Support https proxy

## Download precompiled binary

- [Linux](https://github.com/shenjinti/fgpt/releases/download/v0.1.1/fgpt-linux-v0.1.1.tar.gz) executable binary
- [Mac M1/M2](https://github.com/shenjinti/fgpt/releases/download/v0.1.1/fgpt-mac_aarch64.tar.gz) executable binary
- Windows (Coming soon)
- Or via [Docker](https://hub.docker.com/r/shenjinti/fgpt)

## Installation
```bash
cargo install fgpt
```
## How to use CLI

```bash
# To get help
fgpt "Linux command to list files in a directory"

# Output plain code -c/--code
fgpt -c "Write python code to reverse a string"

# With pipe
git diff | fgpt "Write a commit message for this diff"

# With stdin
fgpt "Convert CSV to JSON" < contacts.csv

# With file -f/--file
fgpt -f contacts.csv  "Convert CSV to JSON"

# REPL mode
fgpt
>> How to list files in a directory
...
```
### proxy options:
```bash
# 1. pass the proxy address by -p/--proxy
fgpt -p 'socks5://127.0.0.1:9080' "Linux command to list files in a directory"
# 2. pass the proxy address by environment variable
export HTTPS_PROXY='socks5://127.0.0.1:9080'
fgpt "Linux command to list files in a directory"
```
### dump stats
```bash
fgpt --stats "Linux command to list files in a directory"
```

## Use by docker
```bash
docker run -it --rm shenjinti/fgpt "Linux command to list files in a directory"
```

## How to use Reverse Proxy
**[WIP]**
```bash
fgpt -s 127.0.0.1:3000
```
