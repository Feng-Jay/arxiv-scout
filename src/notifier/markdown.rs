use crate::models::AnalyzedPaper;
use anyhow::Result;
use chrono::Utc;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

pub fn generate(
    papers: &[AnalyzedPaper],
    total_fetched: usize,
    total_new: usize,
) -> String {
    let mut out = String::new();
    let date = Utc::now().format("%Y-%m-%d");

    writeln!(out, "# 🔭 arxiv-scout Digest — {date}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## 📊 Overview").unwrap();
    writeln!(out, "| | |").unwrap();
    writeln!(out, "|---|---|").unwrap();
    writeln!(out, "| 📡 Fetched from arXiv | {total_fetched} |").unwrap();
    writeln!(out, "| 🆕 New (not seen before) | {total_new} |").unwrap();
    writeln!(out, "| ✅ Relevant papers | {} |", papers.len()).unwrap();

    let deep_count = papers.iter().filter(|p| p.deep_analyzed).count();
    if deep_count > 0 {
        writeln!(out, "| 🌐 Full text analysed | {deep_count} |").unwrap();
    }
    writeln!(out).unwrap();

    if papers.is_empty() {
        writeln!(out).unwrap();
        writeln!(out, "---").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "## 🌤️ All Quiet Today").unwrap();
        writeln!(out).unwrap();
        writeln!(
            out,
            "No papers matched your interests today — the arxiv was checked, but nothing rose above the relevance threshold. 📭"
        )
        .unwrap();
        writeln!(out).unwrap();
        writeln!(out, "Take a break, grab a coffee ☕, and come back tomorrow — great papers are always just around the corner. 🚀").unwrap();
        return out;
    }

    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();

    let mut sorted: Vec<&AnalyzedPaper> = papers.iter().collect();
    sorted.sort_by(|a, b| {
        b.relevance_score
            .partial_cmp(&a.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for p in sorted {
        let score_pct = (p.relevance_score * 100.0).round() as u32;
        let deep_badge = if p.deep_analyzed { "  `🌐 Full Text`" } else { "" };

        writeln!(
            out,
            "## 📄 [{}]({})  `{score_pct}% relevant`{deep_badge}",
            p.paper.title, p.paper.url
        )
        .unwrap();
        writeln!(out).unwrap();

        writeln!(out, "👤 **Authors:** {}", p.paper.authors.join(", ")).unwrap();
        writeln!(
            out,
            "📅 **Published:** {}  🏷️ **Categories:** {}",
            p.paper.published.format("%Y-%m-%d"),
            p.paper.categories.join(", ")
        )
        .unwrap();
        writeln!(out).unwrap();

        writeln!(out, "### 💡 TL;DR").unwrap();
        writeln!(out, "{}", p.summary).unwrap();
        writeln!(out).unwrap();

        writeln!(out, "### 🏆 Key Contributions").unwrap();
        for c in &p.key_contributions {
            writeln!(out, "- {c}").unwrap();
        }
        writeln!(out).unwrap();

        writeln!(out, "### ⚙️ Methodology").unwrap();
        writeln!(out, "{}", p.methodology).unwrap();
        writeln!(out).unwrap();

        writeln!(out, "### 🧪 Experiments").unwrap();
        writeln!(out, "{}", p.experiments).unwrap();
        writeln!(out).unwrap();

        writeln!(out, "### 🌱 Insights & Implications").unwrap();
        writeln!(out, "{}", p.insights).unwrap();
        writeln!(out).unwrap();

        writeln!(out, "### 🎯 Why This Paper").unwrap();
        writeln!(out, "{}", p.selection_reason).unwrap();
        writeln!(out).unwrap();

        writeln!(out, "---").unwrap();
        writeln!(out).unwrap();
    }

    out
}

pub fn generate_empty(total_fetched: usize) -> String {
    let mut out = String::new();
    let date = Utc::now().format("%Y-%m-%d");

    writeln!(out, "# 🔭 arxiv-scout Digest — {date}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## 📊 Overview").unwrap();
    writeln!(out, "| | |").unwrap();
    writeln!(out, "|---|---|").unwrap();
    writeln!(out, "| 📡 Fetched from arXiv | {total_fetched} |").unwrap();
    writeln!(out, "| 🆕 New (not seen before) | 0 |").unwrap();
    writeln!(out, "| ✅ Relevant papers | 0 |").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "---").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## 😴 Nothing New Today").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "All {} fetched papers have already been seen in previous runs. 📬",
        total_fetched
    )
    .unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "No worries — rest up and check back tomorrow. Great research is always brewing. ☕🚀"
    )
    .unwrap();

    out
}

pub fn save<P: AsRef<Path>>(output_dir: P, content: &str) -> Result<PathBuf> {
    let dir = output_dir.as_ref();
    std::fs::create_dir_all(dir)?;
    let filename = format!("{}.md", Utc::now().format("%Y-%m-%d"));
    let path = dir.join(filename);
    std::fs::write(&path, content)?;
    Ok(path)
}
