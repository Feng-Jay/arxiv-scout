# 🔭 arxiv-scout

**Your personal AI-powered arXiv research assistant.** Fetches the latest papers, scores their relevance to your interests, and delivers a structured daily digest — optionally enriched with full-paper analysis.

---

## ✨ Features

- 📡 **Auto-fetch** papers from any arXiv category combination
- 🧠 **LLM-powered relevance filtering** — batch-scores abstracts with a cheap model
- 🔬 **Deep per-paper analysis** — summary, contributions, methodology, experiments, insights
- 🌐 **Deep mode** — fetches full paper text from arXiv HTML for richer analysis
- 📬 **Email delivery** — optional SMTP digest delivery
- 🕐 **Daemon mode** — runs automatically every day at a scheduled UTC time
- 🔌 **Multi-provider** — each model slot has its own provider & API key (Anthropic / OpenAI / custom)

---

## 🔄 Pipeline

```
arXiv API ──► Dedup ──► Filter ──► Analyse ──► Markdown digest
                        (cheap)   (powerful)   [+ optional email]
```

| Step | What happens |
|------|-------------|
| 1️⃣ **Fetch** | Pull recent papers from configured arXiv categories |
| 2️⃣ **Dedup** | Skip papers already seen in previous runs |
| 3️⃣ **Filter** | Batch-score abstracts for relevance with a cheap, fast model |
| 4️⃣ **Analyse** | Per-paper structured analysis for papers above the relevance threshold |
| 5️⃣ **Output** | Save a Markdown digest; optionally deliver via email |

Each analysed paper gets:

- 💡 TL;DR summary
- 🏆 Key contributions
- ⚙️ Methodology / study design
- 🧪 Experimental setup & results
- 🌱 Insights & implications
- 🎯 Why this paper was selected

---

## 🚀 Getting Started

There are three ways to use arxiv-scout. Pick the one that fits you best:

---

### 🅰️ Option 1 — Pre-built Binary (no Rust required)

Download the latest binary for your platform from the [Releases page](https://github.com/Feng-Jay/arxiv-scout/releases/latest):

```bash
# macOS Apple Silicon
curl -L https://github.com/Feng-Jay/arxiv-scout/releases/latest/download/paper-scout-macos-aarch64 -o paper-scout
chmod +x paper-scout

# macOS Intel
curl -L https://github.com/Feng-Jay/arxiv-scout/releases/latest/download/paper-scout-macos-x86_64 -o paper-scout
chmod +x paper-scout

# Linux x86_64
curl -L https://github.com/Feng-Jay/arxiv-scout/releases/latest/download/paper-scout-linux-x86_64 -o paper-scout
chmod +x paper-scout
```

**Windows:** download `paper-scout-windows-x86_64.exe` from the Releases page and run it directly.

Then configure and run:

```bash
cp config.example.toml config.toml
# edit config.toml with your API keys and interests
./paper-scout run
```

> [!NOTE]
> Pre-built Linux binaries are statically linked (musl), so they run on any Linux distribution with no extra dependencies.

---

### 🅱️ Option 2 — GitHub Actions (zero local setup, runs in the cloud ☁️)

Fork the repo and let GitHub run the digest for you every day automatically — no local machine needed.

**Steps:**

1. **Fork** this repository on GitHub
2. Go to your fork → **Settings → Secrets and variables → Actions → New repository secret**
3. Add two secrets:

   | Secret name | Value |
   |---|---|
   | `CONFIG_TOML` | The full contents of your `config.toml` |
   | `EMAIL_PASSWORD` | Your SMTP app password (if using email delivery) |

4. Go to **Actions → Daily Digest → Run workflow** to trigger a manual test run
5. From then on it runs automatically every day at **08:00 Beijing Time** (00:00 UTC)

Generated digests are committed back to your fork under `digests/YYYY-MM-DD.md`.

> [!TIP]
> You can adjust the schedule by editing `.github/workflows/daily.yml` and changing the cron expression. Use [crontab.guru](https://crontab.guru) to find your preferred time.

> [!WARNING]
> Keep your `CONFIG_TOML` secret up to date whenever you change your interests or API keys — the workflow reads it fresh on every run.

---

### 🅲 Option 3 — Build from Source (requires Rust)

```bash
git clone https://github.com/Feng-Jay/arxiv-scout
cd arxiv-scout
cargo build --release
cp config.example.toml config.toml
# edit config.toml
./target/release/paper-scout run
```

Requires [Rust](https://rustup.rs/) stable toolchain.

---

## ⚙️ Configuration

```bash
cp config.example.toml config.toml
```

### Minimal example

```toml
[interests]
topics   = ["automated program repair", "vulnerability detection"]
keywords = ["LLM", "program analysis", "static analysis"]
relevance_threshold = 0.6   # papers below this are dropped

[sources.arxiv]
categories  = ["cs.SE", "cs.CR", "cs.PL"]
max_results = 100
days_back   = 1

[llm]
deep      = false   # set true to fetch full paper text
# pdf_chars = 20000

[llm.filter]        # cheap & fast — batch relevance scoring
provider   = "openai"
api_key    = "sk-..."
model      = "gpt-4o-mini"
max_tokens = 8192

[llm.analysis]      # powerful — per-paper deep analysis
provider   = "anthropic"
api_key    = "sk-ant-..."
model      = "claude-sonnet-4-6"
max_tokens = 8192

[output]
output_dir = "./digests"
```

---

### 🔌 LLM Providers

Each model slot (`filter`, `analysis`) is **independently configured** with its own provider and API key, so you can mix and match freely.

| `provider` | Description |
|---|---|
| `"anthropic"` | Anthropic API — Claude models |
| `"openai"` | OpenAI API — GPT models |
| `"custom"` | Any OpenAI-compatible endpoint; set `base_url` |

```toml
# Example: use a local Ollama model for cheap filtering
[llm.filter]
provider = "custom"
base_url = "http://localhost:11434/v1"
api_key  = "none"
model    = "qwen2.5:14b"
max_tokens = 4096
```

> [!TIP]
> Using a small local model for the filter step and a cloud model for analysis is a cost-effective setup — the filter step processes many papers at once, while the analysis step only runs on the few that pass the threshold.

---

### 🌐 Deep Mode

When `deep = true`, the analyser fetches the full paper text from `https://arxiv.org/html/{id}` and includes the first `pdf_chars` characters in the analysis prompt.

```toml
[llm]
deep      = true
pdf_chars = 15000
```

> [!NOTE]
> arXiv HTML is available for most recent papers submitted in LaTeX. If a paper's HTML page is unavailable, arxiv-scout logs a warning and falls back to abstract-only analysis for that paper.

> [!WARNING]
> Deep mode significantly increases API token usage. Consider using a higher `relevance_threshold` (e.g. `0.75`) alongside `deep = true` to limit the number of papers that go through full analysis.

---

## 🖥️ Usage

**Run once:**
```bash
./target/release/paper-scout run

# Custom config file
./target/release/paper-scout -c my-config.toml run
```

**Run as a daemon** (re-runs daily at the scheduled UTC time):
```bash
./target/release/paper-scout daemon
```

**Schedule:**
```toml
[schedule]
time = "08:00"   # UTC
```

---

## 📄 Output

Digests are saved as `./digests/YYYY-MM-DD.md`. Example:

```markdown
# 🔭 arxiv-scout Digest — 2026-03-06

## Overview
| | |
|---|---|
| Fetched from arXiv | 87 |
| New (not seen before) | 52 |
| Relevant papers | 6 |
| Full paper analysed | 4 |

---

## [Paper Title](https://arxiv.org/abs/...) `84% relevant`  `📄 PDF`

**Authors:** Alice, Bob
**Published:** 2026-03-05  **Categories:** cs.SE, cs.CR

### TL;DR
...

### Key Contributions
- ...

### Methodology
...

### Experiments
...

### Insights & Implications
...

### Why This Paper
...
```

---

## 📬 Email Delivery (optional)

```toml
[email]
smtp_host    = "smtp.gmail.com"
smtp_port    = 587
username     = "you@gmail.com"
password_env = "EMAIL_PASSWORD"   # set via: export EMAIL_PASSWORD=<app-password>
from         = "you@gmail.com"
to           = ["you@gmail.com", "colleague@university.edu"]
```

> [!NOTE]
> For Gmail, generate an [App Password](https://support.google.com/accounts/answer/185833) rather than using your account password.

---

## 🗂️ Project Structure

```
src/
├── main.rs               # CLI, pipeline orchestration
├── config.rs             # TOML config structs
├── models.rs             # Paper / AnalyzedPaper types
├── fetcher/
│   ├── arxiv.rs          # arXiv Atom API client
│   └── pdf.rs            # arXiv HTML fetcher & text stripper
├── filter.rs             # Batch relevance scoring
├── analyzer.rs           # Per-paper deep analysis
├── llm/
│   ├── mod.rs            # LlmProvider trait & provider factory
│   ├── anthropic.rs      # Anthropic Messages API
│   └── openai_compat.rs  # OpenAI-compatible chat API
├── notifier/
│   ├── markdown.rs       # Markdown digest generator
│   └── email.rs          # SMTP delivery via lettre
└── storage.rs            # Seen-paper deduplication
```

---

## 📜 License

MIT
