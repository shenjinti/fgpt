# A free reverse proxy and CLI tool for OpenAI GPT-3.5-turbo.
It allows you to use the GPT-3.5 API without needing to sign up for an API key or pay for usage. 
**[WIP]** 

## Installation
```bash
cargo install rgpt
```
## How to use CLI

```bash
# To get help
rgpt "Linux command to list files in a directory"

# Output plain code
rgpt -c "Write python code to reverse a string"

# With pipe
git diff | rgpt "Write a commit message for this diff"

# With stdin
rgpt -r "Convert CSV to JSON" < contacts.csv

# With file
rgpt -f contacts.csv  "Convert CSV to JSON"

# REPL mode
rgpt
>> How to list files in a directory [ALT+ENTER]
...
```

## How to use Reverse Proxy
```bash
rgpt -s 127.0.0.1:3000
```
