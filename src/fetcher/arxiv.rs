use crate::models::Paper;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;

const ATOM_NS: &str = "http://www.w3.org/2005/Atom";
const ARXIV_API: &str = "https://export.arxiv.org/api/query";

pub struct ArxivFetcher {
    client: Client,
}

impl ArxivFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch(
        &self,
        categories: &[String],
        max_results: usize,
        days_back: u32,
    ) -> Result<Vec<Paper>> {
        let search_query = categories
            .iter()
            .map(|c| format!("cat:{}", c))
            .collect::<Vec<_>>()
            .join("+OR+");

        let url = format!(
            "{}?search_query={}&start=0&max_results={}&sortBy=submittedDate&sortOrder=descending",
            ARXIV_API, search_query, max_results
        );

        let xml = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to reach arXiv API")?
            .text()
            .await
            .context("Failed to read arXiv response")?;

        parse_atom_feed(&xml, days_back)
    }
}

fn extract_arxiv_id(url: &str) -> String {
    let id = url.split('/').last().unwrap_or(url);
    // Strip version suffix, e.g. "2301.00001v2" -> "2301.00001"
    if let Some(v_pos) = id.rfind('v') {
        let (base, ver) = id.split_at(v_pos);
        if ver[1..].chars().all(|c| c.is_ascii_digit()) && !ver[1..].is_empty() {
            return base.to_string();
        }
    }
    id.to_string()
}

fn parse_atom_feed(xml: &str, days_back: u32) -> Result<Vec<Paper>> {
    let doc = roxmltree::Document::parse(xml).context("Failed to parse arXiv Atom XML")?;
    let cutoff = Utc::now() - chrono::TimeDelta::days(days_back as i64);

    let mut papers = Vec::new();

    for entry in doc
        .descendants()
        .filter(|n| n.has_tag_name((ATOM_NS, "entry")))
    {
        let raw_id = entry
            .descendants()
            .find(|n| n.has_tag_name((ATOM_NS, "id")))
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim()
            .to_string();

        let id = extract_arxiv_id(&raw_id);

        let title = entry
            .descendants()
            .find(|n| n.has_tag_name((ATOM_NS, "title")))
            .and_then(|n| n.text())
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        let abstract_text = entry
            .descendants()
            .find(|n| n.has_tag_name((ATOM_NS, "summary")))
            .and_then(|n| n.text())
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        let published_str = entry
            .descendants()
            .find(|n| n.has_tag_name((ATOM_NS, "published")))
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim()
            .to_string();

        let published: DateTime<Utc> = DateTime::parse_from_rfc3339(&published_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        if published < cutoff {
            continue;
        }

        let mut authors = Vec::new();
        for author_node in entry
            .descendants()
            .filter(|n| n.has_tag_name((ATOM_NS, "author")))
        {
            if let Some(name) = author_node
                .descendants()
                .find(|n| n.has_tag_name((ATOM_NS, "name")))
                .and_then(|n| n.text())
            {
                authors.push(name.trim().to_string());
            }
        }

        let url = entry
            .descendants()
            .find(|n| {
                n.has_tag_name((ATOM_NS, "link"))
                    && n.attribute("rel") == Some("alternate")
            })
            .and_then(|n| n.attribute("href"))
            .unwrap_or(&raw_id)
            .to_string();

        let mut categories = Vec::new();
        for cat in entry
            .descendants()
            .filter(|n| n.tag_name().name() == "category")
        {
            if let Some(term) = cat.attribute("term") {
                categories.push(term.to_string());
            }
        }

        if !id.is_empty() && !title.is_empty() {
            papers.push(Paper {
                id,
                title,
                authors,
                abstract_text,
                published,
                url,
                categories,
            });
        }
    }

    Ok(papers)
}
