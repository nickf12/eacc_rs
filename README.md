# EACC_rs

![Rust](https://img.shields.io/badge/Rust-1.86-orange?logo=rust)
![Docker](https://img.shields.io/badge/Docker-enabled-blue?logo=docker)
![License](https://img.shields.io/badge/license-MIT-blue)

**EACC_rs** is a Rust application that monitors job posting events on the Arbitrum blockchain and sends real-time notifications to the `@EACC_New_Jobs` Telegram channel. Built with `alloy-rs` for Ethereum interaction, `reqwest` for Telegram API integration, and `tokio` for asynchronous processing, it provides a reliable way to stay updated on new job opportunities in the EACC ecosystem.

## Features
- **Real-Time Monitoring**: Listens for job events on Arbitrum via Infura WebSocket.
- **Telegram Notifications**: Sends formatted job details (title, description, CID, amount in $AIUS) to `@EACC_New_Jobs`.
- **Dockerized Deployment**: Packaged as a lightweight Docker container for consistent deployment.
- **Robust Testing**: Integration tests for IPFS data fetching and notification logic.

## Prerequisites
- [Rust 1.80+](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/) (for containerized deployment)
- A Telegram bot token from [@BotFather](https://t.me/BotFather)
- An [Infura](https://infura.io/) account with an Arbitrum WebSocket URL
- A GitHub account and [Docker Hub](https://hub.docker.com/) account (for CI/CD)

## Installation

### Clone the Repository
```bash
git clone https://github.com/yourusername/eacc_rs.git
cd eacc_rs
```

### Set up environment variables
```bash
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_CHAT_ID=@EACC_New_Jobs
INFURA_WS_API=your_infura_api
```

### Install dependencies
```bash
cargo build
```

### Running
- Locally
```bash
cargo run
```
- Docker
```bash
docker build -t eacc_rs:latest .
docker run -p 3000:3000 --env-file .env eacc_rs:latest
```

### Testing
```bash
cargo test
```
### Testing app health
```bash
curl http://0.0.0.0:3000/health
```

MIT License

Copyright (c) 2025 Pecu

Permission is hereby granted, free of charge, to any person obtaining a copy...