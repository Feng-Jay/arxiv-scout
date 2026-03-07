use crate::config::EmailConfig;
use anyhow::{Context, Result};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use pulldown_cmark::{html, Options, Parser};

pub async fn send(config: &EmailConfig, subject: &str, markdown: &str) -> Result<()> {
    let password = std::env::var(&config.password_env).map_err(|_| {
        anyhow::anyhow!(
            "Email password env var '{}' is not set",
            config.password_env
        )
    })?;

    let creds = Credentials::new(config.username.clone(), password);

    let mailer: AsyncSmtpTransport<Tokio1Executor> = match config.tls_mode.as_str() {
        "tls" => {
            // Implicit TLS — typically port 465
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_host)
                .context("Invalid SMTP host")?
                .port(config.smtp_port)
                .credentials(creds)
                .build()
        }
        "none" => {
            // Plain SMTP, no TLS — typical for internal/campus relay
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.smtp_host)
                .port(config.smtp_port)
                .credentials(creds)
                .build()
        }
        _ => {
            // "starttls" (default) — STARTTLS upgrade, typically port 587
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
                .context("Invalid SMTP host")?
                .port(config.smtp_port)
                .credentials(creds)
                .build()
        }
    };

    let html_body = markdown_to_html(markdown);

    let from: Mailbox = config
        .from
        .parse()
        .context("Invalid 'from' email address")?;

    let content_type: ContentType = "text/html; charset=utf-8"
        .parse()
        .context("Failed to parse content type")?;

    for recipient in &config.to {
        let to: Mailbox = recipient
            .parse()
            .with_context(|| format!("Invalid 'to' address: {}", recipient))?;

        let email = Message::builder()
            .from(from.clone())
            .to(to)
            .subject(subject)
            .header(content_type.clone())
            .body(html_body.clone())
            .context("Failed to build email message")?;

        mailer.send(email).await.with_context(|| {
            format!(
                "SMTP delivery failed (host={}:{}, tls={}, from={}, to={})",
                config.smtp_host, config.smtp_port, config.tls_mode, config.from, recipient
            )
        })?;
    }

    Ok(())
}

/// Convert markdown to a self-contained HTML email with inline styles.
fn markdown_to_html(md: &str) -> String {
    let mut html_content = String::new();
    let parser = Parser::new_ext(md, Options::all());
    html::push_html(&mut html_content, parser);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
    font-size: 15px;
    line-height: 1.6;
    color: #24292e;
    max-width: 800px;
    margin: 0 auto;
    padding: 24px;
    background: #ffffff;
  }}
  h1 {{ font-size: 2em; border-bottom: 2px solid #eaecef; padding-bottom: 0.3em; margin-top: 0; }}
  h2 {{ font-size: 1.4em; border-bottom: 1px solid #eaecef; padding-bottom: 0.2em; margin-top: 2em; }}
  h3 {{ font-size: 1.1em; color: #444; margin-top: 1.4em; }}
  hr {{ border: none; border-top: 1px solid #eaecef; margin: 2em 0; }}
  code {{ background: #f6f8fa; padding: 2px 5px; border-radius: 3px; font-size: 0.9em; }}
  pre  {{ background: #f6f8fa; padding: 12px; border-radius: 6px; overflow-x: auto; }}
  blockquote {{ border-left: 4px solid #dfe2e5; margin: 0; padding-left: 1em; color: #6a737d; }}
  table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
  th, td {{ border: 1px solid #dfe2e5; padding: 6px 13px; text-align: left; }}
  th {{ background: #f6f8fa; font-weight: 600; }}
  tr:nth-child(even) {{ background: #f6f8fa; }}
  ul, ol {{ padding-left: 1.5em; }}
  a {{ color: #0366d6; text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  strong {{ font-weight: 600; }}
</style>
</head>
<body>
{html_content}
</body>
</html>"#
    )
}
