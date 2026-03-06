use crate::config::InterestsConfig;
use crate::llm::LlmProvider;
use crate::models::{AnalyzedPaper, Paper};
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

#[derive(Deserialize)]
struct AnalysisResult {
    summary: String,
    key_contributions: Vec<String>,
    methodology: String,
    experiments: String,
    insights: String,
    selection_reason: String,
}

pub async fn analyze_papers(
    filtered: &[(Paper, f32, String)],
    provider: &dyn LlmProvider,
    interests: &InterestsConfig,
    threshold: f32,
    deep: bool,
    pdf_chars: usize,
) -> Result<Vec<AnalyzedPaper>> {
    let relevant: Vec<_> = filtered
        .iter()
        .filter(|(_, score, _)| *score >= threshold)
        .collect();

    info!(
        "Analysing {}/{} papers (threshold ≥ {:.0}%, deep={}) ...",
        relevant.len(),
        filtered.len(),
        threshold * 100.0,
        deep,
    );

    let client = Client::new();
    let mut analyzed = Vec::new();

    for (paper, score, filter_reason) in &relevant {
        info!("  Analysing: {}", paper.title);

        // Optionally fetch full paper text via arXiv HTML
        let paper_text = if deep {
            crate::fetcher::pdf::fetch_paper_text(&client, &paper.id, pdf_chars).await
        } else {
            None
        };

        let deep_analyzed = paper_text.is_some();
        let system = analysis_system(interests, deep_analyzed);
        let user = analysis_user(paper, paper_text.as_deref());

        match provider.complete(&system, &user).await {
            Ok(resp) => match parse_analysis(&resp) {
                Ok(r) => analyzed.push(AnalyzedPaper {
                    paper: (*paper).clone(),
                    relevance_score: *score,
                    relevance_reason: filter_reason.clone(),
                    summary: r.summary,
                    key_contributions: r.key_contributions,
                    methodology: r.methodology,
                    experiments: r.experiments,
                    insights: r.insights,
                    selection_reason: r.selection_reason,
                    deep_analyzed,
                }),
                Err(e) => warn!("Analysis parse error for '{}': {}", paper.id, e),
            },
            Err(e) => warn!("Analysis LLM call failed for '{}': {}", paper.id, e),
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    Ok(analyzed)
}

// ── Prompt builders ───────────────────────────────────────────────────────────

fn analysis_system(interests: &InterestsConfig, has_full_text: bool) -> String {
    let source_note = if has_full_text {
        "The full paper text (fetched from arXiv HTML) is provided after the abstract."
    } else {
        "Only the abstract is available; infer as much as possible from it."
    };

    format!(
        r#"You are an expert academic paper analyst writing for a researcher whose interests are: {topics}

{source_note}

Return ONLY valid JSON with NO prose and NO markdown fences:
{{
  "summary": "2-3 sentence comprehensive summary of the paper",
  "key_contributions": ["contribution 1", "contribution 2", "..."],
  "methodology": "Detailed description of the proposed method, model architecture, or study design",
  "experiments": "Experimental setup: datasets, baselines, metrics, and key quantitative results",
  "insights": "Insights, open questions, limitations, and implications for related research",
  "selection_reason": "Why this paper is relevant to the researcher's specific interests and how it advances their work"
}}"#,
        topics = interests.topics.join(", "),
    )
}

fn analysis_user(paper: &Paper, pdf_text: Option<&str>) -> String {
    let mut s = format!(
        "Title: {}\nAuthors: {}\nAbstract: {}",
        paper.title,
        paper.authors.join(", "),
        paper.abstract_text,
    );
    if let Some(text) = pdf_text {
        s.push_str("\n\n--- Extracted Paper Text ---\n");
        s.push_str(text);
    }
    s
}

// ── JSON helpers ──────────────────────────────────────────────────────────────

fn parse_analysis(response: &str) -> Result<AnalysisResult> {
    let s = extract_json_object(response);
    serde_json::from_str(s)
        .map_err(|e| anyhow::anyhow!("JSON parse error: {}\nRaw:\n{}", e, response))
}

fn extract_json_object(text: &str) -> &str {
    let text = text.trim();
    if let Some(after) = text.strip_prefix("```json") {
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
    }
    if let Some(after) = text.strip_prefix("```") {
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
    }
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        return &text[start..=end];
    }
    text
}
