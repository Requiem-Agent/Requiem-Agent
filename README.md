# 🤖 Requiem Agent

> **Autonomous AI Agent Platform** — Rust backend + React frontend + PostgreSQL/Turso + Prometheus/Grafana

[![CI](https://github.com/Requiem-Agent/Requiem-Agent/actions/workflows/ci.yml/badge.svg)](https://github.com/Requiem-Agent/Requiem-Agent/actions)
[![HuggingFace Space](https://img.shields.io/badge/🤗%20HuggingFace-Space-blue)](https://huggingface.co/spaces/rayig/Dev)

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    React Frontend (GitHub Pages)             │
│  Chat UI · Settings · API Keys · Rate Limit Dashboard        │
└──────────────────────┬──────────────────────────────────────┘
                       │ HTTPS / WSS
┌──────────────────────▼──────────────────────────────────────┐
│              Rust Backend (Axum) — HuggingFace Space         │
│  REST API · WebSocket Streaming · ReAct Engine               │
│  Rate Limiting · AES-256-GCM Encryption · JWT Auth           │
└──────┬───────────────┬──────────────────┬───────────────────┘
       │               │                  │
┌──────▼──────┐ ┌──────▼──────┐ ┌────────▼────────┐
│  Turso DB   │ │  Anthropic  │ │   Prometheus +  │
│  (libsql)   │ │  Claude API │ │   Grafana       │
└─────────────┘ └─────────────┘ └─────────────────┘
```

---

## 🚀 Quick Start

### Prerequisites
- Rust 1.75+ (`rustup update stable`)
- Node.js 20+ + pnpm (`npm i -g pnpm`)
- A [Turso](https://turso.tech) database (free tier works)
- Anthropic API key (optional — echo mode works without it)

### 1. Clone & Setup

```bash
git clone https://github.com/Requiem-Agent/Requiem-Agent.git
cd Requiem-Agent
```

### 2. Backend

```bash
cd rust-backend

# Copy and fill environment variables
cp ../.env.example .env
# Edit .env with your TURSO_URL and TURSO_AUTH_TOKEN

# Run in development mode
cargo run

# Server starts on http://localhost:7860
```

### 3. Frontend

```bash
# From repo root
pnpm install
pnpm dev

# Frontend starts on http://localhost:5173
```

### 4. Docker Compose (Full Stack)

```bash
# Copy and fill environment variables
cp .env.example .env

# Start everything: backend + prometheus + grafana
docker compose up -d

# Services:
#   Backend:    http://localhost:7860
#   Prometheus: http://localhost:9090
#   Grafana:    http://localhost:3000 (admin/admin)
```

---

## ⚙️ Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `TURSO_URL` | ✅ | Turso database URL (`libsql://...`) |
| `TURSO_AUTH_TOKEN` | ✅ | Turso auth token |
| `ANTHROPIC_API_KEY` | ⚠️ | Anthropic Claude API key (echo mode if missing) |
| `ENCRYPTION_KEY` | ⚠️ | 64-char hex key for AES-256-GCM (user API key encryption) |
| `JWT_SECRET` | ⚠️ | JWT signing secret |
| `ALLOWED_ORIGINS` | ❌ | Comma-separated CORS origins (default: GitHub Pages + Telegram) |
| `PORT` | ❌ | Server port (default: 7860) |
| `RUST_LOG` | ❌ | Log level (e.g., `requiem_server=debug`) |

Generate `ENCRYPTION_KEY`:
```bash
openssl rand -hex 32
```

---

## 📡 API Endpoints

### Public (no auth)
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/healthz` | Health check |
| `GET` | `/api/metrics` | Prometheus metrics |
| `POST` | `/api/auth/login` | PopCorn account login (username/password) |
| `GET` | `/api/models` | List available LLM models |

### Protected (JWT required)
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/agent/chat` | Send message to ReAct agent |
| `GET` | `/api/ws/agent` | WebSocket streaming (start/cancel/ping) |
| `GET` | `/api/preferences` | Get user preferences |
| `PUT` | `/api/preferences` | Update user preferences |
| `PATCH` | `/api/preferences` | Partial update preferences |
| `GET` | `/api/user-api-keys` | List stored LLM API keys |
| `POST` | `/api/user-api-keys` | Store encrypted LLM API key |
| `DELETE` | `/api/user-api-keys/:id` | Delete API key |
| `GET` | `/api/sessions` | List chat sessions |
| `POST` | `/api/sessions` | Create session |
| `GET` | `/api/sandbox/exec` | Execute code in sandbox |
| `GET` | `/api/rag/search` | Search RAG memory |

---

## 🔌 WebSocket Protocol

Connect to `wss://rayig-dev.hf.space/api/ws/agent` with a JWT token.

```typescript
// Client → Server
{ "type": "start", "message": "Hello!", "mode": "chat" }
{ "type": "cancel" }
{ "type": "ping" }

// Server → Client
{ "type": "token", "content": "Hello" }
{ "type": "step", "step": 1, "thought": "...", "action": "..." }
{ "type": "tool_call", "tool": "web_search", "input": "..." }
{ "type": "tool_result", "tool": "web_search", "output": "..." }
{ "type": "done", "content": "Full response", "steps": 3 }
{ "type": "error", "message": "..." }
{ "type": "pong" }
```

---

## 🗄️ Database Schema

| Migration | Tables |
|-----------|--------|
| `001_initial_schema.sql` | users, sessions, messages |
| `002_rag_memory.sql` | memories (RAG context) |
| `003_rate_limits_and_metrics.sql` | rate_limit_log, metrics_log |
| `004_user_preferences.sql` | user_preferences, user_api_keys, user_shortcuts, user_workspace_settings, user_notification_log |
| `005_conversations.sql` | conversations, conversation_messages, conversation_summaries, conversation_tags |

---

## 🔒 Security

- **AES-256-GCM** encryption for stored LLM API keys (random 96-bit nonce per encryption)
- **Zeroizing** memory cleanup — decrypted keys are wiped from RAM after use
- **Token** authentication with PopCorn accounts (username/password, Argon2id)
- **Per-user rate limiting** — `RateLimitKey::User(user_id)` via sliding window
- **Input validation** on all endpoints
- **Sandbox isolation** — Landlock FS + seccomp-bpf + rlimit for code execution

---

## 📊 Monitoring

- **Prometheus** metrics at `/api/metrics` (8+ metrics: request count, latency, WS connections, LLM calls, etc.)
- **Grafana** dashboard with 14 panels (6 stat + 8 time-series)
- **8 alerting rules**: rate limit hits > 50/min, error rate > 5%, p95 latency > 2s, service down, etc.
- **Slack** notifications for critical alerts

---

## 🧪 Testing

```bash
cd rust-backend

# Unit tests
cargo test

# Integration tests (requires running server)
cargo test --test agent_chat_integration
cargo test --test ws_streaming_e2e
cargo test --test comprehensive_integration
```

---

## 📁 Project Structure

```
Requiem-Agent/
├── rust-backend/           # Axum backend
│   ├── src/
│   │   ├── main.rs         # Server setup, routing
│   │   ├── db.rs           # AppState, Turso connection
│   │   ├── auth.rs         # PopCorn auth (username/password, Argon2id)
│   │   ├── crypto.rs       # AES-256-GCM encryption
│   │   ├── rate_limit.rs   # Per-user/IP rate limiting
│   │   ├── metrics.rs      # Prometheus metrics
│   │   ├── react_loop.rs   # ReAct engine (Reason + Act)
│   │   ├── llm_stream.rs   # Anthropic SSE → WS bridge
│   │   ├── llm_providers.rs # Multi-model support
│   │   ├── plugins.rs      # Tool/plugin system
│   │   ├── agent/          # Agent subsystems
│   │   ├── routes/         # HTTP handlers
│   │   ├── sandbox/        # Code execution sandbox
│   │   └── ...
│   ├── migrations/         # SQL migrations (001-005)
│   └── tests/              # Integration + E2E tests
├── src/                    # React frontend
│   ├── hooks/              # useAgentStream, usePreferences, ...
│   ├── pages/              # Settings, API Keys, Admin, ...
│   └── components/         # UI components (shadcn/ui)
├── monitoring/             # Grafana + Prometheus configs
├── docker-compose.yml      # Full stack Docker setup
└── .github/workflows/      # CI/CD pipelines
```

---

## 🌐 Live Deployments

| Service | URL | Status |
|---------|-----|--------|
| **Frontend** | [requiem-agent.github.io/Requiem-Agent](https://requiem-agent.github.io/Requiem-Agent) | ✅ Live |
| **Backend** | [rayig-dev.hf.space](https://rayig-dev.hf.space) | 🔄 Building |

---

## 📜 License

MIT — see [LICENSE](LICENSE) for details.
