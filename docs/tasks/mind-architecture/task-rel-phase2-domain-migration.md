# Phase 2 ??Relationship ?꾨찓??留덉씠洹몃젅?댁뀡 (4異?+ BondKind/BondStatus/Partnership + type)

**Status**: `ready` ??**Stage 0 醫낃껐 (2026-05-13), v1.0 spec freeze**. Stage 1 吏꾩엯 ?湲?
**Owner**: Bekay + Claude
**Parent**: `docs/tasks/mind-architecture/00-roadmap.md` 짠5 Phase 2 (遺꾪븷 ??蹂?phase)
**Sibling**: `task-rel-phase2.5-channel1.md` (蹂꾨룄 phase濡?遺꾨━, 異뷀썑 ?묒꽦)
**Prerequisite**: Phase 1/1.5/1.6 ???꾨즺 (`phase1-checkpoint-report.md` v0.2)

---

## 짠1 Scope

**?ы븿**:
- `Relationship` ?꾨찓?? 3異?(closeness/trust/power, 짹1.0) ??4異?(trust/affinity/respect/wariness, 짹100)
- `BondKind` 11醫?enum ?꾩엯 (relationships.md v0.7 짠3.1)
- `BondStatus` 5醫?enum ?꾩엯 (짠3.5)
- `Partnership` 4醫?enum ?꾩엯 (짠3.6)
- `type: String` + `type_history: Vec<TypeChange>` ?먯쑀 ?띿뒪???꾨뱶 (짠2)
- ?쒕굹由ъ삤 JSON schema v0.6 ??v0.7 媛깆떊 (Relationship ?꾨뱶 ?쒖젙)
- 寃利??쒕굹由ъ삤 ~45 ?섏뼱 ?곗씠??留덉씠洹몃젅?댁뀡
- 1095+ tests ?뚭? 媛깆떊

**鍮꾪룷??* (Phase 2.3 蹂꾨룄 ???좎꽕):
- appraise 濡쒖쭅 ?뺣퉬 (?쒕??덉씠??湲곕컲 ?쒕떇)
- ?꾨씫 OCC ?앸퀎 + ?먮룞 蹂댁셿/寃쎄퀬 (I1)
- Compound 媛먯젙 ?앸퀎 ?뺤옣
- `RelationshipModifiers` ?뺣???(4異??섍꼍?먯꽌 ?꾨씫 modifier 寃利?
- HEXACO 蹂댁젙???뺣웾 誘몄꽭議곗젙
- base_delta 48? ?쒕굹由ъ삤 湲곕컲 誘몄꽭議곗젙

**鍮꾪룷??* (Phase 2.5 蹂꾨룄):
- Channel 1 Declarative ?쒖꽦??(`declarative_events` / `partnership_event` placeholder??*enum/?꾨뱶 ?뺤쓽留? 諛뺢퀬 LLM emit/寃利??곸슜? Phase 2.5)
- ?ы쉶???쇨???寃利?5 移댄뀒怨좊━ (A~E)
- 4-tier ?곸슜 紐⑤뱶
- **??`axis_modulation` (LLM 誘몄꽭議곗젙 3吏?좊떎)** ??Reflection 寃곌낵 schema ?뺤옣

**鍮꾪룷??* (Phase 3 蹂꾨룄):
- Channel 2 Temporal (BondKind ?쒓컙 寃뚯씠???먮룞 吏꾩엯)
- Channel 3 External (?멸퀎 ?ш굔 overlay)
- `RecollectionAction` 5醫?異붾え ?됰룞
- `ActionTriggerEvaluator` 5-dim feasibility

---

## 짠2 Inputs

| 臾몄꽌 | ?낅젰 踰꾩쟾 | 異쒕젰 踰꾩쟾 |
|---|---|---|
| `relationships.md` | v0.7 (?꾪뻾, 蹂寃??놁쓬) | v0.7 (李몄“留? |
| `_schema.md` | v0.6 (?꾪뻾) | **v0.7** (Phase 2媛 媛깆떊) |
| `00-roadmap.md` | v0.5 (Phase 1.5/1.6 諛섏쁺) | v0.6 (Phase 2 吏꾩엯 ?쒓린) |

**李몄“ baseline**:
- Phase 1 spec stages (6 stage ?⑦꽩)
- Phase 1 checkpoint report v0.2 (?꾩젣 議곌굔 짠7)

---

## 짠3 Findings (Stage 0)

Phase 1 Stage 0 ?⑦꽩(F1~F12)??蹂?phase???곸슜. A 移댄뀒怨좊━ 5媛???*?꾩옱 肄붾뱶 ?ъ떎 議곗궗*. 媛???ぉ? 蹂寃???*?꾩옱 ?곹깭*瑜?grep?쇰줈 ?뺤젙?섍퀬, Phase 2 蹂寃?硫댁쟻???곗젙?쒕떎.


### A1 ??`Relationship` ?꾨찓??+ `Score` ???+ ?ъ슜泥?
**?꾩옱 ?꾨찓??* (`src/domain/relationship.rs`):
```rust
pub struct Relationship {
    owner: NpcId, target: NpcId,
    closeness: Score, trust: Score, power: Score,
}
pub struct RelationshipModifiers {
    pub closeness_modifier: f32, pub closeness_squared: f32,
    pub closeness_abs: f32, pub trust_modifier: f32,
}
```

**`Score` ???*: `src/domain/personality.rs` ?뺤쓽 ??*HEXACO 24 facet怨?怨듭쑀*. `Score(f32)` 踰붿쐞 짹1.0. Phase 2媛 짹100 踰붿쐞濡?媛硫?*怨듭쑀 ???遺꾨━ ?꾩슂*.

**?ъ슜泥?134 留ㅼ튂** (wuxia-core 4留ㅼ튂???쒖쇅, ?먭린 ?덉젙):

| ?곸뿭 | 留ㅼ튂 | 梨낆엫 | Phase 2 蹂寃?|
|---|---|---|---|
| `domain/relationship.rs` | 13 | ?뺤쓽 + ?대? ?뚯뒪??| ?ъ옉??蹂몄껜 |
| `Relationship::neutral()` ?몄텧 | 16 | Policy fallback (媛?臾닿?) | ?먮룞 ?≪닔 (?ы띁 ?쒓렇?덉쿂 蹂댁〈) |
| `Relationship::new` / `RelationshipBuilder` | 4 留ㅼ튂 | ?쒕굹由ъ삤 JSON 吏꾩엯??1 + UI CRUD 1 + ?뚯뒪??2 | ?쒓렇?덉쿂 蹂寃?|
| `.modifiers()` ?듦낵 | 5怨?| emotion/stimulus/scene policy + memory_repository | **蹂寃?0** (?명꽣?섏씠??蹂댁〈, ?대? 留ㅽ븨留??ъ옉?? |
| `.closeness()`/`.trust()`/`.power()` 吏곸젒 ?몄텧 | 6怨?| snapshot + orchestrator + relationship_policy + memory_repository + telling_ingestion + domain_sync | 紐낆떆 蹂寃?(4異?硫붿꽌?쒕챸) |
| ?뚯뒪???몄텧 | ~100 | Builder ?⑦꽩 + neutral ?몄텧 | **?뚭? 硫댁쟻 ??* ???먮룞 留덉씠洹몃젅?댁뀡 ?ㅽ겕由쏀듃 寃??|

**?듭떖 諛쒓껄**:
1. `modifiers()` 異붿긽??寃쎄퀎媛 *OCC 媛먯젙 ?붿쭊 5怨녹쓣 ?먮룞 ?≪닔*. ?명꽣?섏씠??硫댁쟻 ?묒쓬.
2. ?쒕굹由ъ삤 JSON?붾룄硫붿씤 吏꾩엯?먯? `memory_repository.rs:195` ??1怨? 3異뺚넂4異?蹂??猷?1怨?吏묒쨷 媛??
3. ?뚭? 硫댁쟻??蹂몄쭏? *?뚯뒪??~100 ?몄텧??Builder ?쒓렇?덉쿂 蹂寃?. ?먮룞 蹂???ㅽ겕由쏀듃媛 鍮꾩슜 ?덇컧 媛????Phase 2 Stage 1?먯꽌 寃??
4. `Score` ??낆씠 HEXACO? 怨듭쑀??*遺꾨━ 寃곗젙* ?꾩슂 (??B-D1).


### A2 ??OCC ??3異?留ㅽ븨 ?⑥닔 + 媛깆떊 梨낆엫??
**?꾩옱 OCC ??axes 留ㅽ븨**:
```rust
// Relationship::after_dialogue (?⑥씪 ?⑥닔, closeness 1異뺣쭔 ?먮룞)
pub fn after_dialogue(&self, final_state: &EmotionState, significance: f32) -> Self {
    self.with_updated_closeness(final_state.overall_valence(), significance)
    // trust: 蹂寃??놁쓬 (?ν썑 LLM ?됯?)
    // power: 蹂寃??놁쓬 (?쒖궗 ?대깽?몃쭔)
}
```

**怨듭떇**: `new_closeness = clamp(old + valence 횞 0.05 횞 (1 + sig 횞 3.0), 짹1.0)`

**媛깆떊 梨낆엫??*: `application/command/policies/relationship_policy.rs` ???몄텧 ?꾩튂 *2 怨?以묐났* (`handle_dialogue_end` + `handle_relationship_update_with_cause`). Phase 2 ?ъ옉????helper 異붿텧 沅뚯옣.

**`outer_loop_entry()` 寃뚯씠??* ??Phase 2/3 吏꾩엯 ?먮━ *?덉빟??:
```rust
match reflection {
    Some(refl) => {
        refl.significance_score >= 0.3
            || !refl.is_chitchat
            || !refl.declarative_events.is_empty()     // ??Phase 2.5 ?쒖꽦???꾩튂
            || refl.partnership_event.is_some()        // ??Phase 2.5 ?쒖꽦???꾩튂
        // || temporal_signals (Phase 3a)
        // || external_events (Phase 3b)
    }
    None => legacy_significance.is_some(),
}
```
?꾩옱??declarative_events/partnership_event媛 ??긽 鍮?None?대씪 議곌굔???묐룞 ???? Phase 2媛 *enum/?꾨뱶 ?뺤쓽*留?諛뺤쑝硫?Phase 2.5?먯꽌 *吏꾩쭨 ?곗씠??媛 ?섎윭?ㅼ뼱? 寃뚯씠??利됱떆 ?숈옉.

**`RelationshipUpdatedPayload`** ???몃? schema ?곹뼢:
```rust
// ?꾩옱: 6 ?꾨뱶 (3異?횞 2 = before/after)
closeness_before, trust_before, power_before,
closeness_after,  trust_after,  power_after,
// Phase 2 ?? 8 ?꾨뱶 (4異?횞 2) + cause 洹몃?濡?trust_before, affinity_before, respect_before, wariness_before,
trust_after,  affinity_after,  respect_after,  wariness_after,
// power ?먭린 (B-D4 ?뺤젙)
```

**?몃? 援щ룆??*: `relationship_memory_handler`, SSE bridge (`event_bridge`), Mind Studio frontend ??schema 蹂寃??곹뼢.

**?듭떖 諛쒓껄**:
1. ?꾩옱 *?먮룞 媛깆떊? closeness 1異뺣퓧*. Phase 2媛 *4異??먮룞 媛깆떊 猷????대뵒源뚯? 諛뺤쓣吏媛 ?묒뾽 ?ш린 寃곗젙 (??B-D5, B-D6).
2. `RelationshipUpdatedPayload` 6?? ?꾨뱶 蹂寃???frontend `domain_sync.rs` + SSE event_bridge 留ㅽ븨 媛깆떊.
3. `outer_loop_entry()` 寃뚯씠?몃뒗 *Phase 2 蹂寃?0*. Phase 2.5?먯꽌 ?곗씠?곕쭔 ?먮쫫.


### A3 ??`RelationshipChangeCause` enum 5 variants + ?ъ슜泥?
**?뺤쓽** (`src/domain/event.rs:138-150`):
```rust
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RelationshipChangeCause {
    SceneInteraction { scene_id: SceneId },
    InformationTold { origin_chain: Vec<String> },
    WorldEventOverlay { topic: Option<String> },
    Rumor { rumor_id: String },
    #[default] Unspecified,
}
```

**?ъ슜泥?22 留ㅼ튂 遺꾨쪟**:

| ?꾩튂 | 醫낅쪟 | ?⑦꽩 |
|---|---|---|
| `relationship_policy.rs:88, 117, 241` | **emit (3 ?꾩튂)** | `SceneInteraction` (BeatTransitioned) + `Unspecified` 횞 2 |
| `event.rs:901, 1094` | tests | ?⑥쐞 ?뚯뒪??|
| `projection_handlers.rs:311, 336, 347` | tests | RelationshipProjection ?뚭? |
| `relationship_memory_handler.rs:87~120` | **consume** | 5 variant 遺꾧린 ??MemorySource/topic/content 留ㅽ븨 |
| `relationship_memory_handler.rs:188-191` | **consume** | origin_chain 異붿텧 (InformationTold/Rumor) |
| `relationship_memory_handler.rs:376~531` | tests | 遺꾧린蹂??뚭? ?뚯뒪??|

**?듭떖 諛쒓껄**:
1. **emit? SceneInteraction + Unspecified 2 variant留??ㅼ젣 ?묐룞**. `InformationTold`/`WorldEventOverlay`/`Rumor`??enum ?뺤쓽留??덇퀬 *?대뵒?쒕룄 emit?섏? ?딆쓬* (Step C/D ?ㅺ퀎?먭? 誘몃━ 諛뺤? forward-compat ?먮━).
2. **consume 痢??꾩쟾 援ы쁽**. ??variant 異붽? ??`relationship_memory_handler` 遺꾧린??異붽?留??섎㈃ ??(湲곗〈 ?⑦꽩 mirror).
3. **Phase 2 蹂몄껜 ?곹뼢 0**. cause enum怨?*蹂寃?硫댁쟻*? 吏곴탳. Phase 2.5?먯꽌 declarative_events ?쒖꽦??????variant ?꾨낫 (`BondKindFormed` ?? ??*Phase 2 寃곗젙 ?ы빆 ?꾨떂*.
4. `RelationshipUpdatedPayload`??`cause` ?꾨뱶??洹몃?濡?(4異?蹂寃쎄낵 臾닿?).


### A4 ???쒕굹由ъ삤 JSON 3異??곗씠??遺꾪룷

**洹쒕え**: 267 留ㅼ튂 (closeness/trust/power 媛곴컖) = ~89 Relationship instance = **~45 ?섏뼱** (?遺遺?a?봟 ?띾갑??. ?꾩튂 ???쒕굹由ъ삤 JSON ??+ `session_*_result.json` ?뚯뒪??寃곌낵 ?뚯씪.

**蹂꾨룄 `wuxia_world/assets/relationships/` ?붾젆?좊━ 鍮꾩뼱 ?덉쓬** ??愿怨꾨뒗 ?쒕굹由ъ삤 JSON ?덉뿉 吏곸젒 諛뺥옒. ?ν썑 Phase 2.x 遺꾨━ ?묒뾽 ?꾨낫吏留?*Phase 2 踰붿쐞 ?꾨떂*.

**寃利?耳?댁뒪 2媛?*:

| ?섏뼱 | closeness | trust | power | ?섎? |
|---|---|---|---|---|
| ?꾩땐 ???↔껴 | +0.4 | +0.5 | 0.0 | "??移쒓뎄" ?몄떇 (諛곗떊 ?? |
| ?↔껴 ???꾩땐 | -0.2 | -0.3 | -0.4 | "?쒓굅 ??? (諛곗떊 ?섎룄) |
| ?섎젴 ??臾대갚 | +0.7 | +0.8 | -0.1 | ?섑삎???덉젣???щえ |
| 臾대갚 ???섎젴 | +0.7 | +0.8 | +0.1 | ?숈씪 |

**3異???4異?type 蹂??猷?*:

| 3異?| 4異????| 蹂??媛?μ꽦 |
|---|---|---|
| `trust` 짹1.0 | `trust` 짹100 | **?먮룞** ???섎? ?숈씪 蹂댁〈, `횞 100` |
| `closeness` 짹1.0 | `affinity` 짹100 | **諛섏옄??* ???섎? 遺遺?寃뱀묠 (closeness ??affinity), `횞 100` ???붿옄?대꼫 寃??|
| (?놁쓬) | `respect` 짹100 | **?섎룞** ???붿옄?대꼫 蹂댁땐 (B-D10) |
| (?놁쓬) | `wariness` 0~100 | **?섎룞** ???붿옄?대꼫 蹂댁땐 (B-D10) |
| `power` 짹1.0 | (?먭린, type ?≪닔) | **?섎룞** ???붿옄?대꼫媛 `type` ??以??묒꽦 (B-D4 ?뺤젙) |

**?듭떖 諛쒓껄**:
1. **?먮룞 蹂??媛??鍮꾩쑉 ~50%** (trust + closeness留?. respect/wariness/type? *?붿옄?대꼫 ???묒뾽 ?꾩닔*.
2. **`session_*_result.json` 寃곌낵 ?뚯씪??3異?諛뺥옒** ??Phase 2 ?꾨즺 ???쇨큵 ?먭린 + ?ъ떎??沅뚯옣 (B-D9).
3. **?곗씠??留덉씠洹몃젅?댁뀡 ?뚰겕?뚮줈?곕뒗 蹂꾨룄 Stage**媛 ??媛?μ꽦 ????~45 ?섏뼱 횞 4異?+ type = ~225 媛??붿옄?대꼫 寃??
4. `power` ?곗씠???쒖슜?꾧? *誘몃??덉쓬* (?遺遺?짹0.0~짹0.4 踰붿쐞, ActingGuide ?쇰꺼???붿뒪?뚮젅?? ??B-D4 ?뺤젙 洹쇨굅.

### A5 ??`MAX_EVENTS_PER_COMMAND` ?ъ궛??
**?꾩옱 22** (`dispatcher.rs:35-41`): Phase 1 worst-case 8~9 + ?덉쟾 留덉쭊 2.5諛?

**Phase 2 蹂몄껜 ?곹뼢**: 蹂寃?0.
- 3異???4異? payload ?꾨뱶 ?ш린留?6 ??8, ?대깽??*?? ?곹뼢 ?놁쓬.
- BondKind/BondStatus/Partnership/type/type_history ?꾩엯: Relationship ?꾨뱶 異붽?, ?대깽??異붽? ?꾨떂.
- `power` ?먭린: payload ?꾨뱶 媛먯냼.

**Phase 2.5 worst-case ?덉긽** (李멸퀬):
```
DialogueEndRequested 1 + DialogueReflected 1 + RelationshipUpdated (4異? 1
+ declarative_events fan-out N (?꾩떎 ?곹븳 ??5)
+ ?ы쉶???쇨???寃利?reject 理쒕? 5 (5 移댄뀒怨좊━ A~E)
+ EmotionCleared 1 + SceneEnded 1 + Inline projection 3
= 12 + N ??17
```

**寃곕줎**: Phase 2 / Phase 2.5 紐⑤몢 22 ?덉쟾. **?몄긽 遺덊븘??*.

---

## 짠3 醫낇빀 ??Phase 2 ?곹뼢 硫댁쟻

| 蹂寃?硫댁쟻 | ?ш린 | 鍮꾧퀬 |
|---|---|---|
| ?꾨찓??蹂몄껜 (relationship.rs + Score) | ??| ?ъ옉??|
| RelationshipPolicy 留ㅽ븨 | 以?| 2 ?꾩튂 helper 異붿텧 + ?ъ옉??|
| Payload schema (6?? ?꾨뱶) | ?묒쓬 | ?꾨뱶 ?대쫫 蹂寃?+ 異붽? |
| cause enum | **0** | 吏곴탳 |
| consume 痢?(memory_handler) | **0** | ?명꽣?섏씠??蹂댁〈 |
| ?쒕굹由ъ삤 JSON 吏꾩엯??| 以?| 蹂??猷?1怨?吏묒쨷 |
| Mind Studio CRUD | ?묒쓬 | ?⑥닚 ?꾨뱶 蹂寃?|
| ?뚯뒪???뚭? | ??| ~100 ?몄텧 ?먮룞 留덉씠洹몃젅?댁뀡 寃??|
| ?쒕굹由ъ삤 ?곗씠??(?붿옄?대꼫 ?? | ??| ~45 ?섏뼱 횞 5?꾨뱶 = ~225 媛?|
| `MAX_EVENTS_PER_COMMAND` | **0** | 22 ?덉쟾 |

---

## 짠3.6 ?쒕??덉씠??寃利?(S1~S4) ??Stage 0 異붽? 諛쒓껄

B 移댄뀒怨좊━ 寃곗젙 ??ぉ (B-D6, B-D12, B-D13, B-D14)??洹쇨굅 ?뺣낫 + Phase 2 ?듬줈 A ?붿옄??寃利앹쓣 ?꾪빐 臾댄삊吏 ?쒕굹由ъ삤 4 耳?댁뒪??v0.7 짠4 ?붿옄???곸슜.

### S1 ???꾩땐 ???몄???(Gratitude ?⑥닚)

- ?붿옄?대꼫 諛뺣뒗 Beat focus: `event(desirability +0.7) + action(agent_id="lu_zhishen", praiseworthiness +0.8)`
- appraise ?먮룞 ?앹꽦: Joy + **Admiration** + **Gratitude** (compound)
- 寃곌낵: trust +13, affinity +6, respect 0 ??*Admiration ?먮룞 ?앹꽦*?쇰줈 **respect 0 臾몄젣 ?먯껜媛 諛쒖깮 ????*
- ??base_delta ??+ ActionFocus 諛뺢린濡?*諛⑺뼢???먯뿰*

### S2 ???꾩땐 ???↔껴 (?곗떊臾?????ш굔)

- ?붿옄?대꼫 諛뺣뒗 focus: `event(desirability -0.95, prospect=FearConfirmed) + action(agent_id="lu_qian", praiseworthiness -0.95) + object(appealingness -0.95)`
- appraise ?먮룞 ?앹꽦: Distress + FearsConfirmed + **Reproach** + **Hate** + **Anger** (compound)
- 寃곌낵 (Anger + Hate + Reproach ?⑹궛 + HEXACO 횞1.2 + axis_modulation "high"): trust -49, affinity -43, respect -30, wariness +53
- ?쒕굹由ъ삤 諛뺥엺 *???곹깭* (trust -30, affinity -20, wariness 留ㅼ슦 ?믪쓬)? 鍮꾧탳: **affinity/wariness??align**, trust留?*異붽? -31 蹂?? ?꾩슂
- ??Phase 2 ?듬줈 A (?먯쭊?? + Phase 2.5 ?듬줈 B (declarative_events ???꾩빟) **遺꾨떞 ?묐룞** ?낆쬆

### S3 ???섎젴 ???κ탳猷?(?곸땐 媛먯젙)

- ?붿옄?대꼫 諛뺣뒗 focus: `event(desirability_for_self -0.4, desirability_for_other=?κ탳猷?-0.7) + action(agent_id="yu_xiaolong", praiseworthiness -0.6)`
- appraise ?먮룞 ?앹꽦: Distress + **Pity** + **Reproach** + **Anger** (compound)
- 寃곌낵 (Pity + Reproach + Anger ?⑹궛 + HEXACO 횞0.56 + axis_modulation ?쇳빀): trust -9, affinity -3, respect -15, wariness +14
- ??**affinity 嫄곗쓽 蹂???놁쓬** (Pity +5? Anger/Reproach -10 ?곸뇙) ??*?곸땐 媛먯젙???뺥솗???쒕??덉씠?? (??몄옣猷??섎젴??*?κ탳猷≪쓣 鍮꾨궃?섎㈃?쒕룄 ?덊?源뚯썙?? ?⑦꽩 ?ъ갑)
- ??base_delta ?쒓? *?곸땐 媛먯젙* 洹좏삎???먮룞 泥섎━

### S4 ???꾩땐 ??怨좉뎄 (留λ씫 ?섏〈, ???쒓퀎 ?쒗뿕)

- *媛숈? ?명삎 ?ш굔* (怨좉뎄???먮퉬)??NPC ?쒓컖???곕씪 *?꾪? ?ㅻⅨ 媛먯젙*. base_delta ?쒕뒗 *留λ씫 臾댁떆* ???쒓퀎?
- 寃利?寃곌낵: **?쒓퀎 ?꾨떂**. *3 layer separation*???≪닔:
  - Layer 1 (Beat focus ?ㅺ퀎): ?붿옄?대꼫媛 *NPC ?쒓컖?쇰줈* event/action/object 諛뺤쓬 ???κ탳猷??먮퉬瑜?*?꾪삊*?쇰줈 諛뺤쑝硫?`desirability_for_self -0.3`????  - Layer 1.5 (Relationship modifiers): 湲곗〈 ?꾩땐?믨퀬援??곷? 愿怨꾩쓽 `trust_modifier`/`hostility_modifier`媛 *媛먯젙 媛뺣룄 ?먮룞 議곗젙*
  - Layer 2/3 (appraise + base_delta): *?낅젰??諛뺥엺 ?? 寃곗젙濡좎쟻 留ㅽ븨
- ?붿옄?대꼫 ?ㅼ닔 (?듭뀡 A 諛뺤쓬)?먮룄 *Relationship modifiers媛 ?먯뿰 蹂댁젙* ???꾪뙥???쏀솕
- ??base_delta ?쒖쓽 *留λ씫 臾댁떆*??*吏꾩쭨 ?쒓퀎 ?꾨떂*. Layer 3 only??梨낆엫

### ?쒕??덉씠??寃利?醫낇빀

| 耳?댁뒪 | 寃利?寃곌낵 |
|---|---|
| S1 (Gratitude ?⑥닚) | ??Admiration ?먮룞 ?앸퀎 |
| S2 (?곗떊臾?????ш굔) | ??Phase 2/2.5 遺꾨떞 ?묐룞 |
| S3 (?곸땐 媛먯젙) | ??affinity ?뺤껜 (?먮룞 洹좏삎) |
| S4 (留λ씫 ?섏〈) | ??3 layer separation |

??**base_delta 48? ??+ HEXACO 蹂댁젙??+ axis_modulation 寃고빀??Phase 2 ?듬줈 A??*?⑸떦???묐룞*???낆쬆**. v0.7 짠4 ?붿옄??洹몃?濡?諛뺣뒗 寃??곸젅.

### ???듭떖 諛쒓껄 ??appraise ?낅젰 ?섏〈??
S1~S4?먯꽌 *怨듯넻 ?⑦꽩*: **appraise??*?붿옄?대꼫 諛뺤? Beat focus ?꾩쟾?깆뿉 ?섏〈***. ActionFocus ??諛뺤쑝硫?Admiration/Reproach ?먮룞 ?앹꽦 0. EventFocus ??諛뺤쑝硫?Joy/Distress/HappyFor/Pity ???먮룞 ?앹꽦 0.

- ?쒕굹由ъ삤 ?붿옄?대꼫媛 *12+ OCC ?꾩쟾 ?앸퀎*? 遺????- *?곸떇??異붾줎* (?? "?꾩?諛쏆쓬 ??移?갔???됱쐞???덉쓬") ?먮룞??????
??**Phase 2.3 (appraise ?뺣퉬) ?좎꽕 寃곗젙**. Phase 2 蹂몄껜? Phase 2.5 ?ъ씠??*?뉗? phase*濡?遺꾨━. ?쒕??덉씠???쒕굹由ъ삤 set 怨듭떇??+ ?꾨씫 OCC 寃利?寃쎄퀬 (I1) + Compound ?앸퀎 ?뺤옣 + modifiers ?뺣???+ HEXACO/base_delta 誘몄꽭議곗젙.

---

## 짠4 Decisions (Stage 0 ????Phase 2 蹂몄껜 寃곗젙 ?꾨즺)

B 移댄뀒怨좊━ 14媛???ぉ. **Phase 2 蹂몄껜 12媛??꾨? ?뺤젙 ??*. B-D7/B-D11? Phase 2.5 ?쒖젏 寃곗젙.

| # | ??ぉ | ?곹깭 |
|---|---|---|
| B-D1 | `Score` ????대챸 (HEXACO? 遺꾨━/?좎?/?쇰컲?? | ??**?뺤젙 ??A (遺꾨━) + 2 ???*: HEXACO `Score(f32)` 짹1.0 洹몃?濡?/ Relationship 4異??좎꽕 `AxisScore(f32)` 짹100 (trust/affinity/respect) + `WarinessScore(f32)` 0~100 蹂???? wariness ?뚯닔 諛뺣뒗 ?ㅼ닔 *而댄뙆???쒖젏 李⑤떒*. HEXACO ?ъ슜泥?蹂寃?0. |
| B-D2 | 짹1.0 ??짹100 蹂??諛⑹떇 (?대? float? ?뺤닔?) | ??**?뺤젙 ??f32 ?대? ?쒗쁽 + JSON ?뺤닔 round 異쒕젰**. v0.7 짠4.1 肄붾뱶 洹몃?濡??명솚. base_delta 횞 intensity 횞 HEXACO 怨깆뀍 *?뺣????좎?* (?? -25 횞 0.95 횞 1.2 = -28.5 ?뺥솗). ?쒕굹由ъ삤 JSON??`"trust": 75` ?뺤닔 ?쒓린, ?대? 75.0. |
| B-D3 | closeness ??affinity 蹂??猷?(?섎? ?ㅻ쫫) | ??**?뺤젙 ??(c) ?쇳빀**: ?먮룞 蹂??baseline `affinity = closeness 횞 100` + ?붿옄?대꼫 ?좏깮??議곗젙. closeness("?④퍡 ?덉쓣 ??移쒓렐媛?) ??affinity("?쇱옄????洹몃━?") ?섎? 遺遺?寃뱀묠. ?먮룞 蹂?섏쓣 *珥덇린媛??쇰줈 諛뺢퀬 narrative 寃利?Phase 2.3 ?쒕??덉씠??以??댁깋??耳?댁뒪留?議곗젙. ?먯닔 耳?댁뒪(?꾩땐?믨퀬援??? ?뚯닔 蹂댁땐? B-D10 (珥덇린媛?猷??먯꽌 ?≪닔. |
| B-D4 | `power` ?대챸 | ??**?뺤젙 ???먭린, `type` ?먯쑀 ?띿뒪???≪닔** |
| B-D5 | 4異?媛곴컖 蹂꾨룄 留ㅽ븨 ?⑥닔? ?⑥씪 ?⑥닔? | ??**?뺤젙 ???⑥씪 ?⑥닔 (v0.7 짠4.1 洹몃?濡?** `update_axes_from_emotion(rel, emotion, intensity, hexaco)`. ??OCC 媛먯젙 ?낅젰??4異??숈떆 媛깆떊. `base_delta(emotion) -> AxisDelta` 48? lookup + `hexaco_modifier(emotion, hexaco) -> AxisModifier` + clamp. 4異뺣퀎 遺꾨━ ?⑥닔??*肄붾뱶 以묐났 + 鍮꾪슚???대씪 鍮꾩콈?? 援ъ껜 援ы쁽 (lookup ?먮즺援ъ“ / 硫붿꽌??vs ?먯쑀 ?⑥닔)? Stage 1. |
| B-D6 | 4異??먮룞 媛깆떊 猷?+ ?쒖젏 + 媛?쒕젅??| ??**?뺤젙 ??T1 (?????batch) + D6-a (v0.7 짠4.1~4.3 洹몃?濡? base_delta 48? + HEXACO 蹂댁젙??+ BondStatus 李⑤떒 + clamp) + axis_modulation 3吏?좊떎 (low/default/high ??짹5/0/+5, reflection LLM 異쒕젰 ?꾨뱶 ?좎꽕, 異붽? LLM ?몄텧 0)** |
| B-D7 | (Phase 2.5) ??cause variant 紐낅챸 | Phase 2.5 |
| B-D8 | ?쒕굹由ъ삤 ?곗씠??留덉씠洹몃젅?댁뀡 ??諛섏옄???ㅽ겕由쏀듃 + ?붿옄?대꼫 ?섎룞? | ??**?뺤젙 ??W3+ (?먮룞 + Claude AI 異붾줎 + ?붿옄?대꼫 寃??**. 6 ?④퀎 ?뚰겕?뚮줈?? ??Rust binary 留덉씠洹몃젅?댁뀡 ?꾧뎄 ?묒꽦 (`tools/migrate_relationships/`) ????Claude AI 異붾줎?쇰줈 BondKind/type 梨꾩? + ?붿옄?대꼫 寃??????Rust binary ?ㅽ뻾 (?먮룞 ?곗닠 蹂?? trust횞100, closeness횞100?뭓ffinity, BondKind 湲곕컲 respect/wariness baseline) ????而댄뙆??+ 湲곗〈 ?뚯뒪??????narrative ?쒕??덉씠??寃利?????Claude AI 異붾줎?쇰줈 ?댁깋 耳?댁뒪 議곗젙 + ?붿옄?대꼫 寃?? **Claude prompt template 諛뺤쓬** (`docs/migration/claude-prompts/`: bond-kind-inference.md, type-text-inference.md, adjustment-suggestion.md). ?덉쟾?μ튂: ?먮낯 諛깆뾽 (`data/scenarios.backup-v0.6/`) + ?쒕씪?대윴 紐⑤뱶 + diff 異쒕젰. |
| B-D9 | `session_*_result.json` ?먭린 ?뺤콉 | ??**?뺤젙 ??(a) ?쇨큵 ?먭린 + Phase 2 ???ъ깮??*. 寃곌낵 ?뚯씪? *?낅젰 ?꾨땶 異쒕젰* ???ы쁽 媛?? 諛깆뾽 `data/sessions.backup-v0.6/` ?대룞 ???먭린. Phase 2 醫낃껐 ?쒖젏 narrative ?쒕??덉씠??(Stage 5)?먯꽌 4異??쒖뒪?쒖쑝濡??쇨큵 ?ъ깮?? ?ъ깮?깅맂 寃곌낵媛 v0.7 寃利??곗씠?? |
| B-D10 | respect/wariness 珥덇린媛?猷?(0 ?쒖옉? closeness 遺?몃줈 異붿젙?) | ??**?뺤젙 ??(B') 媛꾨떒 ?대━?ㅽ떛 + BondKind 蹂댁셿**. 留덉씠洹몃젅?댁뀡 ???붿옄?대꼫媛 *BondKind 癒쇱? 諛뺤쓬* (?녿뒗 ?섏뼱 None). ?먮룞 蹂?? BondKind ?먯닔 4醫???respect -60 / wariness +80, BondKind Guardian/Mentor ??respect +60 / wariness +5, BondKind 吏湲?4 + Companion/LoyalRetainer ??respect closeness횞70 / wariness +5, BondKind None ??respect closeness횞50 / wariness max(0, -trust횞50). ?붿옄?대꼫 narrative 寃利앹뿉??議곗젙. B-D8 ?뚰겕?뚮줈?곗? 寃고빀. |
| B-D11 | (Phase 2.5) declarative_events ?곹븳 N | Phase 2.5 |
| B-D12 | Shame/Pride (`agent_id=None`) 泥섎━ | ??**?뺤젙 ??4異?蹂??0, PAD留??곹뼢** (v0.7 짠4.2 ?쒖쓽 Shame/Pride ?됱? 4異??먮룞 媛깆떊?먯꽌 臾댁떆) |
| B-D13 | 1??蹂???곹븳 | ??**?뺤젙 ??蹂꾨룄 cap ?놁쓬** (HEXACO 蹂댁젙??+ intensity 怨?+ axis_modulation 짹5媛 ?먯뿰 ?쒓퀎 ?뺤꽦) |
| B-D14 | Well-being/Prospect 10 OCC 4異?留ㅽ븨 ?꾨씫 | ??**?뺤젙 ???섎룄???꾨씫 梨꾪깮** (Joy/Distress/Hope/Fear/Satisfaction/Disappointment/Relief/FearsConfirmed/Remorse/Gratification 10媛쒕뒗 4異?蹂??0, PAD留??곹뼢. Compound 媛먯젙(Anger/Gratitude)??媛꾩젒 ?≪닔) |

### ??Phase 2.3 ?좎꽕 寃곗젙

짠3.6 ?쒕??덉씠??寃利앹뿉??諛쒓껄??*appraise ?낅젰 ?섏〈?? 臾몄젣 ?닿껐???꾪빐 Phase 2 蹂몄껜? Phase 2.5 ?ъ씠??**Phase 2.3 ??appraise ?뺣퉬** ?좎꽕:

- Phase 2 (4異??꾨찓???덉젙) ??**Phase 2.3 (appraise ?뺣퉬, ?쒕??덉씠??湲곕컲)** ??Phase 2.5 (LLM ?듯빀)
- ?묒뾽 ?꾨낫: ?쒕??덉씠???쒕굹由ъ삤 set 怨듭떇??(S1~S4 + ?좉퇋 耳?댁뒪 ~15媛? / ?꾨씫 OCC 寃利?寃쎄퀬 (I1) / Compound ?앸퀎 ?뺤옣 / `RelationshipModifiers` ?뺣???/ HEXACO쨌base_delta 誘몄꽭議곗젙
- 蹂꾨룄 spec `task-rel-phase2.3-appraise-tuning.md` (Phase 2 醫낃껐 ???묒꽦)
- `00-roadmap.md` 짠5??Phase 2.3 ???좎꽕 ?꾩슂

---

## 짠5 Risks (C 移댄뀒怨좊━ ??Stage 0 吏꾪뻾 以?

### R1 ???뚭? 硫댁쟻 ??
- ?뚯뒪??~100 ?몄텧??`Relationship` 3異??쒓렇?덉쿂???섏〈 (Builder ?⑦꽩 + `Relationship::new`)
- ?꾪솕: Stage 1?먯꽌 ?먮룞 留덉씠洹몃젅?댁뀡 ?ㅽ겕由쏀듃 寃??(B-D8 寃곗젙 ??

### R2 ???쒕굹由ъ삤 ?곗씠???붿옄?대꼫 ???묒뾽 ??**????꾪솕 (B-D8 ?뺤젙 2026-05-13)**

- 湲곗〈 ?곕젮: ~45 ?섏뼱 횞 4異?+ type = ~225 媛??붿옄?대꼫 寃???꾩슂
- **?꾪솕**: B-D8 W3+ 梨꾪깮. *?붿옄?대꼫 ???묒뾽* ??*Claude AI 異붾줎 + ?붿옄?대꼫 寃??. ?붿옄?대꼫??*?묒꽦*?섏? ?딄퀬 *寃??留? Claude prompt template (`docs/migration/claude-prompts/`)濡??몃? ?ъ슜?먮룄 ?숈씪 ?뚰겕?뚮줈???곸슜 媛??
- ?붿〈 ?꾪뿕: Claude 異붾줎 *臾명븰???뺥솗?? ??臾댄삊 ?먯쟾 留λ씫 (?섑샇吏/??몄옣猷??ъ“?곸썒???????LLM 吏??踰붿쐞 ?쒓퀎. 寃利?遺?댁? narrative ?쒕??덉씠??(Stage 5)?쇰줈 ?≪닔.

### R3 ??`Score` ???HEXACO? 怨듭쑀 ??**?댁냼 (B-D1 ?뺤젙 2026-05-13)**

- 湲곗〈 ?곕젮: `Score(f32)` 짹1.0 ?꾩옱 HEXACO 24 facet怨?*怨듭쑀 Value Object*. 4異?짹100 ?꾩엯 ??異⑸룎 媛??
- **?댁냼**: B-D1 A (遺꾨━) + 2 ???寃곗젙?쇰줈 *HEXACO `Score` ?ъ슜泥?蹂寃?0*. ??`AxisScore`/`WarinessScore` ????좎꽕濡?寃⑸━.

### R4 ??`RelationshipUpdatedPayload` 6?? ?꾨뱶 schema breaking

- ?몃? 援щ룆?? `relationship_memory_handler`, SSE bridge (`event_bridge`), Mind Studio frontend
- ?꾪솕: Stage 1?먯꽌 schema 媛깆떊 + Phase 1.6??event_bridge ?⑦꽩 ?쒖슜 (?섎룞 emit 0)

### R5 ??appraise ?낅젰 ?섏〈??(S1~S4 寃利앹뿉??諛쒓껄)

- appraise??*?붿옄?대꼫 諛뺤? Beat focus ?꾩쟾?????섏〈. ?꾨씫 ??4異?蹂???꾨씫
- ?붿옄?대꼫媛 *12+ OCC ?뺥솗 ?앸퀎* 遺???? *?곸떇??異붾줎* ?먮룞??????- ?꾪솕: **Phase 2.3 (appraise ?뺣퉬)?먯꽌 ?쒕??덉씠??湲곕컲 寃利?寃쎄퀬 (I1) + Compound ?앸퀎 ?뺤옣**
- Phase 2 蹂몄껜?먮뒗 ?곹뼢 ?놁쓬 (?꾨찓??留덉씠洹몃젅?댁뀡怨?吏곴탳)

### R6 ??base_delta 48? ?쒕굹由ъ삤 寃利?遺??
- ??媛?*諛⑺뼢??? S1~S4 寃利??듦낵. *?뺣웾媛? 誘몄꽭議곗젙 媛?μ꽦 議댁옱.
- ?꾪솕: Phase 2.3?먯꽌 ?쒕굹由ъ삤 set 湲곕컲 ?뺣웾 誘몄꽭議곗젙 (Phase 2 蹂몄껜?먯꽑 v0.7 짠4.2 ??洹몃?濡?諛뺤쓬)

---

## 짠6 Baseline (D 移댄뀒怨좊━)

Phase 2 ?뚭? 寃利앹쓽 湲곗??? Phase 1 醫낃껐 ?쒖젏 (2026-05-11 baseline) ?몄슜 + Stage 1 吏꾩엯 吏곸쟾 ?ъ륫??

### D1 ??cargo test ?듦낵 移댁슫??
| ??ぉ | ?쒖젏 / ?섏튂 | 異쒖쿂 |
|---|---|---|
| Phase 1 醫낃껐 baseline | **1095 passed**, 0 failed (2026-05-11) | `phase1-checkpoint-report.md:35-36, 308` |
| **Stage 1 吏꾩엯 吏곸쟾 ?ъ륫??* | **1220 passed**, 3 skipped, 0 failed (2026-05-14) | `baselines/cargo-test-2026-05-14-PASS.log` ??Phase 1.5/1.6 + ?꾩냽 ?꾩쟻 +125 |
| `cargo check --all-features` | ??| ?숈씪 |
| `cargo build --features chat` | ??| ?숈씪 |

**寃뚯씠??*: Phase 2 留덉씠洹몃젅?댁뀡 ?꾨즺 ??*Stage 1 吏꾩엯 ?쒖젏 1220 + ?좉퇋 ?뚯뒪???? ?듦낵. ?뚭? 0嫄?

### D2 ??`dispatch_v2(EndDialogue)` latency

| 耳?댁뒪 | Phase 1 latency | follow-up |
|---|---|---|
| chitchat | **24.17 쨉s** | 3 |
| significant | **35.03 쨉s** | 4 |
| legacy | **29.34 쨉s** | 3 |

**寃뚯씠??*: Phase 2 ??*짹20% ?대궡*. 4異?留ㅽ븨 異붽?濡??쎄컙 利앷? ?덉긽 (?? ~30/42/35 쨉s). axis_modulation??reflection LLM?먯꽌 異붽??섎?濡?蹂??곹뼢 ?놁쓬.

### D3 ??Narrative 3諛대뱶 calibration

| ?쒕굹由ъ삤 | significance | Target |
|---|---|---|
| chitchat-passerby | **0.000** | <0.3 ??|
| daily-training | **0.461** | 0.3~0.7 ??|
| lin-chong-shanshenmiao | **0.980** | ??.7 ??|

**寃뚯씠??*: Phase 2 留덉씠洹몃젅?댁뀡 ???숈씪 ?쒕굹由ъ삤??*3諛대뱶 ?꾩튂 蹂댁〈*. 媛以묒튂 `0.40/0.30/0.15/0.15` + ?꾧퀎媛?`0.3` ?좎?.

### D4 ??`compute_significance` ?붿쭊 ?깅뒫

| ??ぉ | Phase 1 baseline |
|---|---|
| `compute_significance(10 turn) 횞10000` | **8.36 쨉s/call** (target <1ms, 100x 留덉쭊) |

**寃뚯씠??*: Phase 2 ??짹20% ?대궡.

### D5 ??`MAX_EVENTS_PER_COMMAND`

| ??ぉ | ?꾩옱 | A5 寃곕줎 |
|---|---|---|
| ?곸닔 媛?| **22** | Phase 2 蹂몄껜 蹂寃?0, Phase 2.5 worst-case 17, ?몄긽 遺덊븘??|

### D6 ??肄붾뱶 硫뷀듃由?
| ??ぉ | Phase 1 醫낃껐 | 異쒖쿂 |
|---|---|---|
| domain/ tokio 李몄“ | 0 | userMemories |
| ports.rs tokio 李몄“ | 1 (`send_message_stream` ??蹂꾨룄 migration) | userMemories |
| application/ tokio 李몄“ | 5 (event_bus + memory_projector + director/) | userMemories |
| EventKind variant ??| 31媛?| `00-roadmap.md` 짠2 |

### Stage 1 吏꾩엯 吏곸쟾 ?ъ륫???묒뾽

Stage 1 ?쒖옉 泥??묒뾽: ???섏튂 *?ъ륫???섏뿬 `baselines/cargo-test-2026-MM-DD-PASS.log` ?⑦꽩?쇰줈 諛뺤젣. Phase 2 吏꾪뻾 以?鍮꾧탳 湲곗?.

---

## 짠7 Stages

Phase 1 6 stage ?⑦꽩 ?곕씪 遺꾪븷. 吏곸꽑 ?섏〈 (Stage N ??Stage N+1). 媛?stage 醫낃껐 ??grep 寃뚯씠??+ ?듦낵 移댁슫??寃利?

### Stage 1 ??Type ?좎꽕 + Domain ?ъ옉??(??spec frozen 2026-05-14)

**踰붿쐞 (?곸쐞 怨④꺽)**:
- `AxisScore(f32)` + `WarinessScore(f32)` ?좎꽕 (B-D1/D2)
- `BondKind` 11 variants / `BondStatus` 5 variants + `accepts_live_input()` / `Partnership` 4 variants enum
- `Relationship` 蹂몄껜 ?ъ옉?? 4異?+ bond_kind + bond_status + partnership + type + type_history (B-D4: `power` ?먭린)
- `RelationshipBuilder` 4異?API
- `Relationship::neutral()` ?쒓렇?덉쿂 蹂댁〈 (16怨??먮룞 ?≪닔)
- ?⑥쐞 ?뚯뒪??
**?꾪뿕**: ?묒쓬~以? ?꾨찓??紐⑤뱢 遺꾪븷 + 4異??꾩엯. 16怨??먮룞 ?≪닔媛 ?명꽣?섏씠??硫댁쟻 蹂댁〈.

?몃? ??ぉ 1.1~1.9:

#### 1.1 ???붾젆?좊━ 援ъ“

**寃곗젙**: (a) 紐⑤뱢 遺꾪븷 梨꾪깮.

```
src/domain/relationship/
  mod.rs                # Relationship aggregate (??relationship.rs 蹂몄껜 ?닿?) + RelationshipBuilder + neutral
  axis.rs               # AxisScore + WarinessScore + AxisKind + AxisDelta
  bond.rs               # BondKind + BondStatus + accepts_live_input()
  partnership.rs        # Partnership
```

**鍮꾪룷??* (?섎룄??:
- `RelationshipChangeCause` enum? `src/domain/event.rs`??*?꾩옱 ?꾩튂 ?좎?* (A3 寃利? variant ?섎?媛 *?대깽??遺꾨쪟*??媛源뚯?, Relationship aggregate ?대? X)
- OCC ??4異?留ㅽ븨 (`base_delta` / `hexaco_modifier` / `update_axes_from_emotion`)? **Stage 2 ??`src/domain/relationship/mapping.rs` ?좎꽕** ?꾩튂 ?덉빟

**?닿? ?⑦꽩**:
- 湲곗〈 `src/domain/relationship.rs` (~700以? ???붾젆?좊━濡?遺꾪븷
- 湲곗〈 ?ъ슜泥?import 寃쎈줈 `use crate::domain::relationship::Relationship;` 洹몃?濡??좎? (mod.rs媛 re-export)
- `pub use axis::{AxisScore, WarinessScore, AxisKind, AxisDelta};` ??mod.rs?먯꽌 re-export

**?묒뾽 ?쒖꽌**: 1.1 ?붾젆?좊━ ?앹꽦 ??1.2~1.5 ??????뺤쓽 ??1.6 蹂몄껜 ?닿?/?ъ옉????1.7~1.8 ??1.9 ?뚯뒪??
**寃뚯씠??*: `cargo check` ?듦낵 (?붾젆?좊━ 遺꾪븷 ??而댄뙆???덉쟾).

---

#### 1.2 ??`AxisScore` + `WarinessScore`

**紐⑹쟻**: 4異??먯닔??*遺덈???媛뺤젣* (踰붿쐞 + wariness ?뚯닔 而댄뙆???쒖젏 李⑤떒) + 4異??곗닠 ?곗궛 ?명봽??

**?꾩튂**: `src/domain/relationship/axis.rs` (?좉퇋)

**?쒓렇?덉쿂**:

```rust
//! 愿怨?4異??먯닔 ??낃낵 ?곗닠 ?곗궛.
//! - AxisScore: trust/affinity/respect 짹100
//! - WarinessScore: wariness 0..=100 (?뚯닔 ?섎? ?놁쓬, 蹂???낆쑝濡?而댄뙆???쒖젏 李⑤떒)

use serde::{Deserialize, Serialize};

/// ?뚯뼇 媛??異뺤쓽 ?먯닔 (trust / affinity / respect).
///
/// 踰붿쐞: -100.0 ~ +100.0
/// ?대?: f32 (B-D2 ??base_delta 횞 intensity 횞 HEXACO 怨깆뀍 ?뺣????좎?)
/// JSON: ?뺤닔 round 異쒕젰 (?붿옄?대꼫 移쒗솕)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct AxisScore(f32);

impl AxisScore {
    pub const MIN: f32 = -100.0;
    pub const MAX: f32 = 100.0;
    pub const NEUTRAL: AxisScore = AxisScore(0.0);

    /// ?낅젰??짹100?쇰줈 clamp.
    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn value(&self) -> f32 { self.0 }

    /// delta瑜??뷀븯怨?clamp????媛?
    pub fn add(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

impl Default for AxisScore {
    fn default() -> Self { Self::NEUTRAL }
}

/// 寃쎄퀎??異??먯닔 (wariness ?꾩슜).
///
/// 踰붿쐞: 0.0 ~ +100.0
/// 蹂???낆씠誘濡?*而댄뙆???쒖젏*??AxisScore? ?쇰룞 李⑤떒.
/// `WarinessScore::new(-50.0)` ?몄텧? runtime??0.0?쇰줈 clamp.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct WarinessScore(f32);

impl WarinessScore {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 100.0;
    pub const NEUTRAL: WarinessScore = WarinessScore(0.0);

    pub fn new(value: f32) -> Self {
        Self(value.clamp(Self::MIN, Self::MAX))
    }

    pub fn value(&self) -> f32 { self.0 }

    pub fn add(self, delta: f32) -> Self {
        Self::new(self.0 + delta)
    }
}

impl Default for WarinessScore {
    fn default() -> Self { Self::NEUTRAL }
}

/// 4異뺤씠 *?숈떆?? 諛쏅뒗 蹂??
/// base_delta ??+ HEXACO 怨깆뀍 寃곌낵 (Stage 2 ?뺤쓽/?ъ슜).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AxisDelta {
    pub trust:    f32,
    pub affinity: f32,
    pub respect:  f32,
    pub wariness: f32,
}

impl AxisDelta {
    /// ?ㅼ뭡??怨?(intensity 횞 HEXACO modifier ??.
    pub fn scaled_by(self, factor: f32) -> Self {
        Self {
            trust:    self.trust    * factor,
            affinity: self.affinity * factor,
            respect:  self.respect  * factor,
            wariness: self.wariness * factor,
        }
    }
}

/// ??AxisDelta ?깅텇蹂??⑹궛 (Stage 2 ??蹂듯빀 媛먯젙??base_delta ?⑹궛???ъ슜).
/// ?? `Anger.base_delta() + Hate.base_delta() + Reproach.base_delta()`
impl std::ops::Add for AxisDelta {
    type Output = AxisDelta;
    fn add(self, other: AxisDelta) -> AxisDelta {
        AxisDelta {
            trust:    self.trust    + other.trust,
            affinity: self.affinity + other.affinity,
            respect:  self.respect  + other.respect,
            wariness: self.wariness + other.wariness,
        }
    }
}

/// 異??앸퀎??(base_delta ??lookup???ъ슜, Stage 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisKind {
    Trust, Affinity, Respect, Wariness,
}
```

**?ㅺ퀎 ?섎룄 5媛?*:

| # | ??ぉ | ?섎룄 |
|---|---|---|
| ??| 2 ???遺꾨━ (`AxisScore` / `WarinessScore`) | *而댄뙆???쒖젏*??wariness ?뚯닔 李⑤떒. `let w: WarinessScore = AxisScore::new(50.0);` ??而댄뙆???먮윭 (B-D1) |
| ??| `NEUTRAL` const + `impl Default` 紐낆떆 | `Relationship::neutral()`??湲곕낯媛? derive Default??*?곗뿰?? 0.0怨??쇱튂?섏?留?*紐낆떆 impl*濡??섎룄 諛뺤쓬. 1.8 ?먮룞 ?≪닔 ?꾩? |
| ??| `add(self, delta: f32)` 硫붿꽌?쒕줈留?蹂??| ?몃??먯꽌 `score.value() + 50.0` 媛숈씠 raw f32 ?곗닠?섎㈃ clamp ??????`add()` 媛뺤젣濡?*?먮룞 clamp* |
| ??| `AxisDelta` 蹂????+ `Add` trait | 4異뺤씠 *?쒓볼踰덉뿉 諛쏅뒗 蹂??. Stage 2??蹂듯빀 媛먯젙 ?⑹궛 (`Anger + Hate + Reproach`)???ъ슜. `scaled_by()`濡?intensity/HEXACO 怨?|
| ??| `AxisKind` enum | Stage 2 `base_delta` ??lookup 諛?`update_axes_from_emotion`??異뺣퀎 遺꾧린???ъ슜. `Eq + Hash` 諛뺥? HashMap ?ㅻ줈 ?ъ슜 媛??|

**?⑥쐞 ?뚯뒪??耳?댁뒪** (1.9?먯꽌 援ы쁽):

```
[clamp 踰붿쐞]
- AxisScore::new(150.0).value()       == 100.0  (??cap)
- AxisScore::new(-200.0).value()      == -100.0 (??cap)
- AxisScore::new(50.0).value()        == 50.0   (?뺤긽)
- WarinessScore::new(-50.0).value()   == 0.0    ???듭떖 (?뚯닔 floor)
- WarinessScore::new(150.0).value()   == 100.0
- WarinessScore::new(50.0).value()    == 50.0

[add() ?먮룞 clamp]
- AxisScore::new(50.0).add(60.0).value()       == 100.0  (??cap)
- AxisScore::new(-50.0).add(-60.0).value()     == -100.0 (??cap)
- WarinessScore::new(80.0).add(50.0).value()   == 100.0
- WarinessScore::new(30.0).add(-50.0).value()  == 0.0

[NEUTRAL + Default]
- AxisScore::NEUTRAL.value()     == 0.0
- WarinessScore::NEUTRAL.value() == 0.0
- AxisScore::default()           == AxisScore::NEUTRAL
- WarinessScore::default()       == WarinessScore::NEUTRAL

[AxisDelta scaled_by]
- AxisDelta { trust: 20.0, affinity: 10.0, respect: 0.0, wariness: -10.0 }
    .scaled_by(0.5)
  == AxisDelta { trust: 10.0, affinity: 5.0, respect: 0.0, wariness: -5.0 }

[AxisDelta Add ??Stage 2 蹂듯빀 媛먯젙 ?⑹궛 ?⑦꽩]
- Anger??base_delta + Hate??base_delta = (trust -35, affinity -35, respect -5, wariness +40)
  (Stage 2??base_delta ?쒓? 諛뺥????뺥솗??耳?댁뒪 ??1.2???곗닠 ?숈옉留?寃利?
- AxisDelta { trust: 10.0, ... } + AxisDelta { trust: 5.0, ... }
  ??trust == 15.0

[serde round-trip]
- AxisScore::new(75.0) ??serde_json::to_string ??"75.0" ??from_str ??AxisScore::new(75.0)
- WarinessScore::new(50.0) ?숈씪
```

**而댄뙆??李⑤떒 寃利?* (Rust 而댄뙆?쇰윭 ?먮룞, 紐낆떆 unit test ?놁쓬):
```rust
// ??肄붾뱶??而댄뙆???먮윭:
// let w: WarinessScore = AxisScore::new(50.0);
// ??expected struct `WarinessScore`, found struct `AxisScore`
```

**鍮꾪룷??*:
- `Add<f32>` for AxisScore (raw delta ?뷀븯湲? ??`add()` 硫붿꽌?쒕줈 異⑸텇, trait 以묐났
- `Add<AxisScore>` for AxisScore ??*AxisScore + AxisScore* ?쒕㎤???놁쓬 (?섏떖 1 寃곕줎)
- `Hash` for AxisScore/WarinessScore ??f32 NaN ?뚮Ц 遺덇?

#### 1.3 ??`BondKind`

**紐⑹쟻**: 愿怨꾩쓽 *?뺤꽌쨌湲곕뒫??遺꾨쪟* 11醫? axes 蹂?????꾧퀎 ?꾨떖/?댄깉濡?*Channel 2 Temporal (Phase 3a)*?먯꽌 ?먮룞 吏꾩엯/?댄깉. Phase 2??*enum ?뺤쓽 + ?곸뿭 ?ы띁*留?

**?꾩튂**: `src/domain/relationship/bond.rs` (?좉퇋, BondStatus? 媛숈? ?뚯씪)

**?쒓렇?덉쿂**:

```rust
//! BondKind / BondStatus ??愿怨꾩쓽 ?뺤꽌쨌湲곕뒫 遺꾨쪟 + ?쒕룞 ?곹깭.
//! relationships.md v0.7 짠3.1 (BondKind 11) + 짠3.5 (BondStatus 5)

use serde::{Deserialize, Serialize};

/// 愿怨꾩쓽 ?뺤꽌쨌湲곕뒫??遺꾨쪟 (relationships.md v0.7 짠3.1).
///
/// 11 variants 4 ?곸뿭:
/// - 吏湲걔룸룞諛?(?묎레 ?꾧퀎): 6醫???SwornBrothers, MasterDisciple, Soulmate, LoyalRetainer, Companion, Guardian
/// - 硫섑넗 (以묎컙洹??꾧퀎): 1醫???Mentor
/// - ?먯닔 (?뚭레 ?꾧퀎): 4醫???BloodEnemy, ArchRival, Betrayer, Oppressor
///
/// Phase 2??*enum ?뺤쓽 + ?곸뿭 ?ы띁*源뚯?.
/// ?먮룞 吏꾩엯/?댄깉 (?쒓컙 寃뚯씠??+ ?꾧퀎媛?? Phase 3a (Channel 2 Temporal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BondKind {
    // 吏湲걔룸룞諛????묎레 ?꾧퀎 (6醫?
    SwornBrothers,    // ?섑삎?쑣룸룞吏??    MasterDisciple,   // ?щ?-?쒖옄??(臾댁닠 鍮꾩쟾 ?꾩닔)
    Soulmate,         // ?곹샎???숇컲?먰삎
    LoyalRetainer,    // 媛?졖룹??명삎
    Companion,        // ?됱깮???곗씤 (v0.6 ?좎꽕)
    Guardian,         // 遺紐??먮???(v0.6 ?좎꽕)

    // 硫섑넗 ??以묎컙洹??꾧퀎
    Mentor,           // ?몄깮 ?좊같쨌?꾨같

    // ?먯닔 ???뚭레 ?꾧퀎 (4醫?
    BloodEnemy,       // ?덉쟻
    ArchRival,        // ?숈쟻
    Betrayer,         // 諛곗떊??    Oppressor,        // ?뺤젣??}

impl BondKind {
    /// 吏湲?4醫?(SwornBrothers, MasterDisciple, Soulmate, LoyalRetainer).
    /// 以묎뎅??*吏湲??ε런)* ??源딆? ?뺤떊???숈?/吏??
    pub fn is_zhiji(&self) -> bool {
        matches!(self,
            Self::SwornBrothers | Self::MasterDisciple
            | Self::Soulmate | Self::LoyalRetainer
        )
    }

    /// ?됱깮???곗씤 (Companion).
    pub fn is_companion_class(&self) -> bool {
        matches!(self, Self::Companion)
    }

    /// 遺紐??먮???(Guardian).
    pub fn is_guardian(&self) -> bool {
        matches!(self, Self::Guardian)
    }

    /// ?몄깮 ?좊같쨌?꾨같 (Mentor).
    pub fn is_mentor(&self) -> bool {
        matches!(self, Self::Mentor)
    }

    /// ?먯닔 4醫?(BloodEnemy, ArchRival, Betrayer, Oppressor).
    pub fn is_enemy(&self) -> bool {
        matches!(self,
            Self::BloodEnemy | Self::ArchRival
            | Self::Betrayer | Self::Oppressor
        )
    }
}
```

**?ㅺ퀎 ?섎룄 4媛?*:

| # | ??ぉ | ?섎룄 |
|---|---|---|
| ??| 11 variants 洹몃?濡?(v0.7 짠3.1 紐낆떆) | ?붿옄?대꼫 移쒖닕 ??臾댄삊 ?먯쟾??*愿怨?移댄깉濡쒓렇*. 異붽? ?좎꽕? Phase 3+?먯꽌. |
| ??| ?곸뿭 ?ы띁 5媛?(v0.7 짠3.1 洹몃?濡? | B-D10 留덉씠洹몃젅?댁뀡 baseline 猷곗뿉??*?곸뿭蹂?遺꾧린* ???ъ슜. `is_zhiji`??*吏湲??ε런)* 臾댄삊 ?꾨찓???⑹뼱 蹂댁〈 (npc-mind-rs ?뺤껜??. |
| ??| `is_positive_pole`/`is_negative_pole` *鍮꾪룷?? | YAGNI ??Phase 2?먯꽌 ?ъ슜 鍮덈룄 ??쓬. Phase 3a Channel 2 Temporal 吏꾩엯 ???꾩슂?댁?硫?異붽?. |
| ??| `#[serde(rename_all = "snake_case")]` | JSON 吏곷젹?? `"sworn_brothers"`, `"blood_enemy"` ?? ?붿옄?대꼫 ?쒕굹由ъ삤 JSON 移쒗솕. |

**`Display` impl 鍮꾪룷??*: ?꾨찓??enum? *?쒖닔*. presentation layer (`presentation/locale.rs`)媛 ko/en ?쇰꺼 諛뺤쓬 ???꾩옱 `PowerLevel` ?⑦꽩 ?좎?. 援?젣??誘몃옒 蹂댁〈. Stage 4 ?먮뒗 6?먯꽌 諛뺤쓬.

**?⑥쐞 ?뚯뒪??耳?댁뒪** (1.9?먯꽌 援ы쁽):

```
[?곸뿭 ?ы띁 ??遺꾨쪟 ?뺥빀]
- BondKind::SwornBrothers.is_zhiji()      == true
- BondKind::MasterDisciple.is_zhiji()     == true
- BondKind::Soulmate.is_zhiji()           == true
- BondKind::LoyalRetainer.is_zhiji()      == true
- BondKind::Companion.is_zhiji()          == false
- BondKind::Guardian.is_zhiji()           == false
- BondKind::Mentor.is_zhiji()             == false
- BondKind::BloodEnemy.is_zhiji()         == false

[Companion / Guardian / Mentor]
- BondKind::Companion.is_companion_class() == true
- BondKind::Guardian.is_guardian()         == true
- BondKind::Mentor.is_mentor()             == true
- BondKind::SwornBrothers.is_companion_class() == false  (吏湲곗? ?됱깮???곗씤 援щ퀎)

[?먯닔]
- BondKind::BloodEnemy.is_enemy()  == true
- BondKind::ArchRival.is_enemy()   == true
- BondKind::Betrayer.is_enemy()    == true
- BondKind::Oppressor.is_enemy()   == true
- BondKind::Mentor.is_enemy()      == false

[?곸뿭 ?곹샇 諛고???寃利???11 variants 紐⑤몢 ?뺥솗??1媛??곸뿭???랁븿]
- 11 variants 媛곴컖: is_zhiji + is_companion_class + is_guardian + is_mentor + is_enemy ???⑹씠 ?뺥솗??1

[serde round-trip]
- BondKind::SwornBrothers ??"sworn_brothers" ??SwornBrothers
- BondKind::BloodEnemy   ??"blood_enemy"    ??BloodEnemy
- BondKind::MasterDisciple ??"master_disciple" ??MasterDisciple
- BondKind::LoyalRetainer  ??"loyal_retainer"  ??LoyalRetainer
```

**鍮꾪룷??*:
- `Display` impl ??presentation layer (Stage 4 ?먮뒗 6)
- `is_positive_pole` / `is_negative_pole` ??Phase 3a?먯꽌 ?꾩슂 ??異붽?
- ?쒓컙 寃뚯씠??/ ?꾧퀎媛???Phase 3a Channel 2 Temporal
- BondKind 吏꾩엯 議곌굔 ?⑥닔 ??Phase 3a

#### 1.4 ??`BondStatus` + `accepts_live_input()`

**紐⑹쟻**: 愿怨꾩쓽 *?쒕룞 ?곹깭*. base_delta 李⑤떒??*?듭떖 寃뚯씠?? ??Stage 2 `update_axes_from_emotion`?????ы띁濡?*?낅젰 嫄곕?* 寃곗젙.

**?꾩튂**: `src/domain/relationship/bond.rs` (1.3怨?媛숈? ?뚯씪)

**?쒓렇?덉쿂**:

```rust
use crate::domain::event::EventId;
use serde::{Deserialize, Serialize};

/// 愿怨꾩쓽 ?쒕룞 ?곹깭 (relationships.md v0.7 짠3.5).
///
/// - Active: ?뺤긽 ?쒖꽦. axes ?먮룞 蹂??
/// - Resolved { reason }: terminal ???뷀빐/留ㅻ벊 ?깆쑝濡?*?꾧껐*. axes freeze.
/// - Deceased: terminal ??????щ쭩. axes freeze.
/// - Dormant: ?대㈃ (?ㅻ옖 誘몄젒珥?. axes freeze. ?몃━嫄곕줈 Reactivating ?꾩씠 媛??
/// - Reactivating { trigger }: 蹂듦? 以?(transient state). axes 諛쏄린 ?쒖옉 ??*?곗냽???뚮났*.
///   Active???李⑥씠: 蹂듦? trigger 諛뺥옒 + Phase 3a ?쒓컙 寃뚯씠?????(Active ?먮룞 ?꾩씠).
///
/// ?꾩씠 猷곗? Phase 3a (Channel 2 Temporal). Phase 2??enum + `accepts_live_input()`源뚯?.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BondStatus {
    Active,
    Resolved { reason: String },
    Deceased,
    Dormant,
    Reactivating { trigger: EventId },
}

impl Default for BondStatus {
    fn default() -> Self { BondStatus::Active }
}

impl BondStatus {
    /// 4異??먮룞 蹂?숈쓣 諛쏅뒗吏 (Stage 2 base_delta 李⑤떒???듭떖 ?ы띁).
    /// v0.7 짠4.1:
    ///   `if !rel.bond_status.accepts_live_input() { return; }`
    ///
    /// - Active: true (?뺤긽 ?쒖꽦)
    /// - Reactivating: true ??(蹂듦? ?쒖옉 = axes ?ㅼ떆 諛쏄린. Reactivating state??議댁옱 ?섎?)
    /// - Dormant: false (?대㈃)
    /// - Resolved: false (terminal freeze)
    /// - Deceased: false (terminal freeze)
    pub fn accepts_live_input(&self) -> bool {
        matches!(self, BondStatus::Active | BondStatus::Reactivating { .. })
    }
}
```

**?ㅺ퀎 ?섎룄 5媛?*:

| # | ??ぉ | ?섎룄 |
|---|---|---|
| ??| 5 variants 洹몃?濡?(v0.7 짠3.5 紐낆떆) | 2 variants??payload ?ы븿: `Resolved { reason }` / `Reactivating { trigger }`. terminal/transient state ?쒗쁽. |
| ??| `#[serde(tag = "kind", rename_all = "snake_case")]` | RelationshipChangeCause ?⑦꽩 (event.rs:137) ??JSON: `{ "kind": "resolved", "reason": "..." }`. payload variants ?먯뿰 吏곷젹?? |
| ??| `Default = Active` 紐낆떆 | 留덉씠洹몃젅?댁뀡 ??湲곗〈 ?쒕굹由ъ삤 ?섏뼱媛 紐⑤몢 default Active濡?諛뺥옒. Relationship Aggregate Default ?먮룞 ?≪닔. |
| ??| **`Reactivating.accepts_live_input() == true`** ??| ?곗냽???뚮났 ?쒕㎤?????ы쉶 泥??쒓컙遺???뺤꽌媛 ?ㅼ떆 ?吏곸엫. Reactivating??*議댁옱 ?섎?*瑜??대┝ (false??ㅻ㈃ Dormant? ?숈씪 ?숈옉 ??state 遺꾨━ ?댁쑀 ?щ씪吏?. |
| ??| `Copy` 鍮꾪룷??| variants??`String`/`EventId` ?ы븿 ??Copy 遺덇?. `Clone`留? |

**`accepts_live_input` 寃곗젙 留ㅽ듃由?뒪**:

| ?곹깭 | ?섎? | `accepts_live_input` |
|---|---|---|
| Active | ?뺤긽 ?쒖꽦 | **true** |
| Reactivating { trigger } | 蹂듦? ?쒖옉 (transient) | **true** ??|
| Dormant | ?대㈃ (誘몄젒珥? | false |
| Resolved { reason } | ?꾧껐 (?뷀빐/留ㅻ벊) | false (terminal) |
| Deceased | ????щ쭩 | false (terminal) |

**異붽? ?ы띁 鍮꾪룷??* (YAGNI):
- `is_terminal()` (Resolved + Deceased) ??Phase 3a Channel 2 Temporal?먯꽌 ?꾩슂?댁?硫?異붽?
- `is_dormant()` ??`matches!` 濡?異⑸텇
- ?꾩씠 ?⑥닔 (`reactivate(trigger)` ?? ??Phase 3a (?쒓컙 寃뚯씠??+ ?몃━嫄?猷?

**?⑥쐞 ?뚯뒪??耳?댁뒪** (1.9?먯꽌 援ы쁽):

```
[accepts_live_input ???듭떖 寃뚯씠??
- BondStatus::Active.accepts_live_input()                               == true
- BondStatus::Reactivating { trigger: EventId(...) }.accepts_live_input() == true  ??- BondStatus::Dormant.accepts_live_input()                              == false
- BondStatus::Resolved { reason: "?ы솕".into() }.accepts_live_input()    == false
- BondStatus::Deceased.accepts_live_input()                             == false

[Default]
- BondStatus::default() == BondStatus::Active

[serde round-trip]
- Active ??{"kind": "active"} ??Active
- Resolved { reason: "?ы솕" } ??{"kind": "resolved", "reason": "?ы솕"} ??Resolved { reason: "?ы솕" }
- Deceased ??{"kind": "deceased"} ??Deceased
- Dormant ??{"kind": "dormant"} ??Dormant
- Reactivating { trigger: EventId("evt_001") } ??{"kind": "reactivating", "trigger": "evt_001"} ??Reactivating
```

**鍮꾪룷??*:
- ?꾩씠 ?⑥닔 / ?몃━嫄?猷???Phase 3a Channel 2 Temporal
- `Eq` / `Hash` ??String ?뚮Ц???좎쨷, Phase 2 ?ъ슜泥??놁쓬 (?꾩슂?댁?硫?異붽?)
- `is_terminal` ??異붽? ?ы띁 ??YAGNI

#### 1.5 ??`Partnership`

**紐⑹쟻**: 愿怨꾩쓽 *?뺤떇???숇컲 ?곹깭*. BondKind? *?꾩쟾 吏곴탳* ???뺣왂寃고샎 = trust 0 + Spouse 媛?? axes? 吏곸젒 ?곕룞 X. 蹂???숇젰? *怨듭떇 ?ш굔* (Phase 2.5 declarative_events `PartnershipChange` ?꾨낫).

**?꾩튂**: `src/domain/relationship/partnership.rs` (?좉퇋)

**?쒓렇?덉쿂**:

```rust
//! Partnership ??愿怨꾩쓽 ?뺤떇???숇컲 ?곹깭.
//! relationships.md v0.7 짠3.6

use serde::{Deserialize, Serialize};

/// ?뺤떇???숇컲 ?곹깭 (relationships.md v0.7 짠3.6).
///
/// - Spouse: 諛곗슦??(?쇱씤 愿怨?
/// - Engaged: ?쏀샎 (寃고샎 ?쎌냽)
/// - Lover: ?곗씤 (鍮꾧났???뺤꽌??愿怨?
/// - Separated: 蹂꾧굅 (Spouse/Engaged/Lover?먯꽌??寃곕퀎 ?곹깭)
///
/// BondKind? *?꾩쟾 吏곴탳*. axes? *吏곸젒 ?곕룞 X*.
/// ?뺣왂寃고샎 = trust 0 + Spouse 媛??
/// 蹂???숇젰? *怨듭떇 ?ш굔* ??Phase 2.5 declarative_events `PartnershipChange`.
///
/// `Relationship.partnership: Option<Partnership>` ?⑦꽩?쇰줈 ?ъ슜 (None = ?뺤떇 愿怨??놁쓬).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Partnership {
    Spouse,
    Engaged,
    Lover,
    Separated,
}
```

**?ㅺ퀎 ?섎룄 4媛?*:

| # | ??ぉ | ?섎룄 |
|---|---|---|
| ??| 4 variants payload ?놁쓬 | ?⑥닚 *?뺤떇 ?쇰꺼*. 寃고샎 ?ъ쑀, 蹂꾧굅 ?댁쑀 ?깆? *type ?먯쑀 ?띿뒪?? ?먮뒗 `RelationshipChangeCause`??諛뺥옒. |
| ??| `Copy` 媛??| payload ?놁쑝誘濡?BondKind泥섎읆 Copy. |
| ??| `Eq + Hash` | payload ?놁쑝誘濡??먯뿰. HashMap ?ㅻ줈 ?ъ슜 媛?? |
| ??| `Default` impl *?놁쓬* | `Option<Partnership>`?쇰줈 泥섎━ (`Relationship.partnership: Option<Partnership>`, None = ?뺤떇 愿怨??놁쓬). Default媛 *?대뒓 variant*?몄? ?섎? 紐⑦샇?섎?濡?紐낆떆??Option???먯뿰. |

**`Display` impl 鍮꾪룷??*: BondKind? ?숈씪 ??presentation layer?먯꽌 ko/en ?쇰꺼.

**異붽? ?ы띁 鍮꾪룷??* (YAGNI):
- `is_committed()` (Spouse + Engaged + Lover) ??Phase 2.5 declarative_events 寃利????꾩슂?댁?硫?異붽?
- `is_separated()` ??`matches!`濡?異⑸텇

**?⑥쐞 ?뚯뒪??耳?댁뒪** (1.9?먯꽌 援ы쁽):

```
[variants ?뺥빀]
- Partnership::Spouse, Engaged, Lover, Separated ??4醫?紐⑤몢 ?뺤쓽??
[serde round-trip]
- Spouse    ??"spouse"    ??Spouse
- Engaged   ??"engaged"   ??Engaged
- Lover     ??"lover"     ??Lover
- Separated ??"separated" ??Separated

[Copy + Eq + Hash ?숈옉]
- let a = Partnership::Spouse; let b = a;     // Copy OK
- a == b                                       // Eq OK
- HashSet::from([Spouse, Engaged])             // Hash OK
```

**鍮꾪룷??*:
- ?꾩씠 ?⑥닔 (`Spouse ??Separated` ?? ??Phase 2.5 declarative_events `PartnershipChange`
- `Display` impl ??presentation layer
- `is_committed` ??異붽? ?ы띁 ??YAGNI

#### 1.6 ??`Relationship` 蹂몄껜 ?ъ옉??
**紐⑹쟻**: 1.2~1.5?먯꽌 諛뺤? *紐⑤뱺 ??????듯빀. 4異?+ BondKind + BondStatus + Partnership + type. `power` ?쒓굅 (B-D4). 湲곗〈 ?명꽣?섏씠??(`neutral`, `modifiers`) 蹂댁〈?섏뿬 16怨??먮룞 ?≪닔.

**?꾩튂**: `src/domain/relationship/mod.rs` (?붾젆?좊━ 遺꾪븷 ??蹂몄껜)

**?쒓렇?덉쿂**:

```rust
//! Relationship Aggregate ??4異?+ BondKind + BondStatus + Partnership + type ?듯빀.

use crate::domain::event::{EventId, RelationshipChangeCause};
use crate::domain::npc::NpcId;
use serde::{Deserialize, Serialize};

pub use axis::{AxisScore, WarinessScore, AxisDelta, AxisKind};
pub use bond::{BondKind, BondStatus};
pub use partnership::Partnership;

mod axis;
mod bond;
mod partnership;

/// 愿怨?蹂몄껜 (relationships.md v0.7).
///
/// 4異?+ bond_kind + bond_status + partnership + type + type_history.
/// `power` ?먭린 (B-D4) ???꾧퀎 ?뺣낫??`type_text` ?먯쑀 ?띿뒪?몃줈 ?≪닔.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    owner: NpcId,
    target: NpcId,

    // 4異?(B-D1: 蹂????
    trust:    AxisScore,
    affinity: AxisScore,
    respect:  AxisScore,
    wariness: WarinessScore,

    // 遺꾨쪟 + ?곹깭 (1.3~1.5)
    bond_kind:   Option<BondKind>,            // None = 誘몃텇瑜?    #[serde(default)]
    bond_status: BondStatus,                   // default = Active
    partnership: Option<Partnership>,          // None = ?뺤떇 愿怨??놁쓬

    // ?먯쑀 ?띿뒪??(B-D4: power ?≪닔)
    #[serde(rename = "type")]
    type_text:   String,                       // ?? "議곗젙 ?꾧퀎: 援먮몢?믫깭?? 遺??愿怨?
    #[serde(default)]
    type_history: Vec<TypeChange>,
}

/// type 蹂寃??대젰 element (v0.7 짠2).
///
/// ?쒓컙/?먯씤 異붿쟻? *RelationshipUpdated event log*?먯꽌 蹂꾨룄.
/// type_history??*?쒖궗 ?먮쫫*??吏묒쨷 (?섏떖 1 寃곗젙: 3 ?꾨뱶 ?⑥닚 援ъ“).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeChange {
    pub from_type: String,
    pub to_type:   String,
    pub note:      String,    // 蹂寃?留λ씫 (?? "?섑삎??寃곗뿰 ?ш굔")
}

impl Relationship {
    /// ??愿怨??앹꽦 (?쒕굹由ъ삤 JSON 吏꾩엯?먯뿉???몄텧).
    pub fn new(
        owner: NpcId, target: NpcId,
        trust: AxisScore, affinity: AxisScore,
        respect: AxisScore, wariness: WarinessScore,
    ) -> Self {
        Self {
            owner, target,
            trust, affinity, respect, wariness,
            bond_kind: None,
            bond_status: BondStatus::Active,
            partnership: None,
            type_text: String::new(),
            type_history: Vec::new(),
        }
    }

    /// 以묐┰ 愿怨???紐⑤뱺 4異?0, 洹???default.
    /// **?쒓렇?덉쿂 蹂댁〈** (1.8 ?먮룞 ?≪닔 16怨?.
    pub fn neutral(owner: NpcId, target: NpcId) -> Self {
        Self::new(
            owner, target,
            AxisScore::NEUTRAL, AxisScore::NEUTRAL,
            AxisScore::NEUTRAL, WarinessScore::NEUTRAL,
        )
    }

    // ?? Getters ?????
    pub fn owner(&self)        -> &NpcId           { &self.owner }
    pub fn target(&self)       -> &NpcId           { &self.target }
    pub fn trust(&self)        -> AxisScore        { self.trust }
    pub fn affinity(&self)     -> AxisScore        { self.affinity }
    pub fn respect(&self)      -> AxisScore        { self.respect }
    pub fn wariness(&self)     -> WarinessScore    { self.wariness }
    pub fn bond_kind(&self)    -> Option<BondKind> { self.bond_kind }
    pub fn bond_status(&self)  -> &BondStatus      { &self.bond_status }
    pub fn partnership(&self)  -> Option<Partnership> { self.partnership }
    pub fn type_text(&self)    -> &str             { &self.type_text }
    pub fn type_history(&self) -> &[TypeChange]    { &self.type_history }

    /// 4異??쇨큵 蹂??(Stage 2 `update_axes_from_emotion`?먯꽌 ?몄텧).
    /// BondStatus 李⑤떒? ?몄텧 痢?(Stage 2)?먯꽌 泥섎━.
    /// 罹≪뒓??蹂댁〈 ??Relationship???먭린 ?곹깭 蹂寃?梨낆엫 (?섏떖 2 寃곗젙).
    pub fn apply_delta(&mut self, delta: &AxisDelta) {
        self.trust    = self.trust.add(delta.trust);
        self.affinity = self.affinity.add(delta.affinity);
        self.respect  = self.respect.add(delta.respect);
        self.wariness = self.wariness.add(delta.wariness);
    }

    /// 媛먯젙 ?됯? 而⑦뀓?ㅽ듃 modifier (A2??5怨??ъ슜泥?.
    /// ?섎? 蹂댁〈 + ?대쫫 蹂寃?(?섏떖 3 寃곗젙: closeness_* ??affinity_*).
    /// Phase 2.3?먯꽌 ?뺣???(respect_modifier ?좎꽕 ??寃利?.
    pub fn modifiers(&self) -> RelationshipModifiers {
        let a = self.affinity.value() / 100.0;   // -1.0..1.0 ?뺢퇋??(5怨??ъ슜泥??명솚)
        let t = self.trust.value() / 100.0;
        RelationshipModifiers {
            affinity_modifier: a,
            affinity_squared:  a.powi(2),
            affinity_abs:      a.abs(),
            trust_modifier:    t,
        }
    }
}

/// 媛먯젙 ?됯? 而⑦뀓?ㅽ듃??modifier (5怨??ъ슜泥? emotion/stimulus/scene policy, situation_service, memory_repository).
///
/// **Phase 2 蹂寃?*: `closeness_*` ??`affinity_*` ?대쫫 蹂寃?(?섏떖 3 寃곗젙 ??closeness ?먭린 ?뺥빀).
/// Stage 2?먯꽌 5怨??ъ슜泥??대쫫 媛깆떊.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RelationshipModifiers {
    pub affinity_modifier: f32,
    pub affinity_squared:  f32,
    pub affinity_abs:      f32,
    pub trust_modifier:    f32,
}
```

**?ㅺ퀎 ?섎룄 8媛?*:

| # | ??ぉ | ?섎룄 |
|---|---|---|
| ??| 4異?蹂?????ъ슜 (`AxisScore` 횞3 + `WarinessScore`) | 1.2 寃곗젙. wariness ?뚯닔 而댄뙆???쒖젏 李⑤떒. |
| ??| `bond_kind: Option<BondKind>` | None = 誘몃텇瑜?(?遺遺??쒕굹由ъ삤 ?섏뼱 default). 1.3 ?ы띁濡??곸뿭 遺꾧린. |
| ??| `#[serde(default)] bond_status: BondStatus` (Active) | 1.4 寃곗젙. 留덉씠洹몃젅?댁뀡 ??湲곗〈 ?쒕굹由ъ삤 ?먮룞 Active. |
| ??| `partnership: Option<Partnership>` | 1.5 寃곗젙. None = ?뺤떇 愿怨??놁쓬. |
| ??| `type_text: String` + `#[serde(rename = "type")]` | `type`? Rust ?덉빟?????꾨뱶紐낆? `type_text`, JSON ?ㅻ뒗 `type`. B-D4 power ?≪닔. |
| ??| `type_history: Vec<TypeChange>` ?⑥닚 3 ?꾨뱶 | ?섏떖 1 寃곗젙. from/to/note留? ?쒓컙/?먯씤 異붿쟻? 蹂??쒖뒪?? |
| ??| `apply_delta(&mut self, delta)` | ?섏떖 2 寃곗젙. 罹≪뒓??蹂댁〈. Stage 2 ?⑥닔媛 ?몄텧. |
| ??| `RelationshipModifiers` ?대쫫 蹂寃?(`closeness_*` ??`affinity_*`) | ?섏떖 3 寃곗젙. 5怨??ъ슜泥?Stage 2?먯꽌 ?④퍡 媛깆떊. closeness ?먭린 ?뺥빀. |

**`neutral()` ?쒓렇?덉쿂 蹂댁〈** (1.8 ?먮룞 ?≪닔 ?듭떖):

```rust
// ?꾩옱: Relationship::neutral(owner, target) -> Relationship
// Phase 2: Relationship::neutral(owner, target) -> Relationship  ???숈씪
```

??16怨??몄텧 蹂寃?0. (1.8?먯꽌 grep 寃利?

**湲곗〈 硫붿꽌???먭린**:
- `Relationship::after_dialogue` ??Stage 2 `update_axes_from_emotion`?쇰줈 ?닿? (Stage 3?먯꽌 `relationship_policy.rs` ?ъ슜泥?媛깆떊)
- `Relationship::with_updated_closeness` ??Stage 2??base_delta + apply_delta ?⑦꽩?쇰줈 ?≪닔
- `Relationship::closeness()` / `power()` 硫붿꽌?????꾩쟾 ?쒓굅
- `Relationship::with_power` ???꾩쟾 ?쒓굅 (?몄텧泥?0嫄? A2 諛쒓껄)

**湲곗〈 硫붿꽌??蹂댁〈**:
- `new` / `neutral` / `owner` / `target` / `trust` / `modifiers` ???쒓렇?덉쿂/?쒕㎤??蹂댁〈 (??`modifiers()` 諛섑솚 ????꾨뱶 ?대쫫留?蹂寃?

**?⑥쐞 ?뚯뒪??耳?댁뒪** (1.9?먯꽌 援ы쁽):

```
[new + getter]
- let r = Relationship::new(npc_a, npc_b, AxisScore::new(50), AxisScore::new(40),
                            AxisScore::new(30), WarinessScore::new(20));
  r.trust().value()    == 50.0
  r.affinity().value() == 40.0
  r.respect().value()  == 30.0
  r.wariness().value() == 20.0
  r.bond_kind()        == None
  r.bond_status()      == &BondStatus::Active   (default)
  r.partnership()      == None
  r.type_text()        == ""
  r.type_history()     == &[]

[neutral - ?쒓렇?덉쿂 蹂댁〈]
- Relationship::neutral(npc_a, npc_b) 
  ??4異?紐⑤몢 0, bond_kind None, status Active, partnership None, type "", history []

[apply_delta - 4異??쇨큵 蹂??
- let mut r = Relationship::neutral(a, b);  // 紐⑤몢 0
  r.apply_delta(&AxisDelta { trust: 20.0, affinity: 10.0, respect: 5.0, wariness: 15.0 });
  r.trust().value()    == 20.0
  r.affinity().value() == 10.0
  r.respect().value()  == 5.0
  r.wariness().value() == 15.0

[apply_delta clamp ?숈옉]
- let mut r = Relationship::new(a, b, AxisScore::new(90), AxisScore::NEUTRAL, AxisScore::NEUTRAL, WarinessScore::new(5));
  r.apply_delta(&AxisDelta { trust: 30.0, affinity: 0.0, respect: 0.0, wariness: -20.0 });
  r.trust().value()    == 100.0  (cap)
  r.wariness().value() == 0.0    (floor)

[modifiers - ?대쫫 蹂寃?+ ?뺢퇋??
- let r = Relationship::new(a, b, AxisScore::new(50), AxisScore::new(80), 
                            AxisScore::NEUTRAL, WarinessScore::NEUTRAL);
  let m = r.modifiers();
  m.affinity_modifier == 0.8     (80 / 100)
  m.affinity_squared  == 0.64
  m.affinity_abs      == 0.8
  m.trust_modifier    == 0.5

[serde round-trip]
- Relationship ??JSON ??Relationship (紐⑤뱺 ?꾨뱶 蹂댁〈, type ?꾨뱶??JSON ??"type")
- bond_status ?꾨씫??JSON ??default Active ?먮룞 ?곸슜
- type_history ?꾨씫??JSON ??default [] ?먮룞 ?곸슜

[TypeChange]
- TypeChange { from_type: "議곗젙 ?숇즺".into(), to_type: "泥섎떒 ???.into(), note: "?곗떊臾??ш굔".into() }
  ??serde round-trip OK
```

**鍮꾪룷??*:
- `Relationship::after_dialogue` ??Stage 2/3?먯꽌 泥섎━ (?ъ옉???먮뒗 ?먭린)
- `bond_status` ?꾩씠 ?⑥닔 ??Phase 3a Channel 2 Temporal
- `partnership` ?꾩씠 ?⑥닔 ??Phase 2.5 declarative_events `PartnershipChange`
- type_history ?먮룞 append ?몃뱾????Phase 2.5 declarative_events `TypeChanged`
- `Display` impl ??presentation layer
- 5怨?modifier ?ъ슜泥?媛깆떊 ??Stage 2 (`closeness_*` ??`affinity_*` ?대쫫 蹂寃?
- `with_power` 硫붿꽌?????꾩쟾 ?쒓굅 (?몄텧泥?0嫄?

#### 1.7 ??`RelationshipBuilder` 4異?API

**紐⑹쟻**: ?쒕굹由ъ삤 JSON ?뚯떛 + Mind Studio CRUD?먯꽌 ?ъ슜?섎뒗 *fluent builder*. 4異뺤쑝濡?蹂寃? ???꾨뱶 (bond_kind/bond_status/partnership/type) ?듭뀡 setter 異붽?.

**?꾩튂**: `src/domain/relationship/mod.rs` (1.6 `Relationship`怨?媛숈? ?뚯씪)

**?쒓렇?덉쿂**:

```rust
//! RelationshipBuilder ??fluent API.
//! ?ъ슜泥?
//! - `adapter/memory_repository.rs:195` (?쒕굹由ъ삤 JSON ?뚯떛)
//! - `bin/mind-studio/state.rs:797` (UI CRUD)
//! - ?⑥쐞 ?뚯뒪??~100 ?몄텧

#[derive(Debug, Clone)]
pub struct RelationshipBuilder {
    owner: NpcId,
    target: NpcId,

    // 4異?default = NEUTRAL
    trust:    AxisScore,
    affinity: AxisScore,
    respect:  AxisScore,
    wariness: WarinessScore,

    // ???꾨뱶 default
    bond_kind:    Option<BondKind>,
    bond_status:  BondStatus,
    partnership:  Option<Partnership>,
    type_text:    String,
    type_history: Vec<TypeChange>,
}

impl RelationshipBuilder {
    /// ??builder. 紐⑤뱺 4異?NEUTRAL, ???꾨뱶 default (BondStatus::Active ??None/鍮?.
    pub fn new(owner_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            owner:  NpcId::new(owner_id.into()),
            target: NpcId::new(target_id.into()),
            trust:    AxisScore::NEUTRAL,
            affinity: AxisScore::NEUTRAL,
            respect:  AxisScore::NEUTRAL,
            wariness: WarinessScore::NEUTRAL,
            bond_kind:    None,
            bond_status:  BondStatus::Active,
            partnership:  None,
            type_text:    String::new(),
            type_history: Vec::new(),
        }
    }

    // ?? 4異?setter ?????
    pub fn trust(mut self, value: AxisScore) -> Self {
        self.trust = value;
        self
    }
    pub fn affinity(mut self, value: AxisScore) -> Self {
        self.affinity = value;
        self
    }
    pub fn respect(mut self, value: AxisScore) -> Self {
        self.respect = value;
        self
    }
    pub fn wariness(mut self, value: WarinessScore) -> Self {
        self.wariness = value;
        self
    }

    // ?? ???꾨뱶 setter ?????
    /// bond_kind setter ???섏떖 1 寃곗젙 (A): setter ?덉뿉??Some ?섑븨.
    /// None? *setter 誘명샇異?濡??쒗쁽.
    pub fn bond_kind(mut self, value: BondKind) -> Self {
        self.bond_kind = Some(value);
        self
    }
    pub fn bond_status(mut self, value: BondStatus) -> Self {
        self.bond_status = value;
        self
    }
    /// partnership setter ???숈씪 ?⑦꽩 (Option ?섑븨).
    pub fn partnership(mut self, value: Partnership) -> Self {
        self.partnership = Some(value);
        self
    }
    pub fn type_text(mut self, value: impl Into<String>) -> Self {
        self.type_text = value.into();
        self
    }
    /// type_history setter ???섏떖 2 寃곗젙 (X): ?꾩껜 援먯껜.
    /// append??Phase 2.5 declarative_events `TypeChanged` ?몃뱾?ъ뿉??蹂꾨룄.
    pub fn type_history(mut self, value: Vec<TypeChange>) -> Self {
        self.type_history = value;
        self
    }

    /// 鍮뚮뱶 ??Relationship ?몄뒪?댁뒪 ?앹꽦.
    /// 媛숈? 紐⑤뱢?대?濡?private ?꾨뱶 吏곸젒 packing 媛??
    pub fn build(self) -> Relationship {
        Relationship {
            owner:        self.owner,
            target:       self.target,
            trust:        self.trust,
            affinity:     self.affinity,
            respect:      self.respect,
            wariness:     self.wariness,
            bond_kind:    self.bond_kind,
            bond_status:  self.bond_status,
            partnership:  self.partnership,
            type_text:    self.type_text,
            type_history: self.type_history,
        }
    }
}
```

**?ㅺ퀎 ?섎룄 5媛?*:

| # | ??ぉ | ?섎룄 |
|---|---|---|
| ??| 4異?setter (4媛? ??湲곗〈 `.closeness()` / `.power()` ?쒓굅 + `.affinity()`/`.respect()`/`.wariness()` ?좎꽕 | ?쒕굹由ъ삤 JSON + Mind Studio CRUD 蹂寃?硫댁쟻. Stage 4 留덉씠洹몃젅?댁뀡 ?꾧뎄媛 ?먮룞 蹂?? |
| ??| `bond_kind(BondKind)` setter ??Option ?섑븨 setter ?대? (?섏떖 1 寃곗젙 A) | ?붿옄?대꼫 移쒗솕 ??`.bond_kind(BondKind::SwornBrothers)` 吏곴?. None? setter 誘명샇異쒕줈 ?쒗쁽. |
| ??| `partnership(Partnership)` ?숈씪 ?⑦꽩 | None 泥섎━ ?숈씪. |
| ??| `type_text` setter??`impl Into<String>` | `.type_text("?섑삎??)` literal ?먯뿰. `String::from()` ?몄텧 遺덊븘?? |
| ??| `type_history(Vec<TypeChange>)` ?꾩껜 援먯껜 setter (?섏떖 2 寃곗젙 X) | ?붿옄?대꼫媛 ?쒕굹由ъ삤 JSON??*?꾩껜 history*瑜?諛뺣뒗 ?⑦꽩. append??Phase 2.5?먯꽌 ?먮룞. |

**`.build()` 吏곸젒 ?꾨뱶 packing**:
- ?꾩옱: `Relationship::new(self.owner_id, ...)` ?몄텧 ???대??먯꽌 ?ㅼ떆 packing
- Phase 2: *吏곸젒 ?꾨뱶 梨꾩?* ??`mod.rs` 媛숈? 紐⑤뱢?대?濡?*private ?꾨뱶 ?묎렐 媛??
- ?대윭硫?Builder??*紐⑤뱺 ?꾨뱶 吏곸젒 ?쒖뼱* (bond_kind/type ??`Relationship::new` ?쒓렇?덉쿂???녿뒗 ?꾨뱶 諛뺢린 媛??

**湲곗〈 ?몄텧泥??곹뼢**:

| ?꾩튂 | 蹂寃?|
|---|---|
| `adapter/memory_repository.rs:195` | `.closeness(s).trust(s).power(s)` ??`.trust(s).affinity(s).respect(s).wariness(s)` + ???꾨뱶 setter ??Stage 4 留덉씠洹몃젅?댁뀡 ?꾧뎄媛 ?먮룞 蹂??|
| `bin/mind-studio/state.rs:797` | UI?먯꽌 愿怨??섎룞 ?앹꽦 ??Stage 3?먯꽌 Mind Studio frontend? ?④퍡 媛깆떊 |
| ?뚯뒪??~100 ?몄텧 | `.closeness(s).trust(s).power(s)` ?⑦꽩 ??Stage 4 ?먮룞 留덉씠洹몃젅?댁뀡 ?ㅽ겕由쏀듃濡?蹂??|

**?⑥쐞 ?뚯뒪??耳?댁뒪** (1.9?먯꽌 援ы쁽):

```
[湲곕낯 ?ъ슜 ??4異?setter]
- RelationshipBuilder::new("a", "b")
    .trust(AxisScore::new(50.0))
    .affinity(AxisScore::new(40.0))
    .respect(AxisScore::new(30.0))
    .wariness(WarinessScore::new(20.0))
    .build()
  ??4異?紐⑤몢 ?뺥솗 + bond_kind None + status Active + type_text "" + type_history []

[partial ?ъ슜 ???쇰? setter留?
- RelationshipBuilder::new("a", "b").trust(AxisScore::new(50.0)).build()
  ??trust 50, ?섎㉧吏 axes NEUTRAL, ???꾨뱶 default

[bond_kind setter ??Option ?섑븨]
- RelationshipBuilder::new("a", "b")
    .bond_kind(BondKind::SwornBrothers)
    .build()
  ??bond_kind == Some(SwornBrothers)

[partnership setter]
- RelationshipBuilder::new("a", "b")
    .partnership(Partnership::Spouse)
    .build()
  ??partnership == Some(Spouse)

[type_text + Into<String>]
- RelationshipBuilder::new("a", "b").type_text("?섑삎??).build()
  ??type_text == "?섑삎??

[type_history ?꾩껜 援먯껜]
- let history = vec![TypeChange { from_type: "?숇즺".into(), to_type: "?먯닔".into(), note: "?곗떊臾?.into() }];
  RelationshipBuilder::new("a", "b").type_history(history.clone()).build()
  ??type_history == history

[Builder fluent chain ??紐⑤뱺 ?꾨뱶]
- RelationshipBuilder::new("a", "b")
    .trust(AxisScore::new(50.0))
    .affinity(AxisScore::new(60.0))
    .respect(AxisScore::new(40.0))
    .wariness(WarinessScore::new(10.0))
    .bond_kind(BondKind::SwornBrothers)
    .bond_status(BondStatus::Active)
    .partnership(Partnership::Lover)
    .type_text("?섑삎?쒖씠???곗씤")
    .build()
  ??紐⑤뱺 ?꾨뱶 ?뺥솗
```

**鍮꾪룷??*:
- `bond_kind_none()` / `partnership_none()` 紐낆떆 setter ??誘명샇異쒖씠 None?대?濡?遺덊븘??
- `with_type_change(change)` append 硫붿꽌????Phase 2.5?먯꽌 `TypeChanged` ?몃뱾???먯껜
- ?⑥닚 wrapper 硫붿꽌????YAGNI

#### 1.8 ??`Relationship::neutral()` ?먮룞 ?≪닔 寃利?

**紐⑹쟻**: Stage 1 ?꾨찓???ъ옉????`Relationship::neutral(owner, target) -> Relationship` ?쒓렇?덉쿂媛 *洹몃?濡?蹂댁〈*?섎?濡?22怨??몄텧泥?*蹂寃?0* ??grep?쇰줈 寃利?

##### ?몄텧泥?22 ?꾩튂 (?뚯씪蹂?吏묎퀎)

| ?뚯씪 | ?몄텧 ??| 鍮꾧퀬 |
|---|---|---|
| `domain/relationship.rs` | 3 | ?먯껜 ?⑥쐞 ?뚯뒪????Phase 2?먯꽌 *???⑥쐞 ?뚯뒪?몃줈 援먯껜* (1.9) |
| `application/command/telling_ingestion_handler.rs` | 3 | ?뚯뒪??+ production |
| `application/command/policies/emotion_policy.rs` | 1 | ?⑥쐞 ?뚯뒪??|
| `application/command/policies/guide_policy.rs` | 2 | ?⑥쐞 ?뚯뒪??|
| `application/command/policies/relationship_policy.rs` | 6 | ?⑥쐞 ?뚯뒪??|
| `application/command/policies/scene_policy.rs` | 2 | ?⑥쐞 ?뚯뒪??|
| `application/command/policies/stimulus_policy.rs` | 5 | ?⑥쐞 ?뚯뒪??|
| **?⑷퀎** | **22** | |

??`domain/relationship.rs:324~377` 3媛쒕뒗 Phase 2?먯꽌 ?먯껜 ?뚯뒪??援먯껜. ?섎㉧吏 **19怨?*? *?쒓렇?덉쿂 蹂댁〈留뚯쑝濡??먮룞 ?≪닔*.

##### ?먮룞 ?≪닔 議곌굔

| 議곌굔 | 留뚯” |
|---|---|
| ??`Relationship::neutral(impl Into<String>, impl Into<String>) -> Relationship` ?쒓렇?덉쿂 蹂댁〈 | ??(1.6 諛뺥옒) |
| ??諛섑솚 ???`Relationship` 蹂댁〈 | ??|
| ???몄텧 ??*3異?硫붿꽌??(.closeness/.power) ?몄텧 ?놁쓬* | 22怨?寃利??꾩슂 |

議곌굔 ??? *?꾩냽 肄붾뱶 寃??媛 ?꾩슂. **媛쒕퀎 寃?????cargo check濡??쇨큵 寃利?* ??而댄뙆???먮윭媛 *3異??꾩냽 ?몄텧 ?꾩튂*瑜?*?먮룞 ?앸퀎*.

##### 蹂꾨룄 蹂寃?硫댁쟻 (1.8 鍮꾪룷?? Stage 2/3?먯꽌)

**3異??ъ슜 ?꾩냽 ?몄텧 移댄깉濡쒓렇**:

| ?⑦꽩 | ?꾩튂 ??| 泥섎━ stage |
|---|---|---|
| `.closeness()` / `.power()` ?몄텧 | **14** | Stage 3 ??紐⑤몢 ?쒓굅 (?꾨뱶 ?먯껜 ?먭린) |
| `with_updated_closeness` 硫붿꽌??+ ?몄텧 | 4 (?뺤쓽 1 + ?몄텧 3) | Stage 2 ??`update_axes_from_emotion`?쇰줈 ?닿? |
| `Relationship::after_dialogue` 硫붿꽌??+ ?몄텧 | 4 (?뺤쓽 1 + ?몄텧 3) | Stage 2/3 ???닿? + ?먭린 |
| `with_power` 硫붿꽌??+ ?몄텧 | 1 (?뺤쓽留? ?몄텧 0) | Stage 1.6?먯꽌 *?꾩쟾 ?쒓굅* (A2 諛쒓껄) |

**?꾩튂 ?곸꽭**:
- `.closeness()`/`.power()`: `dialogue_orchestrator.rs:836,838`, `relationship_policy.rs:136,138,141,143,217,219,222,224`, `domain_sync.rs:68,70`, `guide/snapshot.rs:313,315`
- `with_updated_closeness` ?몄텧: `domain/relationship.rs:191` ?대? 1??
- `Relationship::after_dialogue` ?몄텧: `relationship_policy.rs:134,215`, `stimulus_policy.rs:71`

##### ?좑툘 紐낆묶 異⑸룎 ?명듃 (Stage 2/3 吏꾩엯 ???뚯븘??寃?

`after_dialogue` 紐낆묶??**??媛쒕뀗**???곗뿬 ?덉쓬:

1. **`Relationship::after_dialogue` 硫붿꽌??(?꾨찓??** ??Phase 2 ?먭린 ??? ?몄텧 3怨?
2. **`after_dialogue` ?꾨뱶/?붾뱶?ъ씤??(Mind Studio + DTO)** ??*?????泥섎━ ?꾩껜 ?먮쫫*. **Phase 2 蹂寃?臾닿?**.

Stage 2/3?먯꽌 (1)留??닿?/?먭린, (2)??洹몃?濡??좎?. 50+ ?꾩튂??(2)??*Phase 2 硫댁쟻 ?꾨떂*.

##### 寃利?紐낅졊 (Stage 1 醫낃껐 ???ㅽ뻾)

```powershell
# (1) Relationship::neutral ?몄텧 ???뺤씤 ??22 ?좎?
(Get-ChildItem -Path "src" -Recurse -Filter "*.rs" |
  Select-String -Pattern "Relationship::neutral").Count    # ??22

# (2) cargo check ??而댄뙆???먮윭 ?꾩튂媛 *?꾩냽 axes ?몄텧 ?꾩튂* ?앸퀎
cargo check --all-features 2>&1 | Tee-Object -FilePath "baselines\stage1-cargo-check.log"

# (3) ?먮룞 ?≪닔 寃利? 22怨?以?而댄뙆???먮윭 ?꾩튂媛 *3異??ъ슜 ?꾩냽 ?몄텧*怨쇰쭔 ?쇱튂?섎뒗吏 ?뺤씤
# (?덉긽: relationship_policy/stimulus_policy??.closeness/.power/.after_dialogue ?꾩튂留?
```

##### 鍮꾪룷??

- 3異??꾩냽 ?몄텧 媛깆떊 ??Stage 2 (`modifiers()` `closeness_*` ??`affinity_*` ?대쫫 蹂寃? + Stage 3 (`relationship_policy.rs` ?ъ옉?? `dialogue_orchestrator.rs` 4異?DTO, `domain_sync.rs` 4異?DTO, `guide/snapshot.rs` 4異??쒖떆)
- `Relationship::after_dialogue` 硫붿꽌???닿? ??Stage 2 (`update_axes_from_emotion`?쇰줈 ?泥?
- Mind Studio `perform_after_dialogue` ??50+ ?꾩튂 ??Phase 2 硫댁쟻 ??(紐낆묶 異⑸룎留? ?섎? 蹂?

##### Stage 1.8 醫낃껐 寃뚯씠??

1. `Relationship::neutral` ?몄텧 22怨?grep 寃곌낵 蹂댁〈 (`baselines/stage1-neutral-callsites.log`)
2. `cargo check` 而댄뙆???먮윭 ?꾩튂媛 *?덉긽 3異??ъ슜 ?꾩튂* (14 + 3 = 17 + ?꾨찓??3 = ~20)? ?쇱튂
3. 22怨?以?*?덉긽 ??而댄뙆???먮윭* 0嫄?(?쒓렇?덉쿂 蹂댁〈 ?ㅽ뙣 0)

#### 1.9 ??Stage 1 ?⑥쐞 ?뚯뒪??

**紐⑹쟻**: 1.2~1.8?먯꽌 諛뺤? *遺덈???+ 蹂??+ ?쒓렇?덉쿂 蹂댁〈*???⑥쐞 ?뚯뒪?몃줈 寃利? Stage 1 醫낃껐 寃뚯씠??

##### ?뚯뒪???꾩튂 ??*紐⑤뱢 ?대? ?⑦꽩* (?꾩옱 肄붾뱶 ?쇨?)

```
src/domain/relationship/
  mod.rs           # Relationship + RelationshipBuilder + TypeChange tests
    ?붴?? #[cfg(test)] mod tests { ... }
  axis.rs          # AxisScore + WarinessScore + AxisDelta tests
    ?붴?? #[cfg(test)] mod tests { ... }
  bond.rs          # BondKind + BondStatus tests
    ?붴?? #[cfg(test)] mod tests { ... }
  partnership.rs   # Partnership tests
    ?붴?? #[cfg(test)] mod tests { ... }
```

洹쇨굅: ?꾩옱 `domain/relationship.rs:323~` ?꾩튂??`#[cfg(test)] mod tests` 諛뺥엺 ?⑦꽩. Phase 1 ?쇨?.

##### ?뚯씪蹂??뚯뒪??移댁슫??(異붿젙)

| ?뚯씪 | 耳?댁뒪 ?곸뿭 | 異붿젙 移댁슫??|
|---|---|---|
| `axis.rs` | clamp 6 + add 4 + Default/NEUTRAL 4 + AxisDelta scaled_by 2 + AxisDelta Add 2 + serde 2 | **~12** (臾띠쓬) |
| `bond.rs` | BondKind ?곸뿭 ?ы띁 6 + ?곹샇 諛고???1 + serde 2 / BondStatus accepts_live_input 5 + Default 1 + serde 5 | **~10** |
| `partnership.rs` | variants 1 + serde 4 + Copy/Eq/Hash 3 | **~4** |
| `mod.rs` | Relationship new/neutral 2 + apply_delta 2 + modifiers 1 + serde 3 + TypeChange 1 + Builder chain 7 | **~12** |
| **?⑷퀎** | | **~38** |

??Stage 1 ?좉퇋 ?⑥쐞 ?뚯뒪??**~38媛?*. baseline 1220 ??Stage 1 醫낃껐 ??~1258 (?⑥닚 ??.

(?ㅼ젣 移댁슫?몃뒗 Stage 1 援ы쁽 ???뺥솗 ??*baseline log* 諛뺥옒. ??38? *理쒖냼 湲곗?*.)

##### 1.8 ?먮룞 ?≪닔 19怨녹쓽 湲곗〈 ?뚯뒪??蹂댁〈

- `policies/*_test.rs` 22怨?以?19怨?(`domain/relationship.rs` 3怨??쒖쇅)? *湲곗〈 ?뚯뒪??洹몃?濡?. *?쒓렇?덉쿂 蹂댁〈*留뚯쑝濡??듦낵.
- 而댄뙆???먮윭媛 *3異??꾩냽 ?몄텧 ?꾩튂*留??앸퀎 ??Stage 2/3?먯꽌 媛깆떊.

##### Stage 1 醫낃껐 寃뚯씠??(1.1~1.9 紐⑤몢 ?듦낵 ??

| # | 寃뚯씠??| 寃利?|
|---|---|---|
| 1 | `cargo check --all-features` ?듦낵 | 1.2~1.7 ???而댄뙆??|
| 2 | **`Relationship::neutral()` ?몄텧 22怨?*?덉긽 ??而댄뙆???먮윭 0*** | 1.8 ?먮룞 ?≪닔 寃利?|
| 3 | `WarinessScore::new(-50.0)` 而댄뙆??李⑤떒 寃利?| 1.2 ????Rust 而댄뙆?쇰윭 ?먮룞 |
| 4 | `cargo test --all-features --workspace` ?듦낵 | Stage 1 ?좉퇋 ~38媛?+ 湲곗〈 1220 = ~1258 |
| 5 | Baseline log 諛뺤젣 ??`baselines/stage1-cargo-test-2026-MM-DD-PASS.log` | Stage 2 吏꾩엯 吏곸쟾 |

##### Stage 1 ?곗텧 commit + ?뚭퀬

```
commit: phase2-stage1-domain.md ?뚭퀬
?뚯씪: docs/tasks/mind-architecture/phase2-stage1-domain.md
?댁슜:
- Stage 1 1.1~1.9 ?묒뾽 ?댁뿭
- 理쒖쥌 ?뚯뒪??移댁슫??(?? 1258)
- ?먮룞 ?≪닔 19怨??뺤씤
- Stage 2 吏꾩엯 ?꾩젣 (紐⑤뱢 遺꾪븷 ?꾨즺, 4異?????덉젙)
- 諛쒓껄 ?ы빆 (?덈떎硫?
```

##### 鍮꾪룷??

- ?듯빀 ?뚯뒪??(cross-module) ??Stage 5 narrative ?쒕??덉씠??
- `update_axes_from_emotion` ?곸슜 ??4異?蹂??寃利???Stage 2/5
- `RelationshipUpdatedPayload` 6?? schema 寃利???Stage 3
- ?쒕굹由ъ삤 JSON 留덉씠洹몃젅?댁뀡 寃利???Stage 4/5
- Mind Studio frontend 4異??쒖떆 寃利???Stage 3

---

**Stage 1 醫낇빀 寃뚯씠??* (1.1~1.9 紐⑤몢 ?듦낵 ??:
1. `cargo check --all-features` ?듦낵
2. `Relationship::neutral()` ?몄텧 16怨??먮룞 ?≪닔 (蹂寃?0)
3. `WarinessScore::new(-50.0)` 而댄뙆??李⑤떒 ?뺤씤 (遺덈???媛뺤젣)
4. ?⑥쐞 ?뚯뒪???듦낵
5. Stage 1 baseline `baselines/cargo-test-2026-05-14-PASS.log` 1220 tests ?듦낵 ?좎?

**?곗텧 commit**: `phase2-stage1-domain.md` ?뚭퀬

---

### Stage 2 — OCC → 4축 매핑 (base_delta + HEXACO + Updater)

**범위 (상위 골격)**:
- `AxisDelta` (Stage 1.2 박힘) + 신설 `AxisModifier` (2.1)
- `base_delta(OccEmotion) -> AxisDelta` 48셀 lookup (v0.7 §4.2, B-D6 D6-a) — 12 OCC × 4축, Well-being/Prospect 10 OCC는 0 (B-D14)
- `hexaco_modifier(OccEmotion, &Hexaco) -> AxisModifier` 6 보정 룰 (v0.7 §4.3)
- `update_axes_from_emotion(rel, emotion, intensity, hexaco)` 단일 함수 (B-D5)
- BondStatus 차단 + Shame/Pride 변동 0 (B-D12)
- `RelationshipModifiers` 4축 이름 변경 (`closeness_*` → `affinity_*`) + 5곳 사용처 갱신
- 기존 `Relationship::after_dialogue` 호출 3곳 → `update_axes_from_emotion` 이관
- 단위 테스트 (S1~S4 ground truth + Compound 감정 검증)

세부 항목 2.1~2.7:

#### 2.1 — `mapping.rs` 모듈 신설 + 디렉토리 구조 확장

**목적**: OCC → 4축 매핑 함수 (base_delta + hexaco_modifier + update_axes_from_emotion)의 위치. 1.1의 모듈 분할 패턴 확장.

**위치**: `src/domain/relationship/mapping.rs` (신설)

##### 디렉토리 구조 (Stage 2 후)

```
src/domain/relationship/
  mod.rs              # Relationship + Builder + TypeChange + re-export
  axis.rs             # AxisScore + WarinessScore + AxisDelta + AxisKind
                      # + ★ AxisModifier (추가, 의심 1 결정 A)
  bond.rs             # BondKind + BondStatus
  partnership.rs      # Partnership
  mapping.rs          # ★ 신설 — base_delta + hexaco_modifier + update_axes_from_emotion
```

##### mod.rs 변경

```rust
// 기존 (Stage 1)
mod axis;
mod bond;
mod partnership;

// Stage 2 추가
mod mapping;

pub use mapping::update_axes_from_emotion;
// base_delta / hexaco_modifier는 *내부 API* (pub(crate) 또는 mod 내부)
```

##### Public API 노출 패턴

| 함수/타입 | 가시성 | 이유 |
|---|---|---|
| `update_axes_from_emotion(...)` | `pub` | Stage 3 `relationship_policy.rs`가 호출 (외부 진입점) |
| `base_delta(emotion) -> AxisDelta` | `pub(crate)` 또는 private | 내부 헬퍼 — 외부 노출 불필요 |
| `hexaco_modifier(emotion, hexaco) -> AxisModifier` | `pub(crate)` 또는 private | 내부 헬퍼 |
| `AxisModifier` 타입 | `pub` (axis.rs) | 의심 1 결정 A — 데이터 모듈 일관성 |

##### `AxisModifier` 신설 위치 — `axis.rs` (의심 1 결정 A)

Stage 1.2의 *데이터 타입은 axis.rs* 패턴 정합. AxisDelta와 인접 — 의미 비교 명확:
- AxisDelta = 변동량 (+/-)
- AxisModifier = 배수 (×)

```rust
// axis.rs 추가 (2.1 시점에 박음, 2.3에서 메서드 채움)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisModifier {
    pub trust:    f32,   // 곱셈 배수 (default 1.0)
    pub affinity: f32,
    pub respect:  f32,
    pub wariness: f32,
}

impl Default for AxisModifier {
    fn default() -> Self {
        Self { trust: 1.0, affinity: 1.0, respect: 1.0, wariness: 1.0 }
    }
}
```

##### 작업 순서 (Stage 2)

```
2.1 모듈/디렉토리 (현재 항목)
  ↓
2.2 base_delta (병렬 가능)  ─┐
2.3 hexaco_modifier         ─┤
                             ↓
2.4 update_axes_from_emotion 통합
  ↓
2.5 RelationshipModifiers 이름 변경 + 5곳 사용처 갱신
  ↓
2.6 Relationship::after_dialogue 후속 호출 이관
  ↓
2.7 Stage 2 단위 테스트
```

##### 게이트

`cargo check --all-features` 통과 (디렉토리 분할 후 컴파일 안전).

##### 비포함

- base_delta 48셀 표 — 2.2
- HEXACO 보정 룰 본체 — 2.3
- `update_axes_from_emotion` 본체 — 2.4
- AxisModifier의 헬퍼 메서드 (`scale`, `combine` 등) — 2.3에서 필요해지면 추가

#### 2.2 — `base_delta(OccEmotion) -> AxisDelta` 48셀 lookup

**목적**: OCC 감정 → 4축 변동 *순수 매핑 함수*. v0.7 §4.2 표 그대로 박음. intensity/HEXACO 곱셈 *전*의 *base 변동*.

**위치**: `src/domain/relationship/mapping.rs`

**시그니처**:

```rust
//! mapping.rs — OCC → 4축 매핑.
//! v0.7 §4.1~4.3 + B-D6 D6-a 채택.

use crate::domain::emotion::OccEmotion;
use crate::domain::relationship::axis::AxisDelta;

/// OCC 감정 → 4축 base 변동 (intensity/HEXACO 곱셈 *전*).
///
/// v0.7 §4.2 표 — 12 OCC × 4축 = 48셀.
/// Well-being (Joy/Distress) + Prospect (Hope/Fear/Satisfaction/Disappointment/Relief/FearsConfirmed)
/// + Compound 보조 (Remorse/Gratification) **10 OCC는 default (0)** — B-D14 확정.
///
/// 매핑 안된 감정 입력 시: AxisDelta::default() (모두 0) 반환 — *조용한 fallback*.
pub(crate) fn base_delta(emotion: OccEmotion) -> AxisDelta {
    match emotion {
        // ── 지각·평가 4 (대상 외부) ─────
        OccEmotion::Gratitude   => AxisDelta { trust:  20.0, affinity:  10.0, respect:   0.0, wariness: -10.0 },
        OccEmotion::Anger       => AxisDelta { trust: -25.0, affinity: -10.0, respect:   0.0, wariness:  25.0 },
        OccEmotion::Admiration  => AxisDelta { trust:   0.0, affinity:   0.0, respect:  20.0, wariness:   0.0 },
        OccEmotion::Reproach    => AxisDelta { trust: -10.0, affinity: -10.0, respect: -25.0, wariness:  10.0 },

        // ── 공감 4 (Fortune-of-others) ─────
        OccEmotion::HappyFor    => AxisDelta { trust:   5.0, affinity:  10.0, respect:   0.0, wariness:   0.0 },
        OccEmotion::Resentment  => AxisDelta { trust:   0.0, affinity: -10.0, respect:  -5.0, wariness:  15.0 },
        OccEmotion::Pity        => AxisDelta { trust:   0.0, affinity:  10.0, respect:  -5.0, wariness:   0.0 },
        OccEmotion::Gloating    => AxisDelta { trust: -10.0, affinity: -20.0, respect: -10.0, wariness:   0.0 },

        // ── 자기 평가 2 (B-D12: agent_id=None 시 4축 변동 0, 호출 측에서 처리) ─────
        OccEmotion::Pride       => AxisDelta { trust:   0.0, affinity:   5.0, respect:  10.0, wariness:   0.0 },
        OccEmotion::Shame       => AxisDelta { trust:  -5.0, affinity: -10.0, respect: -10.0, wariness:   5.0 },

        // ── 대상 평가 2 (Object) ─────
        OccEmotion::Love        => AxisDelta { trust:   5.0, affinity:  20.0, respect:   5.0, wariness:  -5.0 },
        OccEmotion::Hate        => AxisDelta { trust: -10.0, affinity: -25.0, respect:  -5.0, wariness:  15.0 },

        // ── 매핑 안된 10 OCC (B-D14 의도된 누락) ─────
        // Joy / Distress (Well-being)
        // Hope / Fear / Satisfaction / Disappointment / Relief / FearsConfirmed (Prospect)
        // Remorse / Gratification (Compound — Pride/Shame 합산은 별 OCC로 자동 식별, base_delta 불필요)
        _ => AxisDelta::default(),
    }
}
```

**설계 의도 5개**:

| # | 항목 | 의도 |
|---|---|---|
| ① | `match` 표현식 (HashMap/const 배열 X) | 컴파일 시점 *exhaustive* 검증 — 새 OCC 추가 시 컴파일 경고. lookup 비용 O(1). |
| ② | 12 OCC 명시 + `_ => Default` fallback | v0.7 §4.2 표 12행 그대로. Well-being/Prospect/일부 Compound 10개는 *조용히 0* — B-D14 의도된 누락. |
| ③ | `pub(crate)` 가시성 | mapping 모듈 내부 헬퍼. 외부 직접 호출 차단. 외부 진입점은 `update_axes_from_emotion` (2.4). |
| ④ | Pride/Shame 표값 박음 (≠ 0) | base_delta는 *순수 lookup* — 표 그대로. **B-D12 (agent_id=None 시 4축 변동 0) 처리는 *호출 측* (2.4 `update_axes_from_emotion`)**. base_delta 자체는 *데이터*. |
| ⑤ | 영역 코멘트 4그룹 (지각/공감/자기/대상) | OCC 22 분류와 정합. 유지보수 시 *왜 이 값인지* 추적 가능. |

**계산 검증** (Stage 0 §3.6 S2 케이스):

```
산신묘 사건 — Anger + Hate + Reproach 합산:
  Anger:    AxisDelta { trust: -25, affinity: -10, respect:   0, wariness:  25 }
  Hate:     AxisDelta { trust: -10, affinity: -25, respect:  -5, wariness:  15 }
  Reproach: AxisDelta { trust: -10, affinity: -10, respect: -25, wariness:  10 }
  합산:     AxisDelta { trust: -45, affinity: -45, respect: -30, wariness:  50 }
  × intensity 0.95/0.6/0.7 평균 적용 → S2 Step 4 결과
```

(intensity 적용은 2.4. base_delta는 *순수 표값*만.)

**단위 테스트 케이스** (2.7에서 구현):

```
[12 OCC base_delta 값 검증]
- base_delta(Gratitude)  == AxisDelta { trust:  20.0, affinity:  10.0, respect:   0.0, wariness: -10.0 }
- base_delta(Anger)      == AxisDelta { trust: -25.0, affinity: -10.0, respect:   0.0, wariness:  25.0 }
- base_delta(Admiration) == AxisDelta { trust:   0.0, affinity:   0.0, respect:  20.0, wariness:   0.0 }
- base_delta(Reproach)   == AxisDelta { trust: -10.0, affinity: -10.0, respect: -25.0, wariness:  10.0 }
- base_delta(HappyFor)   == AxisDelta { trust:   5.0, affinity:  10.0, respect:   0.0, wariness:   0.0 }
- base_delta(Resentment) == AxisDelta { trust:   0.0, affinity: -10.0, respect:  -5.0, wariness:  15.0 }
- base_delta(Pity)       == AxisDelta { trust:   0.0, affinity:  10.0, respect:  -5.0, wariness:   0.0 }
- base_delta(Gloating)   == AxisDelta { trust: -10.0, affinity: -20.0, respect: -10.0, wariness:   0.0 }
- base_delta(Pride)      == AxisDelta { trust:   0.0, affinity:   5.0, respect:  10.0, wariness:   0.0 }
- base_delta(Shame)      == AxisDelta { trust:  -5.0, affinity: -10.0, respect: -10.0, wariness:   5.0 }
- base_delta(Love)       == AxisDelta { trust:   5.0, affinity:  20.0, respect:   5.0, wariness:  -5.0 }
- base_delta(Hate)       == AxisDelta { trust: -10.0, affinity: -25.0, respect:  -5.0, wariness:  15.0 }

[10 OCC default 검증 — B-D14]
- base_delta(Joy)             == AxisDelta::default()  // 모두 0
- base_delta(Distress)        == AxisDelta::default()
- base_delta(Hope)            == AxisDelta::default()
- base_delta(Fear)            == AxisDelta::default()
- base_delta(Satisfaction)    == AxisDelta::default()
- base_delta(Disappointment)  == AxisDelta::default()
- base_delta(Relief)          == AxisDelta::default()
- base_delta(FearsConfirmed)  == AxisDelta::default()
- base_delta(Remorse)         == AxisDelta::default()
- base_delta(Gratification)   == AxisDelta::default()

[합산 검증 — S2 산신묘 케이스]
- base_delta(Anger) + base_delta(Hate) + base_delta(Reproach)
  == AxisDelta { trust: -45.0, affinity: -45.0, respect: -30.0, wariness: 50.0 }
```

**비포함**:
- intensity 곱셈 → 2.4 `update_axes_from_emotion`
- HEXACO 곱셈 → 2.3 `hexaco_modifier` + 2.4 적용
- BondStatus 차단 → 2.4
- Shame/Pride agent_id=None 처리 → 2.4 (base_delta 자체는 표값만 반환)

#### 2.3 — `AxisModifier` 메서드 + `hexaco_modifier` 6 보정 룰

**목적**: HEXACO 6 facet → 4축 곱셈 배수. v0.7 §4.3 보정 룰 6개. base_delta 결과에 *곱셈 적용*.

**위치**:
- `AxisModifier` 메서드: `src/domain/relationship/axis.rs` (2.1 결정 — 타입은 axis.rs)
- `hexaco_modifier` 함수: `src/domain/relationship/mapping.rs`

##### 시그니처

```rust
// === axis.rs (2.1 박힌 AxisModifier에 메서드 추가) ===

impl AxisModifier {
    /// 모든 축에 동일 곱셈 (A+ Patience ×0.7 / C+ Prudence ×0.8 같은 "전역" 룰).
    pub fn combine_uniform(self, factor: f32) -> Self {
        Self {
            trust:    self.trust    * factor,
            affinity: self.affinity * factor,
            respect:  self.respect  * factor,
            wariness: self.wariness * factor,
        }
    }

    /// 단일 축에만 곱셈 (H+ Sincerity ×1.2 trust / E+ Anxiety ×1.3 wariness 같은 "축별" 룰).
    pub fn scale_axis(mut self, kind: AxisKind, factor: f32) -> Self {
        match kind {
            AxisKind::Trust    => self.trust    *= factor,
            AxisKind::Affinity => self.affinity *= factor,
            AxisKind::Respect  => self.respect  *= factor,
            AxisKind::Wariness => self.wariness *= factor,
        }
        self
    }
}

// === mapping.rs ===

use crate::domain::personality::HexacoProfile;
use crate::domain::relationship::axis::{AxisModifier, AxisKind};

/// HEXACO 6 facet → 4축 곱셈 배수.
/// v0.7 §4.3 — 6 보정 룰.
///
/// emotion 인자는 *A- Forgiveness 부정감정 한정* 룰에 사용.
pub(crate) fn hexaco_modifier(
    emotion: OccEmotion,
    hexaco: &HexacoProfile,
) -> AxisModifier {
    let mut m = AxisModifier::default();  // 모두 1.0

    // ── H+ Sincerity 높음 → trust 변화 ×1.2 ─────
    if hexaco.honesty_humility.sincerity.value() > HIGH_THRESHOLD {
        m = m.scale_axis(AxisKind::Trust, 1.2);
    }

    // ── A+ Patience 높음 → 모든 변화 ×0.7 ─────
    if hexaco.agreeableness.patience.value() > HIGH_THRESHOLD {
        m = m.combine_uniform(0.7);
    }

    // ── A- Forgiveness 낮음 → 부정 감정 변화 ×1.5 ─────
    if hexaco.agreeableness.forgiveness.value() < LOW_THRESHOLD
        && is_negative_emotion(emotion) {
        m = m.combine_uniform(1.5);
    }

    // ── E+ Anxiety 높음 → wariness 변화 ×1.3 ─────
    if hexaco.emotionality.anxiety.value() > HIGH_THRESHOLD {
        m = m.scale_axis(AxisKind::Wariness, 1.3);
    }

    // ── C+ Prudence 높음 → 모든 변화 ×0.8 (의심 2 결정 a: 간소화) ─────
    // v0.7 "큰 변화 시 ×0.8, 시간 분산" — Stage 2 본체는 *간소 곱셈*. 
    // intensity 조건부 + 시간 분산은 Phase 2.3에서 정밀화.
    if hexaco.conscientiousness.prudence.value() > HIGH_THRESHOLD {
        m = m.combine_uniform(0.8);
    }

    // ── O+ Unconventionality 높음 → 양극 도달 더 쉬움 ─────
    // *Phase 2 본체에서는 미적용* — v0.7 "양극 도달 가속"은 clamp 근처에서만 의미.
    // 단순 곱셈으로 표현 어려움. Phase 2.3 또는 3+에서 정밀화.
    // (placeholder — 적용 0)

    m
}

/// HEXACO facet "높음" 임계 (의심 1 결정 α: 0.5).
const HIGH_THRESHOLD: f32 = 0.5;
const LOW_THRESHOLD:  f32 = -0.5;

/// 부정 감정 식별 — A- Forgiveness 룰 적용 조건.
fn is_negative_emotion(emotion: OccEmotion) -> bool {
    matches!(
        emotion,
        OccEmotion::Anger | OccEmotion::Reproach
        | OccEmotion::Resentment | OccEmotion::Gloating
        | OccEmotion::Hate | OccEmotion::Distress
        | OccEmotion::Fear | OccEmotion::Disappointment
        | OccEmotion::FearsConfirmed | OccEmotion::Shame
        | OccEmotion::Remorse
    )
}
```

##### 설계 의도 6개

| # | 항목 | 의도 |
|---|---|---|
| ① | `AxisModifier` 메서드 2개 (`combine_uniform` / `scale_axis`) | 6 룰 적용 패턴 — 전역 곱셈 (Patience/Prudence) vs 축별 곱셈 (Sincerity/Anxiety) |
| ② | `hexaco_modifier(emotion, &hexaco)` 시그니처 | A- Forgiveness 룰이 *부정 감정 한정*이라 emotion 필요. 다른 룰은 hexaco만 사용. |
| ③ | `HIGH_THRESHOLD = 0.5` / `LOW_THRESHOLD = -0.5` (의심 1 결정 α) | 중립(0.0)과 최대(±1.0)의 *중간*. "높음"/"낮음" 정량 기준. Phase 2.3에서 시뮬레이션 검증 후 미세조정. |
| ④ | C+ Prudence "큰 변화 시" 조건 *간소화* — 모든 변화 ×0.8 (의심 2 결정 a) | v0.7의 "큰 변화 시 + 시간 분산"은 Stage 2 본체에서 *간소 곱셈*. Phase 2.3에서 intensity 조건부 + 시간 분산 정밀화. |
| ⑤ | O+ Unconventionality 룰 *placeholder*만 (적용 0) | v0.7 "양극 도달 더 쉬움"은 *clamp 근처 가속*이라 단순 곱셈으로 표현 어려움. Phase 2.3 또는 3+에서 정밀화. |
| ⑥ | `is_negative_emotion` 헬퍼 | A- Forgiveness 룰의 부정 감정 식별. 12 base + Distress/Fear/Disappointment/FearsConfirmed/Remorse 포함 (총 11개). |

##### 계산 검증 (Stage 0 §3.6 S2 케이스)

```
임충 산신묘 사건 — Anger 0.95 감정 발생.
임충 HEXACO: C+ Prudence 0.8 (높음) + A- Forgiveness -0.7 (낮음)

hexaco_modifier(Anger, &hexaco) 적용:
  초기: AxisModifier { 1.0, 1.0, 1.0, 1.0 }
  ── H+ Sincerity 0.7 > 0.5 → trust ×1.2 → { 1.2, 1.0, 1.0, 1.0 }
  ── A+ Patience 0.3 < 0.5 → 미적용
  ── A- Forgiveness -0.7 < -0.5 + Anger 부정 → 전역 ×1.5 → { 1.8, 1.5, 1.5, 1.5 }
  ── E+ Anxiety 0.4 < 0.5 → 미적용
  ── C+ Prudence 0.8 > 0.5 → 전역 ×0.8 → { 1.44, 1.2, 1.2, 1.2 }
  ── O+ Unconventionality → placeholder, 적용 0

최종: AxisModifier { trust: 1.44, affinity: 1.2, respect: 1.2, wariness: 1.2 }
```

(이 modifier × base_delta(Anger) × intensity 0.95 적용은 2.4 `update_axes_from_emotion`)

##### 단위 테스트 케이스 (2.7에서 구현)

```
[AxisModifier 메서드]
- AxisModifier::default().combine_uniform(0.7)
  == AxisModifier { trust: 0.7, affinity: 0.7, respect: 0.7, wariness: 0.7 }
- AxisModifier::default().scale_axis(AxisKind::Trust, 1.2)
  == AxisModifier { trust: 1.2, affinity: 1.0, respect: 1.0, wariness: 1.0 }
- AxisModifier::default().scale_axis(AxisKind::Wariness, 1.3)
  == AxisModifier { trust: 1.0, affinity: 1.0, respect: 1.0, wariness: 1.3 }

[hexaco_modifier — 단일 룰 발동]
- H+ Sincerity 0.7만 높음, Anger 입력
  → AxisModifier { trust: 1.2, 나머지: 1.0 }
- A+ Patience 0.7만 높음
  → AxisModifier { 1.0, 1.0, 1.0, 1.0 } × 0.7 = { 0.7, 0.7, 0.7, 0.7 }
- A- Forgiveness -0.7 낮음 + Anger (부정)
  → AxisModifier 모두 × 1.5 = { 1.5, 1.5, 1.5, 1.5 }
- A- Forgiveness -0.7 낮음 + Gratitude (긍정) — 룰 미발동
  → AxisModifier::default()
- E+ Anxiety 0.7만 높음
  → AxisModifier { trust: 1.0, affinity: 1.0, respect: 1.0, wariness: 1.3 }
- C+ Prudence 0.7만 높음
  → AxisModifier 모두 × 0.8 = { 0.8, 0.8, 0.8, 0.8 }

[hexaco_modifier — 복합 룰 (S2 임충 케이스)]
- HEXACO: Sincerity 0.7 + Forgiveness -0.7 + Prudence 0.8, Anger 입력
  단계: × 1.2(trust) → × 1.5(전역) → × 0.8(전역)
  최종: AxisModifier { trust: 1.44, affinity: 1.2, respect: 1.2, wariness: 1.2 }

[hexaco_modifier — neutral HEXACO]
- HEXACO 모든 facet 0.0, 어떤 OCC 입력이든
  → AxisModifier::default()  (모든 룰 미발동, 모두 1.0)

[is_negative_emotion 식별]
- Anger / Reproach / Resentment / Gloating / Hate / Distress / Fear /
  Disappointment / FearsConfirmed / Shame / Remorse → true
- Joy / Gratitude / Admiration / HappyFor / Pity / Pride / Love /
  Hope / Satisfaction / Relief / Gratification → false
```

##### 비포함

- intensity 곱셈 적용 → 2.4 `update_axes_from_emotion`
- BondStatus 차단 → 2.4
- Shame/Pride agent_id=None 처리 → 2.4
- C+ Prudence intensity 조건부 정밀화 → Phase 2.3
- C+ Prudence "시간 분산" → Phase 2.3 또는 3+
- O+ Unconventionality "양극 가속" → Phase 2.3 또는 3+

#### 2.4 — `update_axes_from_emotion` 단일 함수

**목적**: Stage 2의 *통합 진입점*. 2.2 (base_delta) + 2.3 (hexaco_modifier)을 intensity와 함께 묶어 적용. BondStatus 차단 게이트 포함. B-D5 단일 함수.

**위치**: `src/domain/relationship/mapping.rs`

##### 시그니처

```rust
use crate::domain::personality::HexacoProfile;
use crate::domain::relationship::axis::AxisDelta;
use crate::domain::relationship::Relationship;
use crate::domain::emotion::OccEmotion;

/// OCC 감정 → 4축 변동 통합 적용.
///
/// 흐름 (v0.7 §4.1):
///   1. BondStatus 차단 (`accepts_live_input` false면 즉시 종료)
///   2. base_delta(emotion) lookup (2.2)
///   3. intensity × hexaco_modifier(emotion, hexaco) 곱셈 (2.3)
///   4. rel.apply_delta(&delta) — 4축 자동 clamp
///
/// **B-D12 (Shame/Pride agent_id=None) 처리는 *호출 측*** (Stage 3 RelationshipPolicy).
/// 자기 평가는 *어느 관계에도 적용 안 함* — 호출 측이 이 함수 호출 안 하는 게 자연.
///
/// 호출 위치 (Stage 3):
/// - `relationship_policy.rs` — DialogueEndRequested handler (대화 끝 batch)
/// - BeatTransitioned 분기
pub fn update_axes_from_emotion(
    rel: &mut Relationship,
    emotion: OccEmotion,
    intensity: f32,
    hexaco: &HexacoProfile,
) {
    // ── 가드 1: BondStatus 차단 ─────
    if !rel.bond_status().accepts_live_input() {
        return;
    }

    // ── 매핑 + 곱셈 ─────
    let base = base_delta(emotion);
    let modulator = hexaco_modifier(emotion, hexaco);
    let delta = AxisDelta {
        trust:    base.trust    * intensity * modulator.trust,
        affinity: base.affinity * intensity * modulator.affinity,
        respect:  base.respect  * intensity * modulator.respect,
        wariness: base.wariness * intensity * modulator.wariness,
    };

    // ── 적용 (Relationship::apply_delta가 자동 clamp) ─────
    rel.apply_delta(&delta);
}
```

##### v0.7 §4.1과 비교

시맨틱 동일. Stage 2 본체는 *`apply_delta` 메서드*를 통해 clamp 일원화. v0.7 의도와 정합 + 캡슐화 보존.

##### 설계 의도 5개

| # | 항목 | 의도 |
|---|---|---|
| ① | `pub` 가시성 (mod.rs `pub use` 통한 외부 노출) | Stage 3 `relationship_policy.rs`가 외부 진입점으로 호출. base_delta/hexaco_modifier는 `pub(crate)`로 *내부 헬퍼*. |
| ② | BondStatus 차단을 *함수 진입 첫 줄*에 | 도메인 invariant — Relationship 자체 책임. Deceased/Resolved/Dormant 즉시 종료. Reactivating은 통과 (1.4 결정). |
| ③ | **B-D12 (Shame/Pride agent_id=None) 처리는 호출 측** (의심 1 결정 A) | 함수 책임 = *상대 관계 4축 갱신*. 자기 평가는 *별개 시스템*. 호출 측 (Stage 3 RelationshipPolicy)이 ActionFocus.agent_id 분기. agent_id=None → 함수 호출 안 함. |
| ④ | 인라인 곱셈 (`base.* * intensity * modulator.*`) (의심 2 결정 b) | `AxisDelta::multiply_by` 같은 추가 메서드 *없음*. 코드 4줄 단순. 가독성 좋음. Stage 2 본체 *추가 메서드 최소*. |
| ⑤ | `&mut Relationship` 시그니처 — `apply_delta` 호출 | Relationship의 캡슐화 보존 (1.6 의심 2 결정 X). 외부에서 *raw 필드 직접 변경* 차단. |

##### 계산 검증 (S2 임충 케이스 — Anger 단독)

```
입력: rel(임충→육겸 산신묘 *전*), Anger 0.95, 임충 HEXACO

1. BondStatus 차단: rel.bond_status = Active → accepts_live_input == true → 통과
2. base_delta(Anger) = AxisDelta { trust: -25, affinity: -10, respect: 0, wariness: 25 }
3. hexaco_modifier(Anger, &hexaco) = AxisModifier { trust: 1.44, affinity: 1.2, respect: 1.2, wariness: 1.2 } (2.3 검증)
4. delta = base * 0.95 * modulator:
     trust    = -25 * 0.95 * 1.44 = -34.2
     affinity = -10 * 0.95 * 1.2  = -11.4
     respect  =   0 * 0.95 * 1.2  =   0
     wariness =  25 * 0.95 * 1.2  =  28.5
5. rel.apply_delta(&delta):
     trust    50 + (-34.2) = +15.8  (clamp ±100 통과)
     affinity 40 + (-11.4) = +28.6
     respect  30 +    0    = +30
     wariness  5 +  28.5   = +33.5
```

Hate + Reproach까지 합치면 *3 차례 함수 호출*. 누적 결과는 Stage 0 §3.6 S2.

##### 단위 테스트 케이스 (2.7에서 구현)

```
[정상 통합 — Active Status]
- let mut rel = Builder::new("a", "b").trust(AxisScore::new(50)).affinity(AxisScore::new(40)).build();
  let hex = HexacoProfile::neutral();
  update_axes_from_emotion(&mut rel, OccEmotion::Gratitude, 0.7, &hex);
  → rel.trust().value()    == 50 + (20 * 0.7 * 1.0) = 64
  → rel.affinity().value() == 40 + (10 * 0.7 * 1.0) = 47
  → rel.wariness().value() ==  0 (clamp floor — Gratitude wariness -10 * 0.7 = -7, clamp 0)

[BondStatus 차단 — Deceased]
- let mut rel = Builder::new("a", "b").trust(AxisScore::new(50)).bond_status(BondStatus::Deceased).build();
  update_axes_from_emotion(&mut rel, OccEmotion::Anger, 0.95, &hex);
  → rel.trust().value() == 50  (변경 0)

[BondStatus 차단 — Resolved + Dormant 동일]
- bond_status = Resolved { reason: "사화".into() } → 변경 0
- bond_status = Dormant → 변경 0

[BondStatus 허용 — Active + Reactivating]
- bond_status = Active → axes 변동 정상
- bond_status = Reactivating { trigger: EventId("evt_001".into()) } → axes 변동 정상

[Default HEXACO — modifier 모두 1.0]
- HexacoProfile::neutral() 입력 시 hexaco_modifier가 default 반환
- 결과: delta == base * intensity (modifier 영향 0)

[Intensity 0.0 — 변동 0]
- update_axes_from_emotion(..., Gratitude, 0.0, ...) → 4축 변동 0

[S2 임충 시뮬레이션 — Anger 0.95 + 임충 HEXACO]
- 위 계산 검증 그대로:
  rel.trust    50 → 15.8
  rel.affinity 40 → 28.6
  rel.respect  30 → 30
  rel.wariness  5 → 33.5

[clamp 동작]
- rel.trust = 95, Gratitude 0.95 적용 → 95 + ~19 = 100 (clamp cap)
- rel.wariness = 3, Love 0.95 적용 → 3 + (-5 * 0.95) = -1.75 → 0.0 (floor)
```

##### 비포함

- B-D12 Shame/Pride 가드 — 호출 측 (Stage 3 RelationshipPolicy)
- 합산 패턴 (Anger + Hate + Reproach 3 차례 호출) — Stage 3 RelationshipPolicy가 *대화 끝 batch*로 호출
- axis_modulation (Phase 2.5 LLM 3지선다) — Reflection schema 확장 별도
- C+ Prudence intensity 조건부 정밀화 — Phase 2.3

#### 2.5 — `RelationshipModifiers` 이름 변경 + 5곳 사용처 갱신 (TBD)

#### 2.6 — `Relationship::after_dialogue` 호출 3곳 → `update_axes_from_emotion` 이관 (TBD)

#### 2.7 — Stage 2 단위 테스트 (TBD)

---

**Stage 2 종합 게이트** (2.1~2.7 모두 통과 시):
1. `cargo check --all-features` 통과
2. 단위 테스트 통과 (S1~S4 ground truth ±N 이내)
3. base_delta 48셀 결정론 (같은 입력 → 같은 출력)
4. BondStatus Deceased/Resolved/Dormant 차단 확인
5. Shame/Pride 4축 변동 0 확인 (agent_id=None)
6. Stage 1 baseline (~1258 tests) + Stage 2 신규 통과

**산출 commit**: `phase2-stage2-mapping.md` 회고
**?곗텧 commit**: `phase2-stage3-updater.md` ?뚭퀬

---

### Stage 4 ??留덉씠洹몃젅?댁뀡 ?꾧뎄 + ?쒕굹由ъ삤 ?곗씠??
**踰붿쐞**:
- Rust binary ?묒꽦: `tools/migrate_relationships/` (B-D8)
  - ?낅젰: v0.6 ?쒕굹由ъ삤 JSON ?붾젆?좊━
  - 異쒕젰: v0.7 ?쒕굹由ъ삤 JSON
  - ?먮룞 蹂?? `trust 횞 100` ??trust / `closeness 횞 100` ??affinity / BondKind 湲곕컲 respect/wariness baseline (B-D10) / `power` ?꾨뱶 ??젣 / default ?꾨뱶 梨꾩?
  - ?듭뀡: `--dry-run` / `--diff` / `--backup-dir`
- Claude prompt template 3 ?뚯씪 (`docs/migration/claude-prompts/`):
  - `bond-kind-inference.md` ???쒕굹由ъ삤 ?섏뼱蹂?BondKind 異붾줎
  - `type-text-inference.md` ??type ?먯쑀 ?띿뒪??異붾줎
  - `adjustment-suggestion.md` ??narrative 寃곌낵 湲곕컲 議곗젙 ?쒖븞 (Stage 5?먯꽌 ?ъ슜)
- 諛깆뾽 ?붾젆?좊━ ?대룞: `data/scenarios.backup-v0.6/` + `data/sessions.backup-v0.6/`
- ?붿옄?대꼫 + Claude ?묒뾽: ~45 ?섏뼱 BondKind/type 諛뺢린
- 留덉씠洹몃젅?댁뀡 ?꾧뎄 ?ㅽ뻾 ??v0.7 JSON ?앹꽦
- `_schema.md` v0.6 ??v0.7 媛깆떊 (Relationship ?뱀뀡 ?쒖젙)

**寃뚯씠??*:
1. 留덉씠洹몃젅?댁뀡 ?꾧뎄 ?먯껜 ?⑥쐞 ?뚯뒪???듦낵
2. 紐⑤뱺 ?쒕굹由ъ삤 JSON v0.7 schema ?듦낵 (serde ??쭅?ы솕 + ?꾨찓??validation)
3. 留덉씠洹몃젅?댁뀡 ??而댄뙆??+ 1095+ tests ?듦낵
4. ?붿옄?대꼫 BondKind/type 寃???듦낵 (Claude 異붾줎 寃곌낵 ?⑸떦???뺤씤)

**?곗텧 commit**: `phase2-stage4-migration.md` ?뚭퀬

---

### Stage 5 ??Narrative ?쒕??덉씠??寃利?
**踰붿쐞**:
- Phase 1 narrative 3 ?쒕굹由ъ삤 (chitchat-passerby/daily-training/lin-chong-shanshenmiao) 4異??쒖뒪?쒖뿉???ъ떎??- S1~S4 ?쒕??덉씠??耳?댁뒪瑜?*Phase 2 narrative test*濡?諛뺤쓬 (`tests/phase2_narrative_test.rs` ?좎꽕)
  - 媛?耳?댁뒪 ground truth (湲곕? 4異?蹂?? 紐낆떆
  - base_delta + HEXACO + (axis_modulation Phase 2.5 ?꾩씠誘濡?紐⑤몢 "default") ?곸슜 寃곌낵 ?뺤씤
- session_*_result.json ?쇨큵 ?ъ깮??(B-D9)
- ?댁깋 耳?댁뒪 ?앸퀎 ??Claude AI 異붾줎?쇰줈 議곗젙 ?쒖븞 (`adjustment-suggestion.md` ?ъ슜) ???붿옄?대꼫 寃????JSON 誘몄꽭議곗젙
- ?쒕굹由ъ삤蹂?narrative report

**寃뚯씠??*:
1. 3諛대뱶 calibration 蹂댁〈 (chitchat 0.000 / daily 0.461 / shanshenmiao 0.980 짹 tolerance ??D3 baseline 鍮꾧탳)
2. S1~S4 ground truth 짹N ?대궡
3. 4異?蹂?숈씠 ?쒕굹由ъ삤 ?섎룄? ?뺥빀 (?붿옄?대꼫 narrative 寃???듦낵)
4. ?댁깋 耳?댁뒪 0嫄?(?먮뒗 紐⑤몢 ?붿옄?대꼫 議곗젙 ?꾨즺)

**?곗텧 commit**: `phase2-stage5-narrative.md` ?뚭퀬

---

### Stage 6 ??Bench + ?뚭퀬 + Phase 2.3 KICKOFF

**踰붿쐞**:
- `dispatch_v2(EndDialogue)` ?ъ륫??(chitchat/significant/legacy) ??D2 baseline 鍮꾧탳
- `compute_significance` ?ъ륫????D4 baseline 鍮꾧탳
- `MAX_EVENTS_PER_COMMAND = 22` ?덉쟾???ы솗??(A5 worst-case ?곗텧 寃利?
- 4異?留ㅽ븨 異붽???latency ?곹뼢 痢≪젙 (Stage 2??base_delta lookup + HEXACO 蹂댁젙 鍮꾩슜)
- Phase 2 checkpoint report ?묒꽦 (`phase2-checkpoint-report.md`)
- **Phase 2.3 KICKOFF ?묒꽦** (`PHASE2.3-KICKOFF.md`) ???ㅼ쓬 phase ?멸퀎
  - Phase 2.3 spec ?묒꽦 以鍮?(`task-rel-phase2.3-appraise-tuning.md`)
  - ?쒕??덉씠???쒕굹由ъ삤 set ?좎꽕 ?붾젆?좊━ (`data/scenarios/appraise-validation/`)
- ?몃? 臾몄꽌 ?몃뜳??媛깆떊:
  - `CLAUDE.md` Mind Architecture Phase 2 ???쒓린
  - `00-roadmap.md` 짠5 Phase 2 ?꾨즺 ?쒓린 + 짠6.5 짠1~짠3 吏꾩쿃 媛깆떊
- spec `task-rel-phase2-domain-migration.md` v1.0 frozen ?쒓린

**寃뚯씠??*:
1. Latency 짹20% ?대궡 (D2 baseline 鍮꾧탳, 4異?留ㅽ븨 異붽? ?곹뼢 痢≪젙媛?諛뺤젣)
2. Bench ?ъ륫???꾨즺
3. ?뚭? 0嫄?(1095+ tests ?듦낵 + narrative 3諛대뱶 蹂댁〈)
4. Phase 2.3 吏꾩엯 以鍮??꾨즺 (KICKOFF + spec ?붾젆?좊━ 珥덉븞)
5. ?몃? 臾몄꽌 ?몃뜳???숆린???꾨즺

**?곗텧 commit**: `phase2-stage6-bench-handoff.md` ?뚭퀬 + `PHASE2.3-KICKOFF.md`

---

## Stage 0 醫낃껐

蹂?spec??짠0~짠7 紐⑤뱺 ???묒꽦 ?꾨즺. Phase 2 蹂몄껜 寃곗젙 12媛?諛뺥옒. Stage 1 吏꾩엯 以鍮??꾨즺.

**?ㅼ쓬 ?묒뾽** (Stage 1 吏꾩엯):
1. `baselines/cargo-test-2026-MM-DD-PASS.log` ?ъ륫??諛뺤젣
2. Stage 1 `feat/phase2-stage1-domain` 釉뚮옖移??묒꽦
3. `src/domain/relationship/axis.rs` ?좎꽕遺???쒖옉

---

## 蹂寃??대젰

| 踰꾩쟾 | ?좎쭨 | 蹂寃?|
|---|---|---|
| 0.1 | 2026-05-13 | 珥덉븞. A 移댄뀒怨좊━ Findings 5媛?醫낃껐, B 移댄뀒怨좊━ 吏꾪뻾 以?(B-D4 ?뺤젙). |
| 0.2 | 2026-05-13 | 짠3.6 ?쒕??덉씠??寃利?(S1~S4) 異붽?. B-D6/D12/D13/D14 ?뺤젙. **Phase 2.3 ?좎꽕 寃곗젙**. 짠5 Risks ?묒꽦 (R1~R6). 짠1 Scope??Phase 2.3/2.5 鍮꾪룷????ぉ + axis_modulation ?쒓린 異붽?. |
| 0.3 | 2026-05-13 | B-D1 (Score ???遺꾨━) + B-D2 (f32 ?대? ?쒗쁽) ?뺤젙. `AxisScore` + `WarinessScore` 2 ????좎꽕 寃곗젙. R3 ?댁냼 ?쒓린. |
| 0.4 | 2026-05-13 | B-D3 (closeness ??affinity ?쇳빀 蹂?? ?뺤젙. ?먮룞 蹂??baseline + ?붿옄?대꼫 ?좏깮??議곗젙. |
| 0.5 | 2026-05-13 | B-D10 (respect/wariness 珥덇린媛?猷? ?뺤젙. 媛꾨떒 ?대━?ㅽ떛 + BondKind 蹂댁셿 (?먯닔/Guardian/Mentor/吏湲?李⑤벑 baseline). |
| 0.6 | 2026-05-13 | B-D5 (4異?留ㅽ븨 ?⑥닔 援ъ“) ?뺤젙. v0.7 짠4.1 ?⑥씪 ?⑥닔 洹몃?濡?(4異??숈떆 媛깆떊). |
| 0.7 | 2026-05-13 | B-D8 (?쒕굹由ъ삤 留덉씠洹몃젅?댁뀡 ?뚰겕?뚮줈?? ?뺤젙. W3+ ??Rust binary ?먮룞 蹂??+ Claude AI 異붾줎 (BondKind/type/議곗젙) + ?붿옄?대꼫 寃?? Claude prompt template 諛뺤쓬 (`docs/migration/claude-prompts/`). R2 ????꾪솕 ?쒓린. |
| 0.8 | 2026-05-13 | **??B 移댄뀒怨좊━ Phase 2 蹂몄껜 12媛?寃곗젙 ?꾨즺**. B-D9 (session_*_result.json ?먭린) ?뺤젙 ??Phase 2 ???쇨큵 ?ъ깮?? B-D7/B-D11? Phase 2.5 ?쒖젏 寃곗젙. 짠4 ?ㅻ뜑 醫낃껐 ?쒓린. |
| 0.9 | 2026-05-13 | **??짠6 Baseline (D 移댄뀒怨좊━) ?묒꽦**. Phase 1 醫낃껐 ?쒖젏 baseline ?몄슜: 1095 tests passed / dispatch_v2 latency 24/35/29쨉s / narrative 3諛대뱶 0.000/0.461/0.980 / compute_significance 8.36쨉s / EventKind 31媛? D1~D6 ??ぉ. Stage 1 吏꾩엯 吏곸쟾 ?ъ륫???묒뾽 紐낆떆. |
| 1.0 | 2026-05-13 | **??Stage 0 醫낃껐**. 짠7 Stages ?묒꽦 ??6 stage 遺꾪븷 (Stage 1 Type/Domain ??Stage 2 Mapping ??Stage 3 Updater ??Stage 4 Migration ??Stage 5 Narrative ??Stage 6 Bench/Handoff). 媛?stage 踰붿쐞쨌寃뚯씠?맞룹궛異?commit 紐낆떆. Phase 2 蹂몄껜 spec ?묒꽦 ?꾨즺, Stage 1 吏꾩엯 以鍮? |
| 1.1 | 2026-05-14 | **??Stage 1 spec ?묒꽦 ?꾨즺 (freeze)**. 1.1 ?붾젆?좊━ 援ъ“ (紐⑤뱢 遺꾪븷 梨꾪깮), 1.2 AxisScore + WarinessScore + AxisDelta + AxisKind, 1.3 BondKind 11 variants + ?곸뿭 ?ы띁 5媛?(is_zhiji 臾댄삊 ?꾨찓???⑹뼱 蹂댁〈), 1.4 BondStatus 5 variants + accepts_live_input (Reactivating ??true), 1.5 Partnership 4 variants, 1.6 Relationship 蹂몄껜 ?ъ옉??(4異?+ bond_* + partnership + type/type_history, power ?먭린, apply_delta 硫붿꽌?? modifiers closeness_* ??affinity_*), 1.7 RelationshipBuilder 4異?fluent API, 1.8 neutral() ?먮룞 ?≪닔 寃利?(22怨??몄텧, 19怨?蹂寃?0 ?덉긽), 1.9 ?⑥쐞 ?뚯뒪??(~38 ?좉퇋, 紐⑤뱢 ?대? ?⑦꽩). Claude Code??肄붾뵫 ?멸퀎. |