use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::models::entities::{alert_rules, alert_targets};
use shared::{AlertMetric, AlertOperator, WebhookFormat};

use super::alert_rule_service::{parse_metric, parse_operator};
use super::preferences_service::Preferences;

pub const FERMAID_O_G_PER_TSP: f64 = 2.6;
pub const FERMAID_K_G_PER_TSP: f64 = 2.8;
pub const DAP_G_PER_TSP: f64 = 3.1;
pub const GOFERM_G_PER_TSP: f64 = 2.0;

#[derive(Debug, Clone)]
pub struct NutrientWebhookPayload {
    pub brew_id: Uuid,
    pub brew_name: String,
    pub addition_number: u8,
    pub nutrient_product: String,
    pub amount_grams: f64,
    pub amount_tsp: f64,
    pub trigger_reason: String,
    pub current_gravity: f64,
    pub threshold_gravity: Option<f64>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum DispatchError {
    HttpError(reqwest::Error),
    ServerError(u16),
    /// The target URL failed SSRF validation (see `webhook_guard`).
    InvalidUrl(String),
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchError::HttpError(e) => write!(f, "HTTP error: {e}"),
            DispatchError::ServerError(code) => write!(f, "Server returned status {code}"),
            DispatchError::InvalidUrl(why) => write!(f, "Blocked webhook URL: {why}"),
        }
    }
}

/// Human label for a metric, with the temperature unit the reader chose.
fn format_metric(metric: &AlertMetric, preferences: &Preferences) -> String {
    match metric {
        AlertMetric::Gravity => "Gravity".to_string(),
        AlertMetric::TemperatureF => format!("Temperature ({})", preferences.temperature_label()),
        AlertMetric::GravityPlateau => "Gravity Plateau".to_string(),
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

/// Everything an alert payload shows, already formatted in the owner's units.
struct AlertText {
    metric: String,
    condition: String,
    actual: String,
}

impl AlertText {
    fn new(
        rule: &alert_rules::Model,
        metric: &AlertMetric,
        operator: &AlertOperator,
        actual: f64,
        preferences: &Preferences,
    ) -> Self {
        let (threshold, actual) = match metric {
            AlertMetric::Gravity => (
                preferences.format_gravity(rule.threshold),
                preferences.format_gravity(actual),
            ),
            AlertMetric::TemperatureF => (
                preferences.format_temperature(rule.threshold),
                preferences.format_temperature(actual),
            ),
            // A plateau threshold is how far gravity may move, not a reading.
            AlertMetric::GravityPlateau => (
                preferences.format_gravity_delta(rule.threshold),
                preferences.format_gravity(actual),
            ),
        };
        let metric = format_metric(metric, preferences);
        let condition = format!("{metric} {} {threshold}", format_operator(operator));
        AlertText {
            metric,
            condition,
            actual,
        }
    }
}

fn build_generic_json(
    rule: &alert_rules::Model,
    operator: &AlertOperator,
    actual: f64,
    text: &AlertText,
    preferences: &Preferences,
    recorded_at: DateTime<Utc>,
) -> serde_json::Value {
    json!({
        "rule_name": rule.name,
        "metric": text.metric,
        "operator": format_operator(operator),
        "threshold": rule.threshold,
        "actual_value": actual,
        "display_condition": text.condition,
        "display_actual": text.actual,
        "display_units": preferences,
        "brew_id": rule.brew_id,
        "hydrometer_id": rule.hydrometer_id,
        "recorded_at": recorded_at.to_rfc3339(),
    })
}

fn build_discord_payload(
    rule: &alert_rules::Model,
    metric: &AlertMetric,
    text: &AlertText,
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
                    "value": text.metric,
                    "inline": true
                },
                {
                    "name": "Condition",
                    "value": text.condition,
                    "inline": true
                },
                {
                    "name": "Actual Value",
                    "value": text.actual,
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
    text: &AlertText,
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
                        "text": format!("*Metric:*\n{}", text.metric)
                    },
                    {
                        "type": "mrkdwn",
                        "text": format!("*Condition:*\n{}", text.condition)
                    },
                    {
                        "type": "mrkdwn",
                        "text": format!("*Actual Value:*\n{}", text.actual)
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

/// The JSON body for a triggered rule, in the target's format.
fn build_alert_payload(
    format: &WebhookFormat,
    preferences: &Preferences,
    rule: &alert_rules::Model,
    gravity: f64,
    temperature_f: f64,
    recorded_at: DateTime<Utc>,
) -> serde_json::Value {
    let metric = parse_metric(&rule.metric);
    let operator = parse_operator(&rule.operator);
    let actual = actual_value(&metric, gravity, temperature_f);
    let text = AlertText::new(rule, &metric, &operator, actual, preferences);

    match format {
        WebhookFormat::GenericJson => {
            build_generic_json(rule, &operator, actual, &text, preferences, recorded_at)
        }
        WebhookFormat::Discord => build_discord_payload(rule, &metric, &text, recorded_at),
        WebhookFormat::Slack => build_slack_payload(rule, &text, recorded_at),
    }
}

pub async fn dispatch(
    preferences: &Preferences,
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
    let payload = build_alert_payload(
        &format,
        preferences,
        rule,
        gravity,
        temperature_f,
        recorded_at,
    );

    // Re-check at send time: the row may predate validation, or the env may have changed.
    super::webhook_guard::validate_url(&target.url).map_err(DispatchError::InvalidUrl)?;
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

pub async fn dispatch_nutrient_notification(
    preferences: &Preferences,
    client: &reqwest::Client,
    target: &alert_targets::Model,
    p: &NutrientWebhookPayload,
) -> Result<(), DispatchError> {
    let format =
        serde_json::from_value::<WebhookFormat>(serde_json::Value::String(target.format.clone()))
            .unwrap_or(WebhookFormat::GenericJson);

    let title = format!(
        "🧪 Nutrient Addition #{} — {}",
        p.addition_number, p.brew_name
    );

    let trigger_detail = if p.trigger_reason == "gravity" {
        format!(
            "Gravity reached {}",
            preferences.format_gravity(p.threshold_gravity.unwrap_or(p.current_gravity))
        )
    } else {
        format!("Time fallback ({})", p.trigger_reason)
    };

    let payload = match format {
        WebhookFormat::GenericJson => json!({
            "brew_id": p.brew_id,
            "brew_name": p.brew_name,
            "addition_number": p.addition_number,
            "nutrient_product": p.nutrient_product,
            "amount_grams": p.amount_grams,
            "amount_tsp": p.amount_tsp,
            "trigger_reason": p.trigger_reason,
            "current_gravity": p.current_gravity,
            "display_current_gravity": preferences.format_gravity(p.current_gravity),
            "threshold_gravity": p.threshold_gravity,
            "recorded_at": p.recorded_at.to_rfc3339(),
        }),
        WebhookFormat::Discord => json!({
            "embeds": [{
                "title": title,
                "color": 0x27AE60_u32,
                "fields": [
                    { "name": "Product", "value": p.nutrient_product, "inline": true },
                    {
                        "name": "Amount",
                        "value": format!("{:.1}g / {:.1} tsp", p.amount_grams, p.amount_tsp),
                        "inline": true
                    },
                    { "name": "Trigger", "value": trigger_detail, "inline": true },
                    {
                        "name": "Current Gravity",
                        "value": preferences.format_gravity(p.current_gravity),
                        "inline": true
                    },
                ],
                "timestamp": p.recorded_at.to_rfc3339(),
            }]
        }),
        WebhookFormat::Slack => json!({
            "blocks": [
                {
                    "type": "header",
                    "text": { "type": "plain_text", "text": title, "emoji": true }
                },
                {
                    "type": "section",
                    "fields": [
                        { "type": "mrkdwn", "text": format!("*Product:*\n{}", p.nutrient_product) },
                        { "type": "mrkdwn", "text": format!("*Amount:*\n{:.1}g / {:.1} tsp", p.amount_grams, p.amount_tsp) },
                        { "type": "mrkdwn", "text": format!("*Trigger:*\n{}", trigger_detail) },
                        { "type": "mrkdwn", "text": format!("*Current Gravity:*\n{}", preferences.format_gravity(p.current_gravity)) },
                    ]
                }
            ]
        }),
    };

    // Re-check at send time: the row may predate validation, or the env may have changed.
    super::webhook_guard::validate_url(&target.url).map_err(DispatchError::InvalidUrl)?;
    let mut request = client.post(&target.url).json(&payload);
    if let Some(ref secret) = target.secret_header {
        request = request.header("Authorization", secret);
    }

    let response = request.send().await.map_err(DispatchError::HttpError)?;
    let status = response.status().as_u16();
    if status >= 400 {
        return Err(DispatchError::ServerError(status));
    }

    tracing::info!(
        target_name = %target.name,
        brew_name = %p.brew_name,
        addition_number = p.addition_number,
        product = %p.nutrient_product,
        status,
        "Nutrient notification dispatched"
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

    // Re-check at send time: the row may predate validation, or the env may have changed.
    super::webhook_guard::validate_url(&target.url).map_err(DispatchError::InvalidUrl)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::preferences_service::{GravityUnit, TemperatureUnit};

    fn rule(metric: &str, operator: &str, threshold: f64) -> alert_rules::Model {
        let now = Utc::now().fixed_offset();
        alert_rules::Model {
            id: Uuid::nil(),
            name: "Rule".into(),
            brew_id: None,
            hydrometer_id: None,
            metric: metric.into(),
            operator: operator.into(),
            threshold,
            alert_target_id: Uuid::nil(),
            enabled: true,
            cooldown_minutes: 60,
            last_triggered_at: None,
            created_at: now,
            updated_at: now,
            window_hours: 24,
        }
    }

    const METRIC: Preferences = Preferences {
        temperature: TemperatureUnit::C,
        gravity: GravityUnit::P,
    };

    #[test]
    fn default_units_keep_fahrenheit_and_sg_text() {
        let p = build_alert_payload(
            &WebhookFormat::Discord,
            &Preferences::default(),
            &rule("gravity", "lte", 1.01),
            1.008,
            68.0,
            Utc::now(),
        );
        let fields = &p["embeds"][0]["fields"];
        assert_eq!(fields[0]["value"], "Gravity");
        assert_eq!(fields[1]["value"], "Gravity ≤ 1.0100 SG");
        assert_eq!(fields[2]["value"], "1.0080 SG");

        let p = build_alert_payload(
            &WebhookFormat::GenericJson,
            &Preferences::default(),
            &rule("temperature_f", "gt", 78.0),
            1.008,
            80.0,
            Utc::now(),
        );
        assert_eq!(p["metric"], "Temperature (°F)");
        assert_eq!(p["threshold"], 78.0);
        assert_eq!(p["display_condition"], "Temperature (°F) > 78.0°F");
        assert_eq!(p["display_actual"], "80.0°F");
    }

    #[test]
    fn preferred_units_flow_into_every_format() {
        let p = build_alert_payload(
            &WebhookFormat::Slack,
            &METRIC,
            &rule("temperature_f", "gt", 77.0),
            1.0,
            86.0,
            Utc::now(),
        );
        let fields = &p["blocks"][1]["fields"];
        assert_eq!(fields[0]["text"], "*Metric:*\nTemperature (°C)");
        assert_eq!(fields[1]["text"], "*Condition:*\nTemperature (°C) > 25.0°C");
        assert_eq!(fields[2]["text"], "*Actual Value:*\n30.0°C");

        let p = build_alert_payload(
            &WebhookFormat::GenericJson,
            &METRIC,
            &rule("gravity_plateau", "plateau", 0.002),
            1.05,
            68.0,
            Utc::now(),
        );
        assert_eq!(
            p["display_condition"], "Gravity Plateau plateau 0.52°P",
            "plateau thresholds are deltas, not absolute gravity"
        );
        assert_eq!(p["display_actual"], "12.39°P");
        assert_eq!(p["display_units"]["gravity"], "P");
    }
}
