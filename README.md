# A free reverse proxy and CLI tool for OpenAI GPT-3.5-turbo

It allows you to use the GPT-3.5 API without needing to sign up for an API key or pay for usage.
> 😄 OpenAI GPT-3.5-turbo is free to use, without any account or API key   
> 🔔 DON'T USE IN PRODUCTION, ONLY FOR PERSONAL USE/TESTING

## Features

- [x] **REPL** mode, you can input questions and get answers interactively
- [x] **Reverse proxy mode**, you can use the OpenAI OpenAPI with a local server
- [x] **CLI mode**, with shell pipe, file input, code output, etc.
- [x] 🔏 Support https proxy and socks5 proxy

## Download precompiled binary

- [Linux x64](https://github.com/shenjinti/fgpt/releases/download/v0.1.7/fgpt-linux_x64.tar.gz) executable binary
- [Mac M1/M2](https://github.com/shenjinti/fgpt/releases/download/v0.1.7/fgpt-mac_aarch64.tar.gz) executable binary
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
cat README.md | fgpt "summarize for reddit post"

# With stdin
fgpt "Convert the follow csv data to json, without any description" < contacts.csv

# With file -f/--file
fgpt -f contacts.csv  "Convert the follow csv data to json, without any description"

# REPL mode
fgpt
>> Write a javascript code to reverse a string
...
```

### With http proxy

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

Offering free self-hosted API access to ChatGPT. This is useful if you want to use the OpenAI API without needing to sign up for an API key.

### 1. Start the server

```bash
fgpt -s 127.0.0.1:4090
```

Your local server will now be running and accessible at: `http://127.0.0.1:4090/v1/chat/completions`

### 2. Example Usage with OpenAI Libraries

```python
import openai
import sys
openai.api_key = 'nothing'
openai.base_url = "http://127.0.0.1:4090/v1/"

completion = openai.chat.completions.create(
    model="gpt-3.5-turbo",
    messages=[
        {"role": "user", "content": "Write a javascript simple code"},
    ],
    stream=True,
)

for chunk in completion:
    print(chunk.choices[0].delta.content, end='')
    sys.stdout.flush()

print()
```

or test with curl:

```bash
curl -X POST -H "Content-Type: application/json" -d '{"model":"gpt-3.5-turbo",
"messages":[{"role":"user","content":"Write a javascript simple code"}], 
"stream":true}' \
http://127.0.0.1:4090/v1/chat/completions
 ```

```bash
curl -X POST -H "Content-Type: application/json" -d '{"model":"gpt-3.5-turbo",
"messages":[{"role":"user","content":"Write a javascript simple code"}]}' \
http://127.0.0.1:4090/v1/chat/completions
 ```
