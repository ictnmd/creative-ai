# Creative AI Studio

A multi-user AI image generation platform. Phase 1 focuses on core image generation capabilities, user authentication, and generation queue management.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | SvelteKit, Tailwind CSS, TypeScript |
| Backend API | Rust (Axum), SQLx |
| Database | PostgreSQL 16 |
| Cache / Queue | DragonflyDB |
| Storage | S3 / MinIO |
| Containerization | Docker, Docker Compose |

## Quick Start

### 1. Configure environment

```bash
cp .env.example .env
# Edit .env and fill in your values
```

### 2. Start infrastructure and services

```bash
docker compose up -d
```

This starts all containers: PostgreSQL, DragonflyDB, MinIO, backend API, worker, and frontend.

### 3. Access services

| Service | URL |
|---------|-----|
| Frontend | http://localhost:5173 |
| Backend API | http://localhost:8080 |
| MinIO Console | http://localhost:9001 |

## Development

### Backend

The backend is a Rust workspace with multiple crates. Run the API server and worker from the `backend/` directory:

```bash
cd backend
cargo run --bin app          # Start the API server (port 8080)
cargo run --bin worker      # Start the generation worker
```

### Frontend

```bash
cd frontend
npm install
npm run dev                 # Start dev server (port 5173)
npm run build               # Production build
npm run check               # Type check with svelte-check
```

## Architecture

Detailed architecture documentation lives in `docs/superpowers/specs/`. This includes:

- System design and component interaction
- Database schema and data models
- API endpoints and request/response contracts
- Worker queue and generation pipeline
- Security and authentication design

## Phases

| Phase | Focus | Features |
|-------|-------|----------|
| 1 | Core Generation | User auth, image generation queue, multi-provider AI (OpenAI, Gemini, Anthropic), S3 storage, generation history |
| 2 | Sharing & Collaboration | Share generations via URL, public galleries, comments, likes |
| 3 | Enhanced Generation | Advanced parameters (aspect ratio, style presets, inpainting/outpainting), batch generation |
| 4 | Billing & Subscriptions | Stripe integration, usage-based billing, free tier with rate limits, subscription plans |
| 5 | Custom Models & Fine-tuning | Bring-your-own API keys, custom model fine-tuning, model marketplace |
| 6 | Creative Workflows | Workflow builder, generation pipelines, team workspaces, API access |
