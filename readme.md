# ZeroFA
Reads your email for 2FA codes and serves them on a web page.

## What it does

- Watches your email for "Your [service] code is [123456]" messages
- Serves the latest code at `http://localhost:8080`

## Setup

Make a `.env` file:
```env
IMAP_SERVER=imap.gmail.com
IMAP_PORT=993
IMAP_USERNAME=your-email@example.com
IMAP_PASSWORD=your-app-password
PORT=8080
```

Run it:
```bash
nix develop
cargo run
```

Visit `http://localhost:8080` to see your codes.

## Deploy

The static build is currently busted fyi, just do `nix develop` then `cargo build`
If the static build worked, you'd technically get a binary you can just copy over.
But not for now.
```bash
nix build .#static
```


