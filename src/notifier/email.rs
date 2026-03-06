use crate::config::EmailConfig;
use anyhow::{Context, Result};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

pub async fn send(config: &EmailConfig, subject: &str, body: &str) -> Result<()> {
    let password = std::env::var(&config.password_env).map_err(|_| {
        anyhow::anyhow!(
            "Email password env var '{}' is not set",
            config.password_env
        )
    })?;

    let creds = Credentials::new(config.username.clone(), password);

    // Use STARTTLS (port 587). Switch to ::relay() for implicit TLS (port 465).
    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
            .context("Invalid SMTP host")?
            .port(config.smtp_port)
            .credentials(creds)
            .build();

    let from: Mailbox = config
        .from
        .parse()
        .context("Invalid 'from' email address")?;

    for recipient in &config.to {
        let to: Mailbox = recipient
            .parse()
            .with_context(|| format!("Invalid 'to' address: {}", recipient))?;

        let email = Message::builder()
            .from(from.clone())
            .to(to)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .context("Failed to build email message")?;

        mailer
            .send(email)
            .await
            .with_context(|| format!("Failed to send email to {}", recipient))?;
    }

    Ok(())
}
