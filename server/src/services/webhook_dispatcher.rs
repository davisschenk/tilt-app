#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::models::entities::{alert_rules, alert_targets};
use shared::{AlertMetric, AlertOperator, WebhookFormat};

use super::alert_rule_service::{parse_metric, parse_operator};

#[derive(Debug)]
pub enum DispatchError {
    HttpError(reqwest::Error),
    ServerError(u16),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::HttpError(e) => write!(f, "HTTP error: {e}"),
            DispatchError::ServerError(code) => write!(f, "Server returned status {code}"),
        }
    }
}

fn format_metric(metric: &AlertMetric) -> &'static str {
    match metric {
        AlertMetric::Gravity => "Gravity",
        AlertMetric::TemperatureF => "Temperature (°F)",
    }
}

fn format_operator(op: &AlertOperator) -> &'static str {
    match op {
        AlertOperator::Lte => "≤",
        AlertOperator::Gte => "≥",
        AlertOperator::Lt => "<",
        AlertOperator::Gt => ">",
        AlertOperator::Eq => "=",
    }
}

fn actual_value(metric: &AlertMetric, gravity: f64, temperature_f: f64) -> f64 {
    match metric {
        AlertMetric::Gravity => gravity,
        AlertMetric::TemperatureF => temperature_f,
    }
}

fn build_generic_json(
    rule: &alert_rules::Model,
    metric: &AlertMetric,
    operator: &AlertOperator,
    actual: f64,
    recorded_at: DateTime<Utc>,
) -> serde_json::Value {
    json!({
        "rule_name": rule.name,
        "metric": format_metric(metric),
        "operator": format_operator(operator),
        "threshold": rule.threshold,
        "actual_value": actual,
        "brew_id": rule.brew_id,
        "hydrometer_id": rule.hydrometer_id,
        "recorded_at": recorded_at.to_rfc3339(),
    })
}

fn build_discord_payload(
    rule: &alert_rules::Model,
    metric: &AlertMetric,
    operator: &AlertOperator,
    actual: f64,
    recorded_at: DateTime<Utc>,
) -> serde_json::Value {
    let color = match metric {
        AlertMetric::Gravity => 0x3498DB,      // blue
        AlertMetric::TemperatureF => 0xE67E22, // orange
    };

    json!({
        "embeds": [{
            "title": format!("🚨 {}", rule.name),
            "color": color,
            "fields": [
                {
                    "name": "Metric",
                    "value": format_metric(metric),
                    "inline": true
                },
                {
                    "name": "Condition",
                    "value": format!("{} {} {}", format_metric(metric), format_operator(operator), rule.threshold),
                    "inline": true
                },
                {
                    "name": "Actual Value",
                    "value": format!("{actual:.4}"),
                    "inline": true
                },
                {
                    "name": "Recorded At",
                    "value": recorded_at.to_rfc3339(),
                    "inline": false
                }
            ],
            "timestamp": recorded_at.to_rfc3339(),
        }]
    })
}

fn build_slack_payload(
    rule: &alert_rules::Model,
    metric: &AlertMetric,
    operator: &AlertOperator,
    actual: f64,
    recorded_at: DateTime<Utc>,
) -> serde_json::Value {
    json!({
        "blocks": [
            {
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": format!("🚨 {}", rule.name),
                    "emoji": true
                }
            },
            {
                "type": "section",
                "fields": [
                    {
                        "type": "mrkdwn",
                        "text": format!("*Metric:*\n{}", format_metric(metric))
                    },
                    {
                        "type": "mrkdwn",
                        "text": format!("*Condition:*\n{} {} {}", format_metric(metric), format_operator(operator), rule.threshold)
                    },
                    {
                        "type": "mrkdwn",
                        "text": format!("*Actual Value:*\n{actual:.4}")
                    },
                    {
                        "type": "mrkdwn",
                        "text": format!("*Recorded At:*\n{}", recorded_at.to_rfc3339())
                    }
                ]
            }
        ]
    })
}

pub async fn dispatch(
    client: &reqwest::Client,
    target: &alert_targets::Model,
    rule: &alert_rules::Model,
    gravity: f64,
    temperature_f: f64,
    recorded_at: DateTime<Utc>,
) -> Result<(), DispatchError> {
    let format =
        serde_json::from_value::<WebhookFormat>(serde_json::Value::String(target.format.clone()))
            .unwrap_or(WebhookFormat::GenericJson);

    let metric = parse_metric(&rule.metric);
    let operator = parse_operator(&rule.operator);
    let actual = actual_value(&metric, gravity, temperature_f);

    let payload = match format {
        WebhookFormat::GenericJson => {
            build_generic_json(rule, &metric, &operator, actual, recorded_at)
        }
        WebhookFormat::Discord => {
            build_discord_payload(rule, &metric, &operator, actual, recorded_at)
        }
        WebhookFormat::Slack => build_slack_payload(rule, &metric, &operator, actual, recorded_at),
    };

    let mut request = client.post(&target.url).json(&payload);

    if let Some(ref secret) = target.secret_header {
        request = request.header("Authorization", secret);
    }

    let response = request.send().await.map_err(DispatchError::HttpError)?;

    let status = response.status().as_u16();
    if status >= 400 {
        tracing::warn!(
            target_name = %target.name,
            rule_name = %rule.name,
            status,
            "Webhook dispatch failed"
        );
        return Err(DispatchError::ServerError(status));
    }

    tracing::info!(
        target_name = %target.name,
        rule_name = %rule.name,
        status,
        "Webhook dispatched successfully"
    );

    Ok(())
}

/// Send a test payload to verify webhook configuration.
pub async fn dispatch_test(
    client: &reqwest::Client,
    target: &alert_targets::Model,
) -> Result<u16, DispatchError> {
    let format =
        serde_json::from_value::<WebhookFormat>(serde_json::Value::String(target.format.clone()))
            .unwrap_or(WebhookFormat::GenericJson);

    let now = Utc::now();
    let test_rule_name = "Test Alert";

    let payload = match format {
        WebhookFormat::GenericJson => json!({
            "rule_name": test_rule_name,
            "metric": "Gravity",
            "operator": "≤",
            "threshold": 1.010,
            "actual_value": 1.008,
            "brew_id": null,
            "hydrometer_id": null,
            "recorded_at": now.to_rfc3339(),
            "test": true
        }),
        WebhookFormat::Discord => json!({
            "embeds": [{
                "title": format!("🧪 {test_rule_name}"),
                "description": "This is a test notification from Tilt Hydrometer Platform.",
                "color": 0x2ECC71,
                "fields": [
                    { "name": "Metric", "value": "Gravity", "inline": true },
                    { "name": "Condition", "value": "Gravity ≤ 1.010", "inline": true },
                    { "name": "Actual Value", "value": "1.0080", "inline": true }
                ],
                "timestamp": now.to_rfc3339(),
            }]
        }),
        WebhookFormat::Slack => json!({
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": format!("🧪 {test_rule_name}"),
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "fields": [
                        { "type": "mrkdwn", "text": "*This is a test notification from Tilt Hydrometer Platform.*" },
                        { "type": "mrkdwn", "text": "*Metric:*\nGravity" },
                        { "type": "mrkdwn", "text": "*Condition:*\nGravity ≤ 1.010" },
                        { "type": "mrkdwn", "text": "*Actual Value:*\n1.0080" }
                    ]
                }
            ]
        }),
    };

    let mut request = client.post(&target.url).json(&payload);

    if let Some(ref secret) = target.secret_header {
        request = request.header("Authorization", secret);
    }

    let response = request.send().await.map_err(DispatchError::HttpError)?;
    let status = response.status().as_u16();

    if status >= 400 {
        return Err(DispatchError::ServerError(status));
    }

    Ok(status)
}
