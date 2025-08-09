# EACC_rs

![Rust](https://img.shields.io/badge/Rust-1.86-orange?logo=rust)
![Docker](https://img.shields.io/badge/Docker-enabled-blue?logo=docker)
![License](https://img.shields.io/badge/license-MIT-blue)

**EACC_rs** is a Rust application that monitors job posting events on the Arbitrum blockchain and sends real-time notifications to the `@EACC_New_Jobs` Telegram channel and `@EaccJobs` X profile. 
Built with `alloy-rs` for Ethereum interaction, `reqwest` for Telegram/X API integration, and `tokio` for asynchronous processing, it provides a reliable way to stay updated on new job opportunities in the EACC ecosystem.

## Features
- **Real-Time Monitoring**: Listens for job events on Arbitrum via Infura WebSocket.
- **Telegram Notifications**: Sends formatted job details (title, description, amount) to `@EACC_New_Jobs`.
- **X Notifications**: Sends formatted job details (title, description, amount ) to `@EaccJobs`.
- **Dockerized Deployment**: Packaged as a lightweight Docker container for consistent deployment.
- **Robust Testing**: Integration tests for IPFS data fetching and notification logic.

## Prerequisites
- [Rust 1.80+](https://www.rust-lang.org/tools/install)
- [Docker](https://docs.docker.com/get-docker/) (for containerized deployment)
- A [Telegram] bot token from [@BotFather](https://t.me/BotFather)
- A [Twitter] developer account for the required API keys
- An [Infura](https://infura.io/) account with an Arbitrum WebSocket URL

## Installation

### Clone the Repository
```bash
git clone https://github.com/nickf12/eacc_rs.git
cd eacc_rs
```

### Set up environment variables
Copy and paste the .env.template file, rename it .env and add the needed data for the environment variables
```bash
cp .env.template .env
```

### Install dependencies
```bash
cargo build
```

### Running
- Locally
```bash
cargo run --bin eacc_rs
```
- Docker
```bash
docker build -t eacc_rs:latest .
```
- Docker-compose
```bash
docker-compose up
```

### Testing
```bash
cargo test
```

MIT License
Copyright (c) 2025 Pecu
Permission is hereby granted, free of charge, to any person obtaining a copy...