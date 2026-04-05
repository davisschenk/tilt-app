use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::alert_rules::{self, ActiveModel, Column, Entity as AlertRule};
use shared::{AlertMetric, AlertOperator, AlertRuleResponse, CreateAlertRule, UpdateAlertRule};

pub fn parse_metric(s: &str) -> AlertMetric {
    serde_json::from_value::<AlertMetric>(serde_json::Value::String(s.to_string()))
        .unwrap_or(AlertMetric::Gravity)
}

pub fn parse_operator(s: &str) -> AlertOperator {
    serde_json::from_value::<AlertOperator>(serde_json::Value::String(s.to_string()))
        .unwrap_or(AlertOperator::Lte)
}

fn enum_to_string<T: serde::Serialize>(val: T) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn model_to_response(model: alert_rules::Model) -> AlertRuleResponse {
    AlertRuleResponse {
        id: model.id,
        name: model.name,
        brew_id: model.brew_id,
        hydrometer_id: model.hydrometer_id,
        metric: parse_metric(&model.metric),
        operator: parse_operator(&model.operator),
        threshold: model.threshold,
        alert_target_id: model.alert_target_id,
        enabled: model.enabled,
        cooldown_minutes: model.cooldown_minutes,
        window_hours: model.window_hours,
        last_triggered_at: model.last_triggered_at.map(Into::into),
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    }
}

pub async fn find_all(
    db: &DatabaseConnection,
    brew_id: Option<Uuid>,
    hydrometer_id: Option<Uuid>,
) -> Result<Vec<AlertRuleResponse>, DbErr> {
    let mut query = AlertRule::find();
    if let Some(bid) = brew_id {
        query = query.filter(Column::BrewId.eq(bid));
    }
    if let Some(hid) = hydrometer_id {
        query = query.filter(Column::HydrometerId.eq(hid));
    }
    let models = query.all(db).await?;
    Ok(models.into_iter().map(model_to_response).collect())
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<AlertRuleResponse>, DbErr> {
    let model = AlertRule::find_by_id(id).one(db).await?;
    Ok(model.map(model_to_response))
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreateAlertRule,
) -> Result<AlertRuleResponse, DbErr> {
    let now = Utc::now().into();
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(input.name),
        brew_id: Set(input.brew_id),
        hydrometer_id: Set(input.hydrometer_id),
        metric: Set(enum_to_string(input.metric)),
        operator: Set(enum_to_string(input.operator)),
        threshold: Set(input.threshold),
        alert_target_id: Set(input.alert_target_id),
        enabled: Set(input.enabled.unwrap_or(true)),
        cooldown_minutes: Set(input.cooldown_minutes.unwrap_or(60)),
        window_hours: Set(input.window_hours.unwrap_or(24)),
        last_triggered_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let result = AlertRule::insert(model).exec_with_returning(db).await?;
    Ok(model_to_response(result))
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateAlertRule,
) -> Result<Option<AlertRuleResponse>, DbErr> {
    let Some(existing) = AlertRule::find_by_id(id).one(db).await? else {
        return Ok(None);
    };

    let mut active: ActiveModel = existing.into();

    if let Some(name) = input.name {
        active.name = Set(name);
    }
    if let Some(metric) = input.metric {
        active.metric = Set(enum_to_string(metric));
    }
    if let Some(operator) = input.operator {
        active.operator = Set(enum_to_string(operator));
    }
    if let Some(threshold) = input.threshold {
        active.threshold = Set(threshold);
    }
    if let Some(alert_target_id) = input.alert_target_id {
        active.alert_target_id = Set(alert_target_id);
    }
    if let Some(brew_id) = input.brew_id {
        active.brew_id = Set(Some(brew_id));
    }
    if let Some(hydrometer_id) = input.hydrometer_id {
        active.hydrometer_id = Set(Some(hydrometer_id));
    }
    if let Some(cooldown_minutes) = input.cooldown_minutes {
        active.cooldown_minutes = Set(cooldown_minutes);
    }
    if let Some(window_hours) = input.window_hours {
        active.window_hours = Set(window_hours);
    }
    if let Some(enabled) = input.enabled {
        active.enabled = Set(enabled);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(db).await?;
    Ok(Some(model_to_response(updated)))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let result = AlertRule::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}

/// Find all enabled alert rules that match the given reading and whose cooldown has expired.
/// Returns the raw DB models so the dispatcher can access alert_target_id directly.
pub async fn find_triggered_rules(
    db: &DatabaseConnection,
    gravity: f64,
    temperature_f: f64,
    brew_id: Option<Uuid>,
    hydrometer_id: Option<Uuid>,
) -> Result<Vec<alert_rules::Model>, DbErr> {
    let all_rules = AlertRule::find()
        .filter(Column::Enabled.eq(true))
        .all(db)
        .await?;

    let now = Utc::now();

    Ok(all_rules
        .into_iter()
        .filter(|rule| {
            // Scope check: if the rule specifies a brew_id, it must match
            if let Some(rule_brew) = rule.brew_id
                && brew_id != Some(rule_brew)
            {
                return false;
            }
            // Scope check: if the rule specifies a hydrometer_id, it must match
            if let Some(rule_hydro) = rule.hydrometer_id
                && hydrometer_id != Some(rule_hydro)
            {
                return false;
            }

            // Cooldown check
            if let Some(last) = rule.last_triggered_at {
                let last_utc: chrono::DateTime<Utc> = last.into();
                let cooldown = chrono::Duration::minutes(i64::from(rule.cooldown_minutes));
                if now - last_utc < cooldown {
                    return false;
                }
            }

            // Plateau rules are evaluated separately (need historical readings)
            let metric = parse_metric(&rule.metric);
            if metric == AlertMetric::GravityPlateau {
                return false; // plateau rules skipped in per-reading evaluation
            }

            // Get the actual value based on metric
            let actual = match metric {
                AlertMetric::Gravity => gravity,
                AlertMetric::TemperatureF => temperature_f,
                AlertMetric::GravityPlateau => unreachable!(),
            };

            // Evaluate the operator
            let op = parse_operator(&rule.operator);
            match op {
                AlertOperator::Lte => actual <= rule.threshold,
                AlertOperator::Gte => actual >= rule.threshold,
                AlertOperator::Lt => actual < rule.threshold,
                AlertOperator::Gt => actual > rule.threshold,
                AlertOperator::Eq => (actual - rule.threshold).abs() < f64::EPSILON,
                AlertOperator::Plateau => false, // handled by evaluate_plateau_rule
            }
        })
        .collect())
}

/// Find all enabled GravityPlateau rules that match the current brew/hydrometer scope,
/// have passed their cooldown, and whose gravity has been stable within the rule's window.
pub async fn find_triggered_plateau_rules(
    db: &DatabaseConnection,
    brew_id: Option<Uuid>,
    hydrometer_id: Option<Uuid>,
) -> Result<Vec<alert_rules::Model>, DbErr> {
    let plateau_rules = AlertRule::find()
        .filter(Column::Enabled.eq(true))
        .filter(Column::Metric.eq("gravity_plateau"))
        .all(db)
        .await?;

    let now = Utc::now();
    let mut triggered = Vec::new();

    for rule in plateau_rules {
        // Scope check
        if let Some(rule_brew) = rule.brew_id
            && brew_id != Some(rule_brew)
        {
            continue;
        }
        if let Some(rule_hydro) = rule.hydrometer_id
            && hydrometer_id != Some(rule_hydro)
        {
            continue;
        }

        // Cooldown check
        if let Some(last) = rule.last_triggered_at {
            let last_utc: chrono::DateTime<Utc> = last.into();
            let cooldown = chrono::Duration::minutes(i64::from(rule.cooldown_minutes));
            if now - last_utc < cooldown {
                continue;
            }
        }

        // Fetch historical readings within the rule's window
        let window_start = now - chrono::Duration::hours(i64::from(rule.window_hours));
        let query = shared::ReadingsQuery {
            brew_id: rule.brew_id.or(brew_id),
            hydrometer_id: rule.hydrometer_id.or(hydrometer_id),
            since: Some(window_start),
            until: Some(now),
            limit: None,
        };

        let readings = match crate::services::reading_service::find_filtered(db, &query).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(rule_id = %rule.id, error = %e, "Failed to fetch readings for plateau evaluation");
                continue;
            }
        };

        if readings.len() < 2 {
            continue;
        }

        // Convert to (hours_elapsed, gravity) format expected by evaluate_plateau_rule
        let earliest = readings.iter().map(|r| r.recorded_at).min().unwrap();
        let window_data: Vec<(f64, f64)> = readings
            .iter()
            .map(|r| {
                let hours = (r.recorded_at - earliest).num_seconds() as f64 / 3600.0;
                (hours, r.gravity)
            })
            .collect();

        if evaluate_plateau_rule(&window_data, rule.threshold) {
            triggered.push(rule);
        }
    }

    Ok(triggered)
}

/// Evaluate a gravity plateau rule against a slice of (recorded_at_hours, gravity) tuples.
///
/// Returns true when max(gravity) - min(gravity) over the window is <= threshold,
/// meaning gravity has been stable. Returns false if fewer than 2 readings in window.
pub fn evaluate_plateau_rule(readings_in_window: &[(f64, f64)], threshold: f64) -> bool {
    if readings_in_window.len() < 2 {
        return false;
    }
    let max_g = readings_in_window
        .iter()
        .map(|(_, g)| *g)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_g = readings_in_window
        .iter()
        .map(|(_, g)| *g)
        .fold(f64::INFINITY, f64::min);
    (max_g - min_g) <= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat(g: f64, n: usize) -> Vec<(f64, f64)> {
        (0..n).map(|i| (i as f64, g)).collect()
    }

    #[test]
    fn plateau_true_when_gravity_flat_within_threshold() {
        let readings: Vec<(f64, f64)> = (0..24)
            .map(|i| (i as f64, 1.012 + (i as f64 * 0.00005)))
            .collect();
        assert!(evaluate_plateau_rule(&readings, 0.002));
    }

    #[test]
    fn plateau_false_when_gravity_drops_more_than_threshold() {
        let readings: Vec<(f64, f64)> = vec![(0.0, 1.030), (12.0, 1.025), (24.0, 1.020)];
        assert!(!evaluate_plateau_rule(&readings, 0.002));
    }

    #[test]
    fn plateau_false_when_fewer_than_2_readings() {
        assert!(!evaluate_plateau_rule(&[(0.0, 1.012)], 0.002));
        assert!(!evaluate_plateau_rule(&[], 0.002));
    }

    #[test]
    fn plateau_scan_rate_agnostic_dense_vs_sparse() {
        // Same gravity range, different density — both should trigger
        let dense: Vec<(f64, f64)> = (0..96).map(|i| (i as f64 * 0.25, 1.012)).collect();
        let sparse: Vec<(f64, f64)> = (0..4).map(|i| (i as f64 * 6.0, 1.012)).collect();
        assert!(evaluate_plateau_rule(&dense, 0.002));
        assert!(evaluate_plateau_rule(&sparse, 0.002));
    }

    #[test]
    fn alert_metric_gravity_plateau_serializes() {
        let s = serde_json::to_string(&shared::AlertMetric::GravityPlateau).unwrap();
        assert_eq!(s, "\"gravity_plateau\"");
        let d: shared::AlertMetric = serde_json::from_str("\"gravity_plateau\"").unwrap();
        assert_eq!(d, shared::AlertMetric::GravityPlateau);
    }

    #[test]
    fn alert_operator_plateau_serializes() {
        let s = serde_json::to_string(&shared::AlertOperator::Plateau).unwrap();
        assert_eq!(s, "\"plateau\"");
        let d: shared::AlertOperator = serde_json::from_str("\"plateau\"").unwrap();
        assert_eq!(d, shared::AlertOperator::Plateau);
    }
}

/// Update last_triggered_at for a rule after successful dispatch.
pub async fn update_last_triggered(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    let Some(existing) = AlertRule::find_by_id(id).one(db).await? else {
        return Ok(());
    };
    let mut active: ActiveModel = existing.into();
    active.last_triggered_at = Set(Some(Utc::now().into()));
    active.update(db).await?;
    Ok(())
}
