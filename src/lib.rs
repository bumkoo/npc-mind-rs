//! NPC Mind Engine
//!
//! 성격(HEXACO)이 상황(Context)을 해석하여
//! 감정(OCC)을 생성하고, LLM 연기 가이드를 출력하는 엔진.

pub mod domain;
pub mod ports;
pub mod presentation;
pub mod adapter;
