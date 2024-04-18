# A free reverse proxy and CLI tool for OpenAI GPT-3.5-turbo.
**[WIP]** 
It allows you to use the GPT-3.5 API without needing to sign up for an API key or pay for usage. 

## Features
- [x] REPL mode, you can input questions and get answers interactively
- [ ] Reverse proxy mode, you can use the GPT-3.5 API without needing to sign up for an API key or pay for usage
- [x] CLI mode, with shell pipe, file input, code output, etc.
- [x] Support https proxy

free gpt-3.5 turbo with a cli tool 

## Download precompiled binary

- [Linux](https://github.com/shenjinti/fgpt/releases/download/v0.1.1/fgpt-linux-v0.1.1.tar.gz) executable binary
- [Mac M1/M2](https://github.com/shenjinti/fgpt/releases/download/v0.1.1/fgpt-mac_aarch64.tar.gz) executable binary
- Windows (Coming soon)
- Or via [Docker](https://hub.docker.com/r/shenjinti/fgpt)
- Or build from source (see below, cargo is required)
    ```bash
    cargo install fgpt
    ```

## How to use

[![asciicast](https://asciinema.org/a/654921.svg)](https://asciinema.org/a/654921)

```bash
# To get answers from GPT-3.5
fgpt "How to get a domain's MX record on linux shell?"

# Output plain code -c/--code
fgpt -c "Write python code to reverse a string"

# With pipe
git diff | fgpt "Write a git commit bief with follow diff"

# With stdin
fgpt "Convert the follow csv data to json, without any description" < contacts.csv

# With file -f/--file
fgpt -f contacts.csv  "Convert the follow csv data to json, without any description"

# REPL mode
fgpt
>> Write a javascript code to reverse a string
...
```
### With http proxy:
If you are unable to connect , you can try using a proxy. HTTP and SOCKS5 proxies are supported. For example:

```bash
# 1. pass the proxy address by -p/--proxy
fgpt -p 'socks5://127.0.0.1:9080' "Linux command to list files in a directory"

# 2. pass the proxy address by environment variable
export HTTPS_PROXY='socks5://127.0.0.1:9080'
fgpt "Linux command to list files in a directory"

# 3. use alias to set the proxy address
alias fgpt='fgpt -p "socks5://127.0.0.1:9080"'
fgpt "Linux command to list files in a directory"
```

### Dump stats 
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
