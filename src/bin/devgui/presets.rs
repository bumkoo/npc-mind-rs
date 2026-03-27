//! 4인 프리셋 캐릭터 (tests/common/mod.rs 기반)

use crate::state::{GuiState, PresetChoice};

pub fn apply_preset(state: &mut GuiState, preset: PresetChoice) {
    state.selected_preset = preset;
    match preset {
        PresetChoice::Custom => {}
        PresetChoice::무백 => apply_무백(state),
        PresetChoice::교룡 => apply_교룡(state),
        PresetChoice::수련 => apply_수련(state),
        PresetChoice::소호 => apply_소호(state),
    }
}

fn apply_무백(s: &mut GuiState) {
    s.npc_id = "mu_baek".into();
    s.npc_name = "무백".into();
    s.npc_description = "정의로운 검객. 의리와 절제를 중시한다.".into();
    // H: 정직-겸손성
    s.sincerity = 0.8;
    s.fairness = 0.7;
    s.greed_avoidance = 0.6;
    s.modesty = 0.5;
    // E: 정서성
    s.fearfulness = -0.6;
    s.anxiety = -0.4;
    s.dependence = -0.7;
    s.sentimentality = 0.2;
    // X: 외향성 (미설정 → 0)
    s.social_self_esteem = 0.0;
    s.social_boldness = 0.0;
    s.sociability = 0.0;
    s.liveliness = 0.0;
    // A: 원만성
    s.forgiveness = 0.6;
    s.gentleness = 0.7;
    s.flexibility = 0.2;
    s.patience = 0.8;
    // C: 성실성
    s.organization = 0.4;
    s.diligence = 0.8;
    s.perfectionism = 0.6;
    s.prudence = 0.7;
    // O: 경험개방성 (미설정 → 0)
    s.aesthetic_appreciation = 0.0;
    s.inquisitiveness = 0.0;
    s.creativity = 0.0;
    s.unconventionality = 0.0;
}

fn apply_교룡(s: &mut GuiState) {
    s.npc_id = "gyo_ryong".into();
    s.npc_name = "교룡".into();
    s.npc_description = "야심적인 여검객. 자유를 갈망하며 관습을 거부한다.".into();
    // H
    s.sincerity = -0.4;
    s.fairness = -0.5;
    s.greed_avoidance = -0.6;
    s.modesty = -0.7;
    // E (미설정 → 0)
    s.fearfulness = 0.0;
    s.anxiety = 0.0;
    s.dependence = 0.0;
    s.sentimentality = 0.0;
    // X
    s.social_self_esteem = 0.7;
    s.social_boldness = 0.8;
    s.sociability = 0.0;
    s.liveliness = 0.6;
    // A
    s.forgiveness = -0.6;
    s.gentleness = -0.5;
    s.flexibility = -0.4;
    s.patience = -0.7;
    // C
    s.organization = -0.5;
    s.diligence = -0.3;
    s.perfectionism = -0.4;
    s.prudence = -0.6;
    // O
    s.aesthetic_appreciation = 0.6;
    s.inquisitiveness = 0.8;
    s.creativity = 0.7;
    s.unconventionality = 0.9;
}

fn apply_수련(s: &mut GuiState) {
    s.npc_id = "shu_lien".into();
    s.npc_name = "수련".into();
    s.npc_description = "절제의 여검객. 의무와 명예를 삶의 기둥으로 삼는다.".into();
    // H
    s.sincerity = 0.8;
    s.fairness = 0.9;
    s.greed_avoidance = 0.7;
    s.modesty = 0.6;
    // E
    s.fearfulness = -0.3;
    s.anxiety = 0.2;
    s.dependence = -0.5;
    s.sentimentality = 0.7;
    // X (미설정 → 0)
    s.social_self_esteem = 0.0;
    s.social_boldness = 0.0;
    s.sociability = 0.0;
    s.liveliness = 0.0;
    // A
    s.forgiveness = 0.5;
    s.gentleness = 0.6;
    s.flexibility = 0.3;
    s.patience = 0.9;
    // C
    s.organization = 0.6;
    s.diligence = 0.8;
    s.perfectionism = 0.5;
    s.prudence = 0.9;
    // O (미설정 → 0)
    s.aesthetic_appreciation = 0.0;
    s.inquisitiveness = 0.0;
    s.creativity = 0.0;
    s.unconventionality = 0.0;
}

fn apply_소호(s: &mut GuiState) {
    s.npc_id = "so_ho".into();
    s.npc_name = "소호".into();
    s.npc_description = "자유로운 낭인. 직감과 행동으로 세상을 살아간다.".into();
    // H
    s.sincerity = 0.1;
    s.fairness = 0.5;
    s.greed_avoidance = 0.3;
    s.modesty = -0.3;
    // E
    s.fearfulness = -0.7;
    s.anxiety = -0.5;
    s.dependence = -0.8;
    s.sentimentality = 0.4;
    // X
    s.social_self_esteem = 0.6;
    s.social_boldness = 0.7;
    s.sociability = 0.5;
    s.liveliness = 0.4;
    // A
    s.forgiveness = 0.1;
    s.gentleness = -0.4;
    s.flexibility = 0.3;
    s.patience = -0.3;
    // C
    s.organization = -0.6;
    s.diligence = 0.2;
    s.perfectionism = -0.4;
    s.prudence = -0.5;
    // O (미설정 → 0)
    s.aesthetic_appreciation = 0.0;
    s.inquisitiveness = 0.0;
    s.creativity = 0.0;
    s.unconventionality = 0.0;
}
