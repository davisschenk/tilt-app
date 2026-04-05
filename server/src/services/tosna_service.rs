use chrono::{DateTime, Duration, Utc};
use sea_orm::DatabaseConnection;
use shared::{
    BrewEventType, CreateBrewEvent, NutrientAddition, NutrientProduct, NutrientProtocol,
    NutrientTrigger,
};
use uuid::Uuid;

use super::alert_target_service;
use super::brew_event_service;
use super::webhook_dispatcher::{
    self, DAP_G_PER_TSP, FERMAID_K_G_PER_TSP, FERMAID_O_G_PER_TSP, GOFERM_G_PER_TSP,
    NutrientWebhookPayload,
};

// ---------------------------------------------------------------------------
// Yeast strain table
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub struct YeastStrainInfo {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub nitrogen_requirement: &'static str,
    pub alcohol_tolerance_pct: f64,
    pub temp_min_f: f64,
    pub temp_max_f: f64,
    pub notes: &'static str,
}

pub static YEAST_STRAIN_TABLE: &[YeastStrainInfo] = &[
    YeastStrainInfo {
        name: "71B",
        aliases: &["lalvin 71b", "71-b"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 14.0,
        temp_min_f: 59.0,
        temp_max_f: 86.0,
        notes: "Fruit-forward, great for meads; metabolises malic acid",
    },
    YeastStrainInfo {
        name: "D47",
        aliases: &["lalvin d47", "d-47"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 14.0,
        temp_min_f: 50.0,
        temp_max_f: 65.0,
        notes: "Produces significant fusel alcohols above 65°F; keep cool",
    },
    YeastStrainInfo {
        name: "EC-1118",
        aliases: &["ec1118", "champagne yeast", "epernay", "prise de mousse"],
        nitrogen_requirement: "medium",
        alcohol_tolerance_pct: 18.0,
        temp_min_f: 50.0,
        temp_max_f: 86.0,
        notes: "Champagne yeast; very dry, neutral; reliable at high gravity",
    },
    YeastStrainInfo {
        name: "K1-V1116",
        aliases: &["k1v1116", "k1 v1116", "lalvin k1"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 18.0,
        temp_min_f: 50.0,
        temp_max_f: 95.0,
        notes: "Very robust; wide temp range; good for light melomels",
    },
    YeastStrainInfo {
        name: "RC-212",
        aliases: &["rc212", "bourgovin rc212"],
        nitrogen_requirement: "medium",
        alcohol_tolerance_pct: 16.0,
        temp_min_f: 59.0,
        temp_max_f: 86.0,
        notes: "Burgundy style; full-bodied; good for fruit meads",
    },
    YeastStrainInfo {
        name: "D21",
        aliases: &["lalvin d21"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 16.0,
        temp_min_f: 50.0,
        temp_max_f: 81.0,
        notes: "Floral and fruity; low nutrient demand",
    },
    YeastStrainInfo {
        name: "DV10",
        aliases: &["lalvin dv10"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 18.0,
        temp_min_f: 50.0,
        temp_max_f: 86.0,
        notes: "Very high alcohol tolerance; neutral profile",
    },
    YeastStrainInfo {
        name: "QA23",
        aliases: &["lalvin qa23"],
        nitrogen_requirement: "medium",
        alcohol_tolerance_pct: 16.0,
        temp_min_f: 54.0,
        temp_max_f: 86.0,
        notes: "Thiol-enhancing; good for white wine style meads",
    },
    YeastStrainInfo {
        name: "Voss Kveik",
        aliases: &["kveik", "omega voss", "voss"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 15.0,
        temp_min_f: 72.0,
        temp_max_f: 104.0,
        notes: "Fast fermenter; fruity esters; thrives at very high temps",
    },
    YeastStrainInfo {
        name: "M05",
        aliases: &["mangrove jack m05", "mj m05"],
        nitrogen_requirement: "medium",
        alcohol_tolerance_pct: 18.0,
        temp_min_f: 64.0,
        temp_max_f: 82.0,
        notes: "Mangrove Jack mead yeast; clean, slightly fruity",
    },
    YeastStrainInfo {
        name: "US-05",
        aliases: &["us05", "safale us-05", "american ale"],
        nitrogen_requirement: "low",
        alcohol_tolerance_pct: 11.0,
        temp_min_f: 59.0,
        temp_max_f: 75.0,
        notes: "American ale; low gravity meads; clean profile",
    },
];

pub fn lookup_strain(name: &str) -> Option<&'static YeastStrainInfo> {
    let lower = name.to_lowercase();
    YEAST_STRAIN_TABLE.iter().find(|s| {
        s.name.to_lowercase() == lower || s.aliases.iter().any(|a| a.to_lowercase() == lower)
    })
}

// ---------------------------------------------------------------------------
// Temperature safety evaluation (task 9)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub async fn evaluate_temperature_safety(
    db: &DatabaseConnection,
    http_client: &reqwest::Client,
    brew_id: Uuid,
    brew_name: &str,
    yeast_strain: &str,
    temperature_f: f64,
    recorded_at: DateTime<Utc>,
    nutrient_alert_target_id: Option<Uuid>,
) {
    let Some(strain) = lookup_strain(yeast_strain) else {
        return;
    };

    if temperature_f <= strain.temp_max_f {
        return;
    }

    let existing_events = match brew_event_service::find_by_brew(db, brew_id, None, None).await {
        Ok(events) => events,
        Err(e) => {
            tracing::error!(brew_id = %brew_id, error = %e, "Failed to load brew events for temp safety check");
            return;
        }
    };

    let cooldown_ok = existing_events.iter().any(|e| {
        e.event_type == BrewEventType::TemperatureChange
            && e.notes
                .as_deref()
                .map(|n| n.contains("Temperature warning"))
                .unwrap_or(false)
            && recorded_at.signed_duration_since(e.event_time) < Duration::minutes(60)
    });

    if cooldown_ok {
        tracing::debug!(brew_id = %brew_id, "Temp warning suppressed by 60-min cooldown");
        return;
    }

    let excess = temperature_f - strain.temp_max_f;
    let fusel_note = if strain.name == "D47" {
        " D47 produces significant fusel alcohols above 65°F."
    } else {
        ""
    };
    let notes = format!(
        "Temperature warning: {:.1}°F exceeds safe max {:.1}°F for {} by {:.1}°F.{}",
        temperature_f, strain.temp_max_f, strain.name, excess, fusel_note
    );

    if let Err(e) = brew_event_service::create(
        db,
        CreateBrewEvent {
            brew_id,
            event_type: BrewEventType::TemperatureChange,
            label: format!("Temp Warning — {}", strain.name),
            notes: Some(notes.clone()),
            gravity_at_event: None,
            temp_at_event: Some(temperature_f),
            event_time: recorded_at,
        },
    )
    .await
    {
        tracing::error!(brew_id = %brew_id, error = %e, "Failed to create temperature warning event");
    }

    let target = match nutrient_alert_target_id {
        None => return,
        Some(id) => match alert_target_service::find_raw_by_id(db, id).await {
            Ok(Some(t)) if t.enabled => t,
            Ok(_) => return,
            Err(e) => {
                tracing::error!(error = %e, "Failed to load alert target for temp warning");
                return;
            }
        },
    };

    let title = format!("🌡️ Temperature Warning — {brew_name}");
    let detail = format!(
        "{:.1}°F exceeds safe max {:.1}°F for {} by {:.1}°F.{}",
        temperature_f, strain.temp_max_f, strain.name, excess, fusel_note
    );

    use serde_json::json;
    use shared::WebhookFormat;

    {
        let format = serde_json::from_value::<WebhookFormat>(serde_json::Value::String(
            target.format.clone(),
        ))
        .unwrap_or(WebhookFormat::GenericJson);

        let payload = match format {
            WebhookFormat::GenericJson => json!({
                "brew_id": brew_id,
                "brew_name": brew_name,
                "yeast_strain": strain.name,
                "current_temp_f": temperature_f,
                "max_safe_temp_f": strain.temp_max_f,
                "excess_degrees": excess,
                "recorded_at": recorded_at.to_rfc3339(),
            }),
            WebhookFormat::Discord => json!({
                "embeds": [{
                    "title": title,
                    "color": 0xE67E22_u32,
                    "fields": [
                        { "name": "Yeast", "value": strain.name, "inline": true },
                        { "name": "Current Temp", "value": format!("{:.1}°F", temperature_f), "inline": true },
                        { "name": "Safe Max", "value": format!("{:.1}°F", strain.temp_max_f), "inline": true },
                        { "name": "Details", "value": detail.clone(), "inline": false },
                    ],
                    "timestamp": recorded_at.to_rfc3339(),
                }]
            }),
            WebhookFormat::Slack => json!({
                "blocks": [
                    { "type": "header", "text": { "type": "plain_text", "text": title, "emoji": true } },
                    { "type": "section", "fields": [
                        { "type": "mrkdwn", "text": format!("*Yeast:*\n{}", strain.name) },
                        { "type": "mrkdwn", "text": format!("*Current Temp:*\n{:.1}°F", temperature_f) },
                        { "type": "mrkdwn", "text": format!("*Safe Max:*\n{:.1}°F", strain.temp_max_f) },
                        { "type": "mrkdwn", "text": format!("*Details:*\n{}", detail) },
                    ]}
                ]
            }),
        };

        let mut req = http_client.post(&target.url).json(&payload);
        if let Some(ref secret) = target.secret_header {
            req = req.header("Authorization", secret);
        }
        if let Err(e) = req.send().await {
            tracing::warn!(target_name = %target.name, error = %e, "Temp warning dispatch failed");
        }
    }
}

#[allow(dead_code)]
pub fn og_to_brix(og: f64) -> f64 {
    261.3 * (1.0 - 1.0 / og)
}

pub fn nitrogen_factor(requirement: &str) -> f64 {
    match requirement {
        "low" => 0.75,
        "high" => 1.25,
        _ => 0.90,
    }
}

/// Compute required YAN in ppm using the standard TOSNA formula:
/// YAN (ppm) = (OG - 1.0) × 1000 × nitrogen_factor
///
/// e.g. OG 1.092, medium (0.90) → 92 × 0.90 = 82.8 ppm
pub fn required_yan_ppm(og: f64, nitrogen_req: &str) -> f64 {
    (og - 1.0) * 1000.0 * nitrogen_factor(nitrogen_req)
}

pub fn gallons_to_liters(gallons: f64) -> f64 {
    gallons * 3.785_41
}

pub fn fermaid_o_grams_for_yan(yan_ppm: f64, volume_liters: f64) -> f64 {
    (yan_ppm / 40.0) * volume_liters
}

pub fn fermaid_k_grams_for_yan(yan_ppm: f64, volume_liters: f64) -> f64 {
    (yan_ppm / 100.0) * volume_liters
}

#[allow(dead_code)]
pub fn dap_grams_for_yan(yan_ppm: f64, volume_liters: f64) -> f64 {
    (yan_ppm / 210.0) * volume_liters
}

pub fn abv_at_gravity(og: f64, current_gravity: f64) -> f64 {
    (og - current_gravity) * 131.25
}

pub fn sugar_depletion_gravity(og: f64, target_fg: f64, fraction: f64) -> f64 {
    og - (og - target_fg) * fraction
}

pub fn max_inorganic_gravity(og: f64) -> f64 {
    og - 9.0 / 131.25
}

pub fn compute_schedule(
    protocol: NutrientProtocol,
    og: f64,
    target_fg: f64,
    batch_gallons: f64,
    nitrogen_req: &str,
    pitch_time: DateTime<Utc>,
) -> Vec<NutrientAddition> {
    match protocol {
        NutrientProtocol::Tosna2 => {
            tosna_2_schedule(og, target_fg, batch_gallons, nitrogen_req, pitch_time)
        }
        NutrientProtocol::Tosna3 => {
            tosna_3_schedule(og, target_fg, batch_gallons, nitrogen_req, pitch_time)
        }
        NutrientProtocol::AdvancedSna => {
            advanced_sna_schedule(og, target_fg, batch_gallons, nitrogen_req, pitch_time)
        }
    }
}

pub fn tosna_2_schedule(
    og: f64,
    target_fg: f64,
    batch_gallons: f64,
    nitrogen_req: &str,
    pitch_time: DateTime<Utc>,
) -> Vec<NutrientAddition> {
    let volume_liters = gallons_to_liters(batch_gallons);
    let yan_ppm = required_yan_ppm(og, nitrogen_req);
    let total_grams = fermaid_o_grams_for_yan(yan_ppm, volume_liters);
    let per_addition = total_grams / 4.0;

    let g15 = sugar_depletion_gravity(og, target_fg, 0.15);
    let g30 = sugar_depletion_gravity(og, target_fg, 0.30);
    let g45 = sugar_depletion_gravity(og, target_fg, 0.45);
    let g33 = sugar_depletion_gravity(og, target_fg, 0.333_333);

    vec![
        NutrientAddition {
            addition_number: 1,
            product: NutrientProduct::FermaidO,
            amount_grams: per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g15),
            fallback_hours: Some(24),
            due_at: Some(pitch_time + Duration::hours(24)),
        },
        NutrientAddition {
            addition_number: 2,
            product: NutrientProduct::FermaidO,
            amount_grams: per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g30),
            fallback_hours: Some(48),
            due_at: Some(pitch_time + Duration::hours(48)),
        },
        NutrientAddition {
            addition_number: 3,
            product: NutrientProduct::FermaidO,
            amount_grams: per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g45),
            fallback_hours: Some(72),
            due_at: Some(pitch_time + Duration::hours(72)),
        },
        NutrientAddition {
            addition_number: 4,
            product: NutrientProduct::FermaidO,
            amount_grams: per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g33),
            fallback_hours: Some(168),
            due_at: Some(pitch_time + Duration::hours(168)),
        },
    ]
}

pub fn tosna_3_schedule(
    og: f64,
    target_fg: f64,
    batch_gallons: f64,
    nitrogen_req: &str,
    pitch_time: DateTime<Utc>,
) -> Vec<NutrientAddition> {
    let volume_liters = gallons_to_liters(batch_gallons);
    let yan_ppm = required_yan_ppm(og, nitrogen_req);
    let half_yan = yan_ppm / 2.0;
    let k_grams_per_addition = fermaid_k_grams_for_yan(half_yan, volume_liters) / 2.0;
    let o_grams_per_addition = fermaid_o_grams_for_yan(half_yan, volume_liters) / 2.0;

    let g15 = sugar_depletion_gravity(og, target_fg, 0.15);
    let g30 = sugar_depletion_gravity(og, target_fg, 0.30);
    let g45 = sugar_depletion_gravity(og, target_fg, 0.45);
    let g33 = sugar_depletion_gravity(og, target_fg, 0.333_333);

    vec![
        NutrientAddition {
            addition_number: 1,
            product: NutrientProduct::FermaidK,
            amount_grams: k_grams_per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g15),
            fallback_hours: Some(24),
            due_at: Some(pitch_time + Duration::hours(24)),
        },
        NutrientAddition {
            addition_number: 2,
            product: NutrientProduct::FermaidK,
            amount_grams: k_grams_per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g30),
            fallback_hours: Some(48),
            due_at: Some(pitch_time + Duration::hours(48)),
        },
        NutrientAddition {
            addition_number: 3,
            product: NutrientProduct::FermaidO,
            amount_grams: o_grams_per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g45),
            fallback_hours: Some(72),
            due_at: Some(pitch_time + Duration::hours(72)),
        },
        NutrientAddition {
            addition_number: 4,
            product: NutrientProduct::FermaidO,
            amount_grams: o_grams_per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g33),
            fallback_hours: Some(168),
            due_at: Some(pitch_time + Duration::hours(168)),
        },
    ]
}

pub fn advanced_sna_schedule(
    og: f64,
    target_fg: f64,
    batch_gallons: f64,
    nitrogen_req: &str,
    pitch_time: DateTime<Utc>,
) -> Vec<NutrientAddition> {
    let volume_liters = gallons_to_liters(batch_gallons);
    let yan_ppm = required_yan_ppm(og, nitrogen_req);

    let goferm_grams = batch_gallons * 1.25;
    let o_yan = yan_ppm * 0.40;
    let inorganic_yan = yan_ppm * 0.60;

    let o_total = fermaid_o_grams_for_yan(o_yan, volume_liters);
    let k_per_addition = fermaid_k_grams_for_yan(inorganic_yan / 2.0, volume_liters);

    let g15 = sugar_depletion_gravity(og, target_fg, 0.15);
    let g33 = sugar_depletion_gravity(og, target_fg, 0.333_333);
    let inorganic_cutoff = max_inorganic_gravity(og);

    let safe_k_gravity = if inorganic_cutoff > g15 {
        inorganic_cutoff
    } else {
        g15
    };

    vec![
        NutrientAddition {
            addition_number: 1,
            product: NutrientProduct::GoFerm,
            amount_grams: goferm_grams,
            primary_trigger: NutrientTrigger::AtPitch,
            gravity_threshold: None,
            fallback_hours: Some(0),
            due_at: Some(pitch_time),
        },
        NutrientAddition {
            addition_number: 2,
            product: NutrientProduct::FermaidO,
            amount_grams: o_total / 2.0,
            primary_trigger: NutrientTrigger::TimeElapsed,
            gravity_threshold: None,
            fallback_hours: Some(24),
            due_at: Some(pitch_time + Duration::hours(24)),
        },
        NutrientAddition {
            addition_number: 3,
            product: NutrientProduct::FermaidK,
            amount_grams: k_per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(safe_k_gravity),
            fallback_hours: Some(48),
            due_at: Some(pitch_time + Duration::hours(48)),
        },
        NutrientAddition {
            addition_number: 4,
            product: NutrientProduct::FermaidK,
            amount_grams: k_per_addition,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(safe_k_gravity),
            fallback_hours: Some(72),
            due_at: Some(pitch_time + Duration::hours(72)),
        },
        NutrientAddition {
            addition_number: 5,
            product: NutrientProduct::FermaidO,
            amount_grams: o_total / 2.0,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(g33),
            fallback_hours: Some(168),
            due_at: Some(pitch_time + Duration::hours(168)),
        },
    ]
}

fn product_name(p: NutrientProduct) -> &'static str {
    match p {
        NutrientProduct::FermaidO => "Fermaid-O",
        NutrientProduct::FermaidK => "Fermaid-K",
        NutrientProduct::Dap => "DAP",
        NutrientProduct::GoFerm => "GoFerm",
    }
}

fn grams_to_tsp(product: NutrientProduct, grams: f64) -> f64 {
    let g_per_tsp = match product {
        NutrientProduct::FermaidO => FERMAID_O_G_PER_TSP,
        NutrientProduct::FermaidK => FERMAID_K_G_PER_TSP,
        NutrientProduct::Dap => DAP_G_PER_TSP,
        NutrientProduct::GoFerm => GOFERM_G_PER_TSP,
    };
    grams / g_per_tsp
}

fn is_inorganic(p: NutrientProduct) -> bool {
    matches!(p, NutrientProduct::FermaidK | NutrientProduct::Dap)
}

fn is_due(
    addition: &NutrientAddition,
    current_gravity: f64,
    recorded_at: DateTime<Utc>,
    pitch_time: DateTime<Utc>,
) -> (bool, &'static str) {
    match addition.primary_trigger {
        NutrientTrigger::AtPitch => (true, "at_pitch"),
        NutrientTrigger::GravityThreshold => {
            if let Some(thresh) = addition.gravity_threshold
                && current_gravity <= thresh
            {
                return (true, "gravity");
            }
            if let Some(hours) = addition.fallback_hours
                && recorded_at >= pitch_time + Duration::hours(hours as i64)
            {
                return (true, "time_fallback");
            }
            (false, "")
        }
        NutrientTrigger::TimeElapsed => {
            if let Some(hours) = addition.fallback_hours
                && recorded_at >= pitch_time + Duration::hours(hours as i64)
            {
                return (true, "time_fallback");
            }
            (false, "")
        }
    }
}

fn addition_label(num: u8) -> String {
    format!("Addition #{num}")
}

#[allow(clippy::too_many_arguments)]
pub async fn evaluate_due_additions(
    db: &DatabaseConnection,
    http_client: &reqwest::Client,
    brew_id: Uuid,
    brew_name: &str,
    og: f64,
    target_fg: f64,
    batch_gallons: f64,
    nitrogen_req: &str,
    protocol_str: &str,
    pitch_time: DateTime<Utc>,
    current_gravity: f64,
    recorded_at: DateTime<Utc>,
    nutrient_alert_target_id: Option<Uuid>,
) {
    let protocol = NutrientProtocol::from_protocol_str(protocol_str);
    let schedule = compute_schedule(
        protocol,
        og,
        target_fg,
        batch_gallons,
        nitrogen_req,
        pitch_time,
    );

    let existing_events = match brew_event_service::find_by_brew(db, brew_id, None, None).await {
        Ok(events) => events,
        Err(e) => {
            tracing::error!(brew_id = %brew_id, error = %e, "Failed to load brew events for TOSNA evaluation");
            return;
        }
    };

    let completed_additions: std::collections::HashSet<u8> = existing_events
        .iter()
        .filter(|e| e.event_type == shared::BrewEventType::NutrientAddition)
        .filter_map(|e| {
            e.notes.as_deref().and_then(|n| {
                n.strip_prefix("Addition #")
                    .and_then(|rest| rest.split(':').next())
                    .and_then(|num| num.trim().parse::<u8>().ok())
            })
        })
        .collect();

    let current_abv = abv_at_gravity(og, current_gravity);

    let target_opt = match nutrient_alert_target_id {
        None => None,
        Some(id) => match alert_target_service::find_raw_by_id(db, id).await {
            Ok(Some(t)) if t.enabled => Some(t),
            Ok(_) => None,
            Err(e) => {
                tracing::error!(error = %e, "Failed to load alert target for TOSNA notification");
                None
            }
        },
    };

    for addition in &schedule {
        if completed_additions.contains(&addition.addition_number) {
            continue;
        }

        if is_inorganic(addition.product) && current_abv >= 9.0 {
            tracing::info!(
                brew_id = %brew_id,
                addition_number = addition.addition_number,
                current_abv,
                "Skipping inorganic addition — ABV >= 9%"
            );
            continue;
        }

        let (due, reason) = is_due(addition, current_gravity, recorded_at, pitch_time);
        if !due {
            continue;
        }

        let amount_tsp = grams_to_tsp(addition.product, addition.amount_grams);
        let notes = format!(
            "Addition #{}: {:.1}g {} (triggered by {}), gravity={:.4}",
            addition.addition_number,
            addition.amount_grams,
            product_name(addition.product),
            reason,
            current_gravity
        );

        if let Err(e) = brew_event_service::create(
            db,
            CreateBrewEvent {
                brew_id,
                event_type: BrewEventType::NutrientAddition,
                label: addition_label(addition.addition_number),
                notes: Some(notes),
                gravity_at_event: Some(current_gravity),
                temp_at_event: None,
                event_time: recorded_at,
            },
        )
        .await
        {
            tracing::error!(brew_id = %brew_id, addition_number = addition.addition_number, error = %e, "Failed to create NutrientAddition brew event");
            continue;
        }

        let payload = NutrientWebhookPayload {
            brew_id,
            brew_name: brew_name.to_string(),
            addition_number: addition.addition_number,
            nutrient_product: product_name(addition.product).to_string(),
            amount_grams: addition.amount_grams,
            amount_tsp,
            trigger_reason: reason.to_string(),
            current_gravity,
            threshold_gravity: addition.gravity_threshold,
            recorded_at,
        };

        if let Some(target) = &target_opt
            && let Err(e) =
                webhook_dispatcher::dispatch_nutrient_notification(http_client, target, &payload)
                    .await
        {
            tracing::warn!(target_name = %target.name, error = %e, "Nutrient notification dispatch failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn og_to_brix_known_value() {
        let brix = og_to_brix(1.100);
        assert!((brix - 23.77).abs() < 0.1, "Expected ~23.77, got {brix}");
    }

    #[test]
    fn required_yan_ppm_medium() {
        // (1.100 - 1.0) * 1000 * 0.90 = 90 ppm
        let ppm = required_yan_ppm(1.100, "medium");
        assert!((ppm - 90.0).abs() < 0.01, "Expected 90.0 ppm, got {ppm}");
    }

    #[test]
    fn required_yan_ppm_low() {
        // (1.092 - 1.0) * 1000 * 0.75 = 69 ppm
        let ppm = required_yan_ppm(1.092, "low");
        assert!((ppm - 69.0).abs() < 0.01, "Expected 69.0 ppm, got {ppm}");
    }

    #[test]
    fn fermaid_o_grams_matches_meadmakr_1092_1gal_medium() {
        // Meadmakr: OG 1.092, 1 gal, medium nitrogen, TOSNA 2 → ~8g total
        let yan = required_yan_ppm(1.092, "medium");
        let liters = gallons_to_liters(1.0);
        let grams = fermaid_o_grams_for_yan(yan, liters);
        assert!((grams - 7.84).abs() < 0.1, "Expected ~7.84g, got {grams}");
    }

    #[test]
    fn fermaid_o_grams_for_one_gallon_1100() {
        // (1.100 - 1.0) * 1000 * 0.90 = 90 ppm; (90/40) * 3.785 = 8.52g
        let yan = required_yan_ppm(1.100, "medium");
        let liters = gallons_to_liters(1.0);
        let grams = fermaid_o_grams_for_yan(yan, liters);
        assert!((grams - 8.52).abs() < 0.1, "Expected ~8.52g, got {grams}");
    }

    #[test]
    fn abv_calculation() {
        let abv = abv_at_gravity(1.100, 1.020);
        assert!((abv - 10.5).abs() < 0.1, "Expected ~10.5%, got {abv}");
    }

    #[test]
    fn inorganic_cutoff_at_nine_pct() {
        let cutoff = max_inorganic_gravity(1.060);
        let expected = 1.060 - 9.0 / 131.25;
        assert!((cutoff - expected).abs() < f64::EPSILON);
        assert!(abv_at_gravity(1.060, cutoff) < 9.01);
    }

    #[test]
    fn third_sugar_break() {
        let g = sugar_depletion_gravity(1.100, 1.000, 0.333_333);
        assert!((g - 1.0667).abs() < 0.001, "Expected ~1.067, got {g}");
    }

    #[test]
    fn tosna_2_returns_four_fermaid_o_additions() {
        let now = Utc::now();
        let additions = tosna_2_schedule(1.080, 1.010, 1.0, "medium", now);
        assert_eq!(additions.len(), 4);
        for a in &additions {
            assert_eq!(a.product, NutrientProduct::FermaidO);
        }
    }

    #[test]
    fn tosna_2_addition_1_gravity_and_fallback() {
        let now = Utc::now();
        let additions = tosna_2_schedule(1.080, 1.010, 1.0, "medium", now);
        let a1 = &additions[0];
        assert_eq!(a1.primary_trigger, NutrientTrigger::GravityThreshold);
        assert_eq!(a1.fallback_hours, Some(24));
        assert!(a1.gravity_threshold.is_some());
        let expected_g = sugar_depletion_gravity(1.080, 1.010, 0.15);
        assert!((a1.gravity_threshold.unwrap() - expected_g).abs() < 0.0001);
    }

    #[test]
    fn tosna_2_addition_4_is_one_third_break_with_7_day_fallback() {
        let now = Utc::now();
        let additions = tosna_2_schedule(1.080, 1.010, 1.0, "medium", now);
        let a4 = &additions[3];
        assert_eq!(a4.fallback_hours, Some(168));
        let expected_g = sugar_depletion_gravity(1.080, 1.010, 0.333_333);
        assert!((a4.gravity_threshold.unwrap() - expected_g).abs() < 0.0001);
    }

    #[test]
    fn tosna_3_uses_fermaid_k_for_first_two() {
        let now = Utc::now();
        let additions = tosna_3_schedule(1.080, 1.010, 1.0, "medium", now);
        assert_eq!(additions.len(), 4);
        assert_eq!(additions[0].product, NutrientProduct::FermaidK);
        assert_eq!(additions[1].product, NutrientProduct::FermaidK);
        assert_eq!(additions[2].product, NutrientProduct::FermaidO);
        assert_eq!(additions[3].product, NutrientProduct::FermaidO);
    }

    #[test]
    fn advanced_sna_has_five_additions_with_goferm_first() {
        let now = Utc::now();
        let additions = advanced_sna_schedule(1.060, 1.010, 1.0, "medium", now);
        assert_eq!(additions.len(), 5);
        assert_eq!(additions[0].product, NutrientProduct::GoFerm);
        assert_eq!(additions[0].primary_trigger, NutrientTrigger::AtPitch);
    }

    #[test]
    fn advanced_sna_inorganic_clamped_below_nine_pct_abv() {
        let now = Utc::now();
        let og = 1.060;
        let additions = advanced_sna_schedule(og, 1.010, 1.0, "medium", now);
        let cutoff = max_inorganic_gravity(og);
        for a in &additions {
            if a.product == NutrientProduct::FermaidK || a.product == NutrientProduct::Dap {
                if let Some(thresh) = a.gravity_threshold {
                    assert!(
                        thresh >= cutoff,
                        "Inorganic addition threshold {thresh} is below 9% ABV cutoff {cutoff}"
                    );
                }
            }
        }
    }

    #[test]
    fn nitrogen_factor_values() {
        assert!((nitrogen_factor("low") - 0.75).abs() < f64::EPSILON);
        assert!((nitrogen_factor("medium") - 0.90).abs() < f64::EPSILON);
        assert!((nitrogen_factor("high") - 1.25).abs() < f64::EPSILON);
        assert!((nitrogen_factor("unknown") - 0.90).abs() < f64::EPSILON);
    }

    #[test]
    fn strain_table_has_eleven_strains() {
        assert!(YEAST_STRAIN_TABLE.len() >= 11);
    }

    #[test]
    fn strain_71b_is_low_nitrogen() {
        let s = lookup_strain("71B").expect("71B must be in table");
        assert_eq!(s.nitrogen_requirement, "low");
    }

    #[test]
    fn strain_ec1118_is_medium_nitrogen() {
        let s = lookup_strain("EC-1118").expect("EC-1118 must be in table");
        assert_eq!(s.nitrogen_requirement, "medium");
    }

    #[test]
    fn strain_d47_is_low_and_max_65f() {
        let s = lookup_strain("D47").expect("D47 must be in table");
        assert_eq!(s.nitrogen_requirement, "low");
        assert!((s.temp_max_f - 65.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lookup_strain_case_insensitive() {
        let s = lookup_strain("71b").expect("71b lowercase must match");
        assert_eq!(s.name, "71B");
    }

    #[test]
    fn lookup_strain_via_alias_champagne_yeast() {
        let s =
            lookup_strain("champagne yeast").expect("alias 'champagne yeast' must match EC-1118");
        assert_eq!(s.name, "EC-1118");
    }

    #[test]
    fn lookup_strain_unknown_returns_none() {
        assert!(lookup_strain("XYZ-Unknown-9000").is_none());
    }

    #[test]
    fn voss_kveik_temp_range() {
        let s = lookup_strain("Voss Kveik").expect("Voss Kveik in table");
        assert!((s.temp_min_f - 72.0).abs() < f64::EPSILON);
        assert!((s.temp_max_f - 104.0).abs() < f64::EPSILON);
    }

    #[test]
    fn d47_notes_contain_fusel_warning() {
        let s = lookup_strain("D47").unwrap();
        assert!(
            s.notes.to_lowercase().contains("fusel"),
            "D47 notes must mention fusel alcohols"
        );
    }

    #[test]
    fn temperature_safety_fires_for_d47_at_68f() {
        let og = 1.060;
        let current = 1.040;
        let strain = lookup_strain("D47").unwrap();
        assert!(
            68.0 > strain.temp_max_f,
            "68°F should exceed D47 max ({})°F",
            strain.temp_max_f
        );
        let _ = (og, current);
    }

    #[test]
    fn temperature_safety_does_not_fire_below_max() {
        let strain = lookup_strain("D47").unwrap();
        let safe_temp = strain.temp_max_f - 1.0;
        assert!(safe_temp <= strain.temp_max_f);
    }

    #[test]
    fn is_due_gravity_threshold_fires_when_at_or_below() {
        let pitch = Utc::now() - Duration::hours(10);
        let addition = NutrientAddition {
            addition_number: 1,
            product: NutrientProduct::FermaidO,
            amount_grams: 5.0,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(1.070),
            fallback_hours: Some(24),
            due_at: None,
        };
        let (due, reason) = is_due(&addition, 1.070, pitch + Duration::hours(10), pitch);
        assert!(due);
        assert_eq!(reason, "gravity");

        let (due2, _) = is_due(&addition, 1.071, pitch + Duration::hours(10), pitch);
        assert!(!due2, "Should not fire when gravity above threshold");
    }

    #[test]
    fn is_due_time_fallback_fires_after_hours_elapsed() {
        let pitch = Utc::now() - Duration::hours(25);
        let addition = NutrientAddition {
            addition_number: 2,
            product: NutrientProduct::FermaidO,
            amount_grams: 5.0,
            primary_trigger: NutrientTrigger::GravityThreshold,
            gravity_threshold: Some(1.060),
            fallback_hours: Some(24),
            due_at: None,
        };
        let now = Utc::now();
        let (due, reason) = is_due(&addition, 1.080, now, pitch);
        assert!(due, "Should fire via time fallback");
        assert_eq!(reason, "time_fallback");
    }

    #[test]
    fn is_due_at_pitch_always_fires() {
        let pitch = Utc::now();
        let addition = NutrientAddition {
            addition_number: 1,
            product: NutrientProduct::GoFerm,
            amount_grams: 5.0,
            primary_trigger: NutrientTrigger::AtPitch,
            gravity_threshold: None,
            fallback_hours: Some(0),
            due_at: None,
        };
        let (due, reason) = is_due(&addition, 1.060, pitch, pitch);
        assert!(due);
        assert_eq!(reason, "at_pitch");
    }

    #[test]
    fn grams_to_tsp_fermaid_o() {
        let tsp = grams_to_tsp(NutrientProduct::FermaidO, 2.6);
        assert!((tsp - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn grams_to_tsp_fermaid_k() {
        let tsp = grams_to_tsp(NutrientProduct::FermaidK, 2.8);
        assert!((tsp - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn grams_to_tsp_dap() {
        let tsp = grams_to_tsp(NutrientProduct::Dap, 3.1);
        assert!((tsp - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn is_inorganic_identifies_k_and_dap() {
        assert!(is_inorganic(NutrientProduct::FermaidK));
        assert!(is_inorganic(NutrientProduct::Dap));
        assert!(!is_inorganic(NutrientProduct::FermaidO));
        assert!(!is_inorganic(NutrientProduct::GoFerm));
    }

    #[test]
    fn fermaid_o_not_blocked_at_high_abv() {
        let og = 1.080;
        let current = 1.010;
        let abv = abv_at_gravity(og, current);
        assert!(abv >= 9.0, "Test requires ABV >= 9%, got {abv}");
        assert!(
            !is_inorganic(NutrientProduct::FermaidO),
            "Fermaid-O is organic — not blocked"
        );
    }

    #[test]
    fn fermaid_k_blocked_when_abv_at_or_above_nine() {
        let og = 1.080;
        let current = 1.010;
        let abv = abv_at_gravity(og, current);
        assert!(abv >= 9.0, "Test precondition: ABV >= 9%, got {abv}");
        assert!(
            is_inorganic(NutrientProduct::FermaidK),
            "Fermaid-K is inorganic — should be blocked"
        );
    }
}
