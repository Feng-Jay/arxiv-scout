use crate::config::InterestsConfig;
use crate::llm::LlmProvider;
use crate::models::Paper;
use anyhow::Result;
use serde::Deserialize;
use tracing::{info, warn};

/// Number of papers sent to the LLM in one batch
const BATCH_SIZE: usize = 20;

#[derive(Deserialize)]
struct FilterResult {
    id: String,
    score: f32,
    reason: String,
}

/// Returns `(paper, score 0–1, reason)` for every paper processed.
/// Papers whose LLM call or JSON parse fails are silently skipped with a warning.
pub async fn filter_papers(
    papers: &[Paper],
    provider: &dyn LlmProvider,
    interests: &InterestsConfig,
) -> Result<Vec<(Paper, f32, String)>> {
    let mut results = Vec::new();

    for (i, batch) in papers.chunks(BATCH_SIZE).enumerate() {
        info!(
            "Filtering batch {}/{} ({} papers)",
            i + 1,
            papers.chunks(BATCH_SIZE).count(),
            batch.len()
        );

        let system = build_system(interests);
        let user = build_user(batch);
        info!("Filtering user prompt: {}", user);
        match provider.complete(&system, &user).await {
            Ok(response) => match parse_response(&response, batch) {
                Ok(batch_results) => results.extend(batch_results),
                Err(e) => warn!("Failed to parse filter response for batch {}: {}", i + 1, e),
            },
            Err(e) => warn!("LLM filter request failed for batch {}: {}", i + 1, e),
        }

        // Small delay to avoid hammering rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    Ok(results)
}

fn build_system(interests: &InterestsConfig) -> String {
    format!(
        r#"You are a research paper relevance evaluator.

The researcher is interested in: {topics}
Keywords of interest: {keywords}

Score each paper's relevance from 0 to 10:
- 0-3: Not relevant
- 4-6: Somewhat relevant
- 7-9: Highly relevant
- 10: Core contribution to the researcher's interests

Return ONLY a valid JSON array, no prose, no markdown fences:
```json
[{{"id":"<arxiv_id>","score":<0-10>,"reason":"<1-2 sentences>"}}]
```"#,
        topics = interests.topics.join(", "),
        keywords = interests.keywords.join(", "),
    )
}

fn build_user(papers: &[Paper]) -> String {
    let mut prompt = String::from("Evaluate relevance of these papers:\n\n");
    for p in papers {
        let abstract_preview: String = p.abstract_text.chars().take(600).collect();
        prompt.push_str(&format!(
            "ID: {}\nTitle: {}\nAbstract: {}\n\n---\n\n",
            p.id, p.title, abstract_preview
        ));
    }
    prompt
}

fn parse_response(response: &str, papers: &[Paper]) -> Result<Vec<(Paper, f32, String)>> {
    let json_str = extract_json_array(response);
    let filter_results: Vec<FilterResult> = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("JSON parse error: {}\nRaw response:\n{}", e, response))?;

    let paper_map: std::collections::HashMap<&str, &Paper> =
        papers.iter().map(|p| (p.id.as_str(), p)).collect();

    let results = filter_results
        .into_iter()
        .filter_map(|r| {
            paper_map.get(r.id.as_str()).map(|p| {
                // Normalise score to 0.0–1.0
                ((*p).clone(), (r.score / 10.0).clamp(0.0, 1.0), r.reason)
            })
        })
        .collect();

    Ok(results)
}

/// Strips optional ```json ... ``` fences and returns the innermost JSON array.
fn extract_json_array(text: &str) -> &str {
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
    if let (Some(start), Some(end)) = (text.find('['), text.rfind(']')) {
        return &text[start..=end];
    }
    text
}
