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
        AlertMetric::GravityPlateau => "Gravity Plateau",
    }
}

fn format_operator(op: &AlertOperator) -> &'static str {
    match op {
        AlertOperator::Lte => "≤",
        AlertOperator::Gte => "≥",
        AlertOperator::Lt => "<",
        AlertOperator::Gt => ">",
        AlertOperator::Eq => "=",
        AlertOperator::Plateau => "plateau",
    }
}

fn actual_value(metric: &AlertMetric, gravity: f64, temperature_f: f64) -> f64 {
    match metric {
        AlertMetric::Gravity => gravity,
        AlertMetric::TemperatureF => temperature_f,
        AlertMetric::GravityPlateau => gravity,
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
        AlertMetric::Gravity => 0x3498DB,        // blue
        AlertMetric::TemperatureF => 0xE67E22,   // orange
        AlertMetric::GravityPlateau => 0x27AE60, // green
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

/// Send a nutrient addition notification.
pub async fn dispatch_nutrient(
    client: &reqwest::Client,
    target: &alert_targets::Model,
    brew_name: &str,
    brew_id: uuid::Uuid,
    addition: &crate::models::entities::nutrient_additions::Model,
    total_additions: i32,
    one_third_break_sg: f64,
) -> Result<(), DispatchError> {
    let format =
        serde_json::from_value::<WebhookFormat>(serde_json::Value::String(target.format.clone()))
            .unwrap_or(WebhookFormat::GenericJson);

    let trigger_label = if addition.trigger_type == "gravity_or_time" {
        "gravity/time"
    } else {
        "time"
    };

    // Build a human-readable nutrient list
    let mut nutrients = Vec::new();
    if addition.fermaid_o_grams > 0.0 {
        nutrients.push(format!("{:.1}g Fermaid O", addition.fermaid_o_grams));
    }
    if addition.fermaid_k_grams > 0.0 {
        nutrients.push(format!("{:.1}g Fermaid K", addition.fermaid_k_grams));
    }
    if addition.dap_grams > 0.0 {
        nutrients.push(format!("{:.1}g DAP", addition.dap_grams));
    }
    let nutrient_str = nutrients.join(", ");
    let message = format!(
        "Nutrient addition #{} of {} is due for {} — add {}",
        addition.addition_number, total_additions, brew_name, nutrient_str
    );

    let payload = match format {
        WebhookFormat::GenericJson => json!({
            "type": "nutrient_addition",
            "brew_name": brew_name,
            "brew_id": brew_id.to_string(),
            "addition_number": addition.addition_number,
            "total_additions": total_additions,
            "fermaid_o_grams": addition.fermaid_o_grams,
            "fermaid_k_grams": addition.fermaid_k_grams,
            "dap_grams": addition.dap_grams,
            "trigger": trigger_label,
            "one_third_break_sg": one_third_break_sg,
            "message": message,
        }),
        WebhookFormat::Discord => json!({
            "embeds": [{
                "title": format!("🧪 Nutrient Addition #{} — {}", addition.addition_number, brew_name),
                "color": 0xF39C12, // amber
                "description": message,
                "fields": build_nutrient_discord_fields(addition, trigger_label, one_third_break_sg),
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }]
        }),
        WebhookFormat::Slack => json!({
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": format!("🧪 Nutrient Addition #{} — {}", addition.addition_number, brew_name),
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "fields": build_nutrient_slack_fields(addition, trigger_label, &nutrient_str, one_third_break_sg)
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
        tracing::warn!(
            target_name = %target.name,
            addition = addition.addition_number,
            status,
            "Nutrient webhook dispatch failed"
        );
        return Err(DispatchError::ServerError(status));
    }

    tracing::info!(
        target_name = %target.name,
        brew = brew_name,
        addition = addition.addition_number,
        "Nutrient webhook dispatched"
    );
    Ok(())
}

fn build_nutrient_discord_fields(
    addition: &crate::models::entities::nutrient_additions::Model,
    trigger: &str,
    break_sg: f64,
) -> Vec<serde_json::Value> {
    let mut fields = vec![
        json!({"name": "Addition", "value": format!("#{} of 4", addition.addition_number), "inline": true}),
        json!({"name": "Trigger", "value": trigger, "inline": true}),
    ];
    if addition.fermaid_o_grams > 0.0 {
        fields.push(json!({"name": "Fermaid O", "value": format!("{:.1}g", addition.fermaid_o_grams), "inline": true}));
    }
    if addition.fermaid_k_grams > 0.0 {
        fields.push(json!({"name": "Fermaid K", "value": format!("{:.1}g", addition.fermaid_k_grams), "inline": true}));
    }
    if addition.dap_grams > 0.0 {
        fields.push(
            json!({"name": "DAP", "value": format!("{:.1}g", addition.dap_grams), "inline": true}),
        );
    }
    fields.push(
        json!({"name": "1/3 Sugar Break", "value": format!("{:.3}", break_sg), "inline": true}),
    );
    fields
}

fn build_nutrient_slack_fields(
    addition: &crate::models::entities::nutrient_additions::Model,
    trigger: &str,
    nutrient_str: &str,
    break_sg: f64,
) -> Vec<serde_json::Value> {
    vec![
        json!({"type": "mrkdwn", "text": format!("*Addition:*\n#{} of 4", addition.addition_number)}),
        json!({"type": "mrkdwn", "text": format!("*Nutrients:*\n{nutrient_str}")}),
        json!({"type": "mrkdwn", "text": format!("*Trigger:*\n{trigger}")}),
        json!({"type": "mrkdwn", "text": format!("*1/3 Sugar Break:*\n{:.3}", break_sg)}),
    ]
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
