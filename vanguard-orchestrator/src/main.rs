use futures::StreamExt;
use vanguard_core::{InterceptorReport, REPORTS_SUBJECT_WILDCARD};

const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| DEFAULT_NATS_URL.to_string());
    let client = async_nats::connect(&nats_url).await?;
    let mut reports = client.subscribe(REPORTS_SUBJECT_WILDCARD).await?;
    println!("orchestrator online — listening on `{REPORTS_SUBJECT_WILDCARD}` via {nats_url}");

    while let Some(message) = reports.next().await {
        let report: InterceptorReport = match serde_json::from_slice(&message.payload) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("discarding invalid report: {error}");
                continue;
            }
        };

        let contacts: Vec<String> = report
            .threats
            .iter()
            .map(|threat| {
                format!(
                    "{} lvl{} at ({:.0}, {:.0}) v({:.0}, {:.0})",
                    &threat.id.to_string()[..8],
                    threat.threat_level,
                    threat.position.x,
                    threat.position.y,
                    threat.speed.x,
                    threat.speed.y,
                )
            })
            .collect();

        println!(
            "report from platform {} — {} interceptor(s) ready, {} contact(s): {}",
            &report.platform_id.to_string()[..8],
            report.interceptors_remaining,
            report.threats.len(),
            if contacts.is_empty() {
                "none".to_string()
            } else {
                contacts.join(", ")
            },
        );
    }

    Ok(())
}
