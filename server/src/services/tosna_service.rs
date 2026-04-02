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

pub fn og_to_brix(og: f64) -> f64 {
    261.3 * (1.0 - 1.0 / og)
}

pub fn sugar_g_per_l(og: f64) -> f64 {
    og_to_brix(og) * og * 10.0
}

pub fn nitrogen_factor(requirement: &str) -> f64 {
    match requirement {
        "low" => 0.75,
        "high" => 1.25,
        _ => 0.90,
    }
}

pub fn required_yan_ppm(og: f64, nitrogen_req: &str) -> f64 {
    sugar_g_per_l(og) * nitrogen_factor(nitrogen_req)
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

    let targets = match alert_target_service::find_all_raw(db).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "Failed to load alert targets for TOSNA notification");
            return;
        }
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

        for target in &targets {
            if !target.enabled {
                continue;
            }
            if let Err(e) =
                webhook_dispatcher::dispatch_nutrient_notification(http_client, target, &payload)
                    .await
            {
                tracing::warn!(target_name = %target.name, error = %e, "Nutrient notification dispatch failed");
            }
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
    fn sugar_g_per_l_uses_og_multiplier() {
        let s = sugar_g_per_l(1.100);
        assert!(
            (s - 261.4).abs() < 1.0,
            "Expected ~261.4 g/L (corrected formula), got {s}"
        );
        let simple = og_to_brix(1.100) * 10.0;
        assert!(
            (s - simple).abs() > 5.0,
            "Corrected formula should differ meaningfully from Brix*10 at high OG"
        );
    }

    #[test]
    fn required_yan_ppm_medium() {
        let ppm = required_yan_ppm(1.100, "medium");
        assert!((ppm - 235.0).abs() < 5.0, "Expected ~235 ppm, got {ppm}");
    }

    #[test]
    fn fermaid_o_grams_for_one_gallon_1100() {
        let yan = required_yan_ppm(1.100, "medium");
        let liters = gallons_to_liters(1.0);
        let grams = fermaid_o_grams_for_yan(yan, liters);
        assert!((grams - 22.2).abs() < 1.0, "Expected ~22.2g, got {grams}");
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
