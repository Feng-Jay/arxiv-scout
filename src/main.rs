mod analyzer;
mod config;
mod fetcher;
mod filter;
mod llm;
mod models;
mod notifier;
mod storage;

use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "paper-scout", about = "Daily arXiv paper digest powered by LLM")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the full pipeline once (fetch → filter → analyse → report → output)
    Run,
    /// Run as a daemon; re-executes the pipeline daily at the scheduled UTC time
    Daemon,
    Debug,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("paper_scout=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();
    let cfg = config::Config::load(&cli.config)?;

    match cli.command {
        Commands::Run => run_once(&cfg).await?,
        Commands::Daemon => run_daemon(&cfg).await?,
        Commands::Debug => run_debug_send(&cfg).await?,
    }

    Ok(())
}

async fn run_once(cfg: &config::Config) -> Result<()> {
    let max_attempts = cfg.retry.max_attempts;

    // ── Build the three specialised providers ─────────────────────────────────
    let filter_provider = llm::create_provider(&cfg.llm.filter)?;
    let analysis_provider = llm::create_provider(&cfg.llm.analysis)?;

    // ── 1. Fetch ──────────────────────────────────────────────────────────────
    info!(
        "Fetching from arXiv (categories: {}, last {} day(s)) ...",
        cfg.sources.arxiv.categories.join(", "),
        cfg.sources.arxiv.days_back,
    );
    let fetcher = fetcher::arxiv::ArxivFetcher::new();
    let all_papers = fetcher
        .fetch(
            &cfg.sources.arxiv.categories,
            cfg.sources.arxiv.max_results,
            cfg.sources.arxiv.days_back,
            max_attempts,
        )
        .await?;
    let total_fetched = all_papers.len();
    info!("Fetched {} papers", total_fetched);

    // ── 2. Dedup ──────────────────────────────────────────────────────────────
    let storage_path = std::path::Path::new(&cfg.output.output_dir).join("seen_papers.json");
    let mut store = storage::Storage::load(&storage_path)?;

    let new_papers: Vec<_> = all_papers
        .into_iter()
        .filter(|p| !store.is_seen(&p.id))
        .collect();
    let total_new = new_papers.len();
    info!("{} new papers after deduplication", total_new);

    if new_papers.is_empty() {
        info!("Nothing new to process today.");
        return Ok(());
    }

    // ── 3. Filter (filter model — cheap, batch) ───────────────────────────────
    info!("Filtering papers by relevance (filter model) ...");
    let filtered =
        filter::filter_papers(&new_papers, filter_provider.as_ref(), &cfg.interests, max_attempts).await?;
    info!("{} papers scored", filtered.len());

    // ── 4. Analysis (analysis model — per paper) ──────────────────────────────
    let deep = cfg.llm.deep;
    if deep {
        info!(
            "Deep mode enabled — will fetch arXiv HTML, up to {} chars",
            cfg.llm.pdf_chars
        );
    }
    info!("Running analysis (analysis model) ...");
    let analyzed = analyzer::analyze_papers(
        &filtered,
        analysis_provider.as_ref(),
        &cfg.interests,
        cfg.interests.relevance_threshold,
        deep,
        cfg.llm.pdf_chars,
        max_attempts,
    )
    .await?;
    info!("{} papers analysed", analyzed.len());

    // ── 5. Mark seen & save storage ───────────────────────────────────────────
    for p in &new_papers {
        store.mark_seen(&p.id);
    }
    store.cleanup_old(60);
    store.save()?;

    // ── 6. Generate markdown digest ───────────────────────────────────────────
    let markdown = notifier::markdown::generate(&analyzed, total_fetched, total_new);
    let digest_path = notifier::markdown::save(&cfg.output.output_dir, &markdown)?;
    info!("Digest saved to {}", digest_path.display());

    // ── 7. Optional email ─────────────────────────────────────────────────────
    if let Some(email_cfg) = &cfg.email {
        let subject = format!("Paper Scout Digest — {}", Utc::now().format("%Y-%m-%d"));
        match notifier::email::send(email_cfg, &subject, &markdown).await {
            Ok(_) => info!("Email sent to {}", email_cfg.to.join(", ")),
            Err(e) => warn!("Email delivery failed: {:#}", e),
        }
    }

    Ok(())
}

async fn run_debug_send(cfg: &config::Config) -> Result<()> {
    let markdown = std::fs::read_to_string("./digests/2026-03-07.md")?;
    if let Some(email_cfg) = &cfg.email {
        let subject = format!("Paper Scout Digest (Debug) — {}", Utc::now().format("%Y-%m-%d"));
        match notifier::email::send(email_cfg, &subject, &markdown).await {
            Ok(_) => info!("Debug email sent to {}", email_cfg.to.join(", ")),
            Err(e) => warn!("Debug email delivery failed: {:#}", e),
        }
    } else {
        info!("No email config found; skipping debug email send.");
    }
    Ok(())
}

async fn run_daemon(cfg: &config::Config) -> Result<()> {
    info!(
        "Daemon mode started. Daily digest at {} UTC.",
        cfg.schedule.time
    );

    loop {
        let now = Utc::now();
        let (hour, min) = parse_hhmm(&cfg.schedule.time)?;

        let today_target = now
            .date_naive()
            .and_hms_opt(hour, min, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid schedule time"))?
            .and_utc();

        let next_run = if today_target > now {
            today_target
        } else {
            today_target + chrono::TimeDelta::days(1)
        };

        let secs = (next_run - now).num_seconds().max(0) as u64;
        info!(
            "Next run in {}s (at {} UTC)",
            secs,
            next_run.format("%Y-%m-%d %H:%M")
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(secs)).await;

        if let Err(e) = run_once(cfg).await {
            warn!("Pipeline error: {e}");
        }
    }
}

fn parse_hhmm(s: &str) -> Result<(u32, u32)> {
    let (h, m) = s
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("Schedule time must be HH:MM, got '{}'", s))?;
    Ok((h.parse()?, m.parse()?))
}
