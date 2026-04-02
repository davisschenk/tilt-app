use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::nutrient_additions::{self, Entity as NutrientAddition};
use crate::models::entities::nutrient_schedules::{self, Entity as NutrientSchedule};
use shared::{
    NitrogenRequirement, NutrientAdditionDetail, NutrientAdditionResponse,
    NutrientCalculateRequest, NutrientCalculateResponse, NutrientProtocol,
    NutrientScheduleResponse, NutrientTriggerType,
};

fn enum_to_string<T: serde::Serialize>(val: T) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

// --- Pure calculation functions ---

const LITERS_PER_GALLON: f64 = 3.785;

// Nutrient YAN contributions (ppm per g/L)
const FERMAID_O_YAN_PER_GL: f64 = 40.0;
const FERMAID_K_YAN_PER_GL: f64 = 100.0;
const DAP_YAN_PER_GL: f64 = 210.0;

// Max dosage per addition (g/L)
const FERMAID_O_MAX_GL: f64 = 0.45;
const FERMAID_K_MAX_GL: f64 = 0.50;
const DAP_MAX_GL: f64 = 0.96;

// GoFerm effective YAN per g/L
const GO_FERM_YAN_PER_GL: f64 = 30.0;

const NUM_ADDITIONS: i32 = 4;

pub fn sg_to_brix(sg: f64) -> f64 {
    (182.4601 * sg.powi(3)) - (775.6821 * sg.powi(2)) + (1262.7794 * sg) - 669.5622
}

pub fn nitrogen_factor(req: NitrogenRequirement) -> f64 {
    match req {
        NitrogenRequirement::Low => 0.75,
        NitrogenRequirement::Medium => 0.90,
        NitrogenRequirement::High => 1.25,
    }
}

pub fn calculate_yan_requirement(og: f64, nitrogen_req: NitrogenRequirement) -> f64 {
    let brix = sg_to_brix(og);
    brix * 10.0 * nitrogen_factor(nitrogen_req)
}

pub fn one_third_sugar_break(og: f64) -> f64 {
    1.0 + (2.0 * (og - 1.0) / 3.0)
}

pub fn calculate_yeast_grams(batch_size_gallons: f64) -> f64 {
    (2.0 * batch_size_gallons).ceil()
}

pub fn calculate_go_ferm(yeast_grams: f64) -> (f64, f64) {
    let packets = (yeast_grams / 5.0).ceil();
    let go_ferm_grams = 1.25 * packets * 5.0;
    let rehydration_water_ml = go_ferm_grams * 20.0;
    (go_ferm_grams, rehydration_water_ml)
}

fn allocate_nutrients_per_addition(
    yan_per_addition: f64,
    protocol: NutrientProtocol,
    batch_liters: f64,
    allow_dap: bool,
) -> (f64, f64, f64, bool) {
    let mut remaining_yan = yan_per_addition;
    let mut capped = false;

    // Fermaid O
    let o_gl = (remaining_yan / FERMAID_O_YAN_PER_GL).min(FERMAID_O_MAX_GL);
    let o_yan = o_gl * FERMAID_O_YAN_PER_GL;
    remaining_yan -= o_yan;
    if remaining_yan < 0.0 {
        remaining_yan = 0.0;
    }
    let o_grams = o_gl * batch_liters;

    if o_gl >= FERMAID_O_MAX_GL && remaining_yan > 0.0 && protocol == NutrientProtocol::FermaidO {
        capped = true;
    }

    // Fermaid K
    let k_grams;
    if matches!(
        protocol,
        NutrientProtocol::FermaidOK | NutrientProtocol::FermaidOKDap
    ) && remaining_yan > 0.0
    {
        let k_gl = (remaining_yan / FERMAID_K_YAN_PER_GL).min(FERMAID_K_MAX_GL);
        let k_yan = k_gl * FERMAID_K_YAN_PER_GL;
        remaining_yan -= k_yan;
        if remaining_yan < 0.0 {
            remaining_yan = 0.0;
        }
        k_grams = k_gl * batch_liters;

        if k_gl >= FERMAID_K_MAX_GL
            && remaining_yan > 0.0
            && protocol == NutrientProtocol::FermaidOK
        {
            capped = true;
        }
    } else {
        k_grams = 0.0;
    }

    // DAP
    let dap_grams;
    if protocol == NutrientProtocol::FermaidOKDap && remaining_yan > 0.0 && allow_dap {
        let dap_gl = (remaining_yan / DAP_YAN_PER_GL).min(DAP_MAX_GL);
        let dap_yan = dap_gl * DAP_YAN_PER_GL;
        remaining_yan -= dap_yan;
        if remaining_yan < 0.0 {
            remaining_yan = 0.0;
        }
        dap_grams = dap_gl * batch_liters;

        if dap_gl >= DAP_MAX_GL && remaining_yan > 0.0 {
            capped = true;
        }
    } else {
        dap_grams = 0.0;
    }

    (round1(o_grams), round1(k_grams), round1(dap_grams), capped)
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

pub fn calculate_nutrient_plan(req: &NutrientCalculateRequest) -> NutrientCalculateResponse {
    let total_yan = round2(calculate_yan_requirement(req.og, req.nitrogen_requirement));

    let yeast_grams = calculate_yeast_grams(req.batch_size_gallons);
    let (go_ferm_grams, rehydration_water_ml) = calculate_go_ferm(yeast_grams);

    // GoFerm offset
    let go_ferm_yan_offset = if req.go_ferm_offset {
        let batch_liters = req.batch_size_gallons * LITERS_PER_GALLON;
        round2((go_ferm_grams / batch_liters) * GO_FERM_YAN_PER_GL)
    } else {
        0.0
    };

    let fruit_yan_offset = req.fruit_offset_ppm.max(0.0);
    let effective_yan = (total_yan - go_ferm_yan_offset - fruit_yan_offset).max(0.0);
    let effective_yan = round2(effective_yan);

    let batch_liters = req.batch_size_gallons * LITERS_PER_GALLON;
    let yan_per_addition = effective_yan / NUM_ADDITIONS as f64;

    let break_sg = round2(one_third_sugar_break(req.og));

    let mut additions = Vec::with_capacity(NUM_ADDITIONS as usize);
    let mut any_capped = false;

    for i in 1..=NUM_ADDITIONS {
        let allow_dap = i < NUM_ADDITIONS; // No DAP in final addition
        let (o, k, d, capped) = allocate_nutrients_per_addition(
            yan_per_addition,
            req.nutrient_protocol,
            batch_liters,
            allow_dap,
        );
        if capped {
            any_capped = true;
        }

        let (trigger_type, target_hours, target_gravity) = if i < NUM_ADDITIONS {
            (NutrientTriggerType::Time, Some(i as f64 * 24.0), None)
        } else {
            (
                NutrientTriggerType::GravityOrTime,
                Some(168.0), // day 7 fallback
                Some(break_sg),
            )
        };

        additions.push(NutrientAdditionDetail {
            addition_number: i,
            fermaid_o_grams: o,
            fermaid_k_grams: k,
            dap_grams: d,
            trigger_type,
            target_hours,
            target_gravity,
        });
    }

    NutrientCalculateResponse {
        total_yan_ppm: total_yan,
        effective_yan_ppm: effective_yan,
        go_ferm_yan_offset_ppm: go_ferm_yan_offset,
        fruit_yan_offset_ppm: fruit_yan_offset,
        one_third_break_sg: break_sg,
        go_ferm_grams,
        yeast_grams,
        rehydration_water_ml,
        additions,
        max_dosage_capped: any_capped,
    }
}

// --- Schedule CRUD ---

fn schedule_model_to_response(
    schedule: nutrient_schedules::Model,
    addition_models: Vec<nutrient_additions::Model>,
) -> NutrientScheduleResponse {
    let additions = addition_models
        .into_iter()
        .map(|a| NutrientAdditionResponse {
            id: a.id,
            addition_number: a.addition_number,
            fermaid_o_grams: a.fermaid_o_grams,
            fermaid_k_grams: a.fermaid_k_grams,
            dap_grams: a.dap_grams,
            trigger_type: shared::parse_nutrient_trigger_type(&a.trigger_type),
            target_hours: a.target_hours,
            target_gravity: a.target_gravity,
            notified_at: a.notified_at.map(chrono::DateTime::<Utc>::from),
            created_at: a.created_at.into(),
        })
        .collect();

    NutrientScheduleResponse {
        id: schedule.id,
        brew_id: schedule.brew_id,
        batch_size_gallons: schedule.batch_size_gallons,
        og: schedule.og,
        nitrogen_requirement: shared::parse_nitrogen_requirement(&schedule.nitrogen_requirement),
        nutrient_protocol: shared::parse_nutrient_protocol(&schedule.nutrient_protocol),
        total_yan_ppm: schedule.total_yan_ppm,
        effective_yan_ppm: schedule.effective_yan_ppm,
        go_ferm_yan_offset_ppm: schedule.go_ferm_yan_offset_ppm,
        fruit_yan_offset_ppm: schedule.fruit_yan_offset_ppm,
        go_ferm_grams: schedule.go_ferm_grams,
        yeast_grams: schedule.yeast_grams,
        rehydration_water_ml: schedule.rehydration_water_ml,
        one_third_break_sg: schedule.one_third_break_sg,
        alert_target_id: schedule.alert_target_id,
        additions,
        max_dosage_capped: schedule.max_dosage_capped,
        created_at: schedule.created_at.into(),
        updated_at: schedule.updated_at.into(),
    }
}

pub async fn get_schedule(
    db: &DatabaseConnection,
    brew_id: Uuid,
) -> Result<Option<NutrientScheduleResponse>, DbErr> {
    let schedule = NutrientSchedule::find()
        .filter(nutrient_schedules::Column::BrewId.eq(brew_id))
        .one(db)
        .await?;

    let Some(schedule) = schedule else {
        return Ok(None);
    };

    let additions = NutrientAddition::find()
        .filter(nutrient_additions::Column::ScheduleId.eq(schedule.id))
        .order_by_asc(nutrient_additions::Column::AdditionNumber)
        .all(db)
        .await?;

    Ok(Some(schedule_model_to_response(schedule, additions)))
}

pub async fn create_schedule(
    db: &DatabaseConnection,
    brew_id: Uuid,
    req: &shared::CreateNutrientSchedule,
) -> Result<NutrientScheduleResponse, DbErr> {
    let calc_req = NutrientCalculateRequest {
        og: req.og,
        batch_size_gallons: req.batch_size_gallons,
        nitrogen_requirement: req.nitrogen_requirement,
        nutrient_protocol: req.nutrient_protocol,
        go_ferm_offset: req.go_ferm_offset,
        fruit_offset_ppm: req.fruit_offset_ppm,
    };
    let result = calculate_nutrient_plan(&calc_req);

    let now = Utc::now().fixed_offset();

    let schedule = nutrient_schedules::ActiveModel {
        id: Set(Uuid::new_v4()),
        brew_id: Set(brew_id),
        batch_size_gallons: Set(req.batch_size_gallons),
        og: Set(req.og),
        nitrogen_requirement: Set(enum_to_string(req.nitrogen_requirement)),
        nutrient_protocol: Set(enum_to_string(req.nutrient_protocol)),
        total_yan_ppm: Set(result.total_yan_ppm),
        go_ferm_yan_offset_ppm: Set(result.go_ferm_yan_offset_ppm),
        fruit_yan_offset_ppm: Set(result.fruit_yan_offset_ppm),
        effective_yan_ppm: Set(result.effective_yan_ppm),
        go_ferm_grams: Set(result.go_ferm_grams),
        yeast_grams: Set(result.yeast_grams),
        rehydration_water_ml: Set(result.rehydration_water_ml),
        one_third_break_sg: Set(result.one_third_break_sg),
        alert_target_id: Set(req.alert_target_id),
        max_dosage_capped: Set(result.max_dosage_capped),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let schedule = schedule.insert(db).await?;

    let mut addition_models = Vec::new();
    for detail in &result.additions {
        let addition = nutrient_additions::ActiveModel {
            id: Set(Uuid::new_v4()),
            schedule_id: Set(schedule.id),
            addition_number: Set(detail.addition_number),
            fermaid_o_grams: Set(detail.fermaid_o_grams),
            fermaid_k_grams: Set(detail.fermaid_k_grams),
            dap_grams: Set(detail.dap_grams),
            trigger_type: Set(enum_to_string(detail.trigger_type)),
            target_hours: Set(detail.target_hours),
            target_gravity: Set(detail.target_gravity),
            notified_at: Set(None),
            created_at: Set(now),
        };
        let model = addition.insert(db).await?;
        addition_models.push(model);
    }

    Ok(schedule_model_to_response(schedule, addition_models))
}

pub async fn delete_schedule(db: &DatabaseConnection, brew_id: Uuid) -> Result<bool, DbErr> {
    let result = NutrientSchedule::delete_many()
        .filter(nutrient_schedules::Column::BrewId.eq(brew_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected > 0)
}

pub async fn find_pending_additions_for_brew(
    db: &DatabaseConnection,
    brew_id: Uuid,
) -> Result<Option<(nutrient_schedules::Model, Vec<nutrient_additions::Model>)>, DbErr> {
    let schedule = NutrientSchedule::find()
        .filter(nutrient_schedules::Column::BrewId.eq(brew_id))
        .one(db)
        .await?;

    let Some(schedule) = schedule else {
        return Ok(None);
    };

    if schedule.alert_target_id.is_none() {
        return Ok(None);
    }

    let additions = NutrientAddition::find()
        .filter(nutrient_additions::Column::ScheduleId.eq(schedule.id))
        .filter(nutrient_additions::Column::NotifiedAt.is_null())
        .order_by_asc(nutrient_additions::Column::AdditionNumber)
        .all(db)
        .await?;

    if additions.is_empty() {
        return Ok(None);
    }

    Ok(Some((schedule, additions)))
}

pub async fn mark_addition_notified(
    db: &DatabaseConnection,
    addition_id: Uuid,
) -> Result<(), DbErr> {
    let now = Utc::now().fixed_offset();
    nutrient_additions::ActiveModel {
        id: Set(addition_id),
        notified_at: Set(Some(now)),
        ..Default::default()
    }
    .update(db)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sg_to_brix_known_values() {
        assert!((sg_to_brix(1.000) - 0.0).abs() < 0.5);
        assert!((sg_to_brix(1.050) - 12.4).abs() < 0.3);
        assert!((sg_to_brix(1.100) - 23.8).abs() < 0.3);
        assert!((sg_to_brix(1.120) - 28.1).abs() < 0.3);
    }

    #[test]
    fn one_third_break_calculation() {
        let break_sg = one_third_sugar_break(1.120);
        assert!((break_sg - 1.080).abs() < 0.001);

        let break_sg = one_third_sugar_break(1.100);
        assert!((break_sg - 1.067).abs() < 0.001);
    }

    #[test]
    fn yeast_and_goferm_calculations() {
        let yeast = calculate_yeast_grams(5.0);
        assert_eq!(yeast, 10.0);

        let (goferm, water) = calculate_go_ferm(yeast);
        assert_eq!(goferm, 12.5); // 1.25 * 2 packets * 5
        assert_eq!(water, 250.0); // 12.5 * 20
    }

    #[test]
    fn fermaid_o_only_5gal_1120_medium() {
        let req = NutrientCalculateRequest {
            og: 1.120,
            batch_size_gallons: 5.0,
            nitrogen_requirement: NitrogenRequirement::Medium,
            nutrient_protocol: NutrientProtocol::FermaidO,
            go_ferm_offset: false,
            fruit_offset_ppm: 0.0,
        };
        let result = calculate_nutrient_plan(&req);

        // ~28.1 Brix * 10 * 0.9 = ~252.9 ppm YAN
        assert!(result.total_yan_ppm > 240.0 && result.total_yan_ppm < 265.0);
        assert_eq!(result.additions.len(), 4);

        // All 4 additions should have Fermaid O > 0
        for a in &result.additions {
            assert!(a.fermaid_o_grams > 0.0);
            assert_eq!(a.fermaid_k_grams, 0.0);
            assert_eq!(a.dap_grams, 0.0);
        }

        // First 3 are time-triggered
        assert_eq!(result.additions[0].trigger_type, NutrientTriggerType::Time);
        assert_eq!(result.additions[0].target_hours, Some(24.0));
        assert_eq!(result.additions[1].target_hours, Some(48.0));
        assert_eq!(result.additions[2].target_hours, Some(72.0));

        // 4th is gravity_or_time
        assert_eq!(
            result.additions[3].trigger_type,
            NutrientTriggerType::GravityOrTime
        );
        assert!(result.additions[3].target_gravity.is_some());
        assert_eq!(result.additions[3].target_hours, Some(168.0));
    }

    #[test]
    fn hybrid_ok_protocol() {
        let req = NutrientCalculateRequest {
            og: 1.120,
            batch_size_gallons: 5.0,
            nitrogen_requirement: NitrogenRequirement::High,
            nutrient_protocol: NutrientProtocol::FermaidOK,
            go_ferm_offset: false,
            fruit_offset_ppm: 0.0,
        };
        let result = calculate_nutrient_plan(&req);

        // High nitrogen with O+K should use both
        for a in &result.additions {
            assert!(a.fermaid_o_grams > 0.0);
            assert!(a.fermaid_k_grams > 0.0);
            assert_eq!(a.dap_grams, 0.0);
        }
    }

    #[test]
    fn hybrid_okdap_no_dap_in_addition_4() {
        let req = NutrientCalculateRequest {
            og: 1.140,
            batch_size_gallons: 5.0,
            nitrogen_requirement: NitrogenRequirement::High,
            nutrient_protocol: NutrientProtocol::FermaidOKDap,
            go_ferm_offset: false,
            fruit_offset_ppm: 0.0,
        };
        let result = calculate_nutrient_plan(&req);

        // Addition 4 should have no DAP
        assert_eq!(result.additions[3].dap_grams, 0.0);

        // Earlier additions may have DAP
        let has_dap = result.additions[..3].iter().any(|a| a.dap_grams > 0.0);
        assert!(has_dap);
    }

    #[test]
    fn go_ferm_offset_reduces_effective_yan() {
        let base = NutrientCalculateRequest {
            og: 1.120,
            batch_size_gallons: 5.0,
            nitrogen_requirement: NitrogenRequirement::Medium,
            nutrient_protocol: NutrientProtocol::FermaidO,
            go_ferm_offset: false,
            fruit_offset_ppm: 0.0,
        };
        let with_offset = NutrientCalculateRequest {
            go_ferm_offset: true,
            ..base.clone()
        };

        let r1 = calculate_nutrient_plan(&base);
        let r2 = calculate_nutrient_plan(&with_offset);

        assert!(r2.effective_yan_ppm < r1.effective_yan_ppm);
        assert!(r2.go_ferm_yan_offset_ppm > 0.0);
    }

    #[test]
    fn fruit_offset_reduces_effective_yan() {
        let base = NutrientCalculateRequest {
            og: 1.100,
            batch_size_gallons: 5.0,
            nitrogen_requirement: NitrogenRequirement::Medium,
            nutrient_protocol: NutrientProtocol::FermaidO,
            go_ferm_offset: false,
            fruit_offset_ppm: 0.0,
        };
        let with_fruit = NutrientCalculateRequest {
            fruit_offset_ppm: 50.0,
            ..base.clone()
        };

        let r1 = calculate_nutrient_plan(&base);
        let r2 = calculate_nutrient_plan(&with_fruit);

        assert!((r1.effective_yan_ppm - r2.effective_yan_ppm - 50.0).abs() < 1.0);
    }
}
