//! RigChatAdapter 타임아웃 통합 테스트
//!
//! - `send_message` 타임아웃 검증
//! - `send_message_stream` 타임아웃 검증

#![cfg(feature = "chat")]

use npc_mind::adapter::rig_chat::RigChatAdapter;
use npc_mind::ports::{ConversationError, ConversationPort};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;
use futures::StreamExt;

#[tokio::test]
async fn test_rig_chat_send_message_timeout() {
    // 1. 아무 응답도 하지 않는 가짜 서버 기동
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            // 연결은 받지만 데이터는 보내지 않고 대기
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = socket.shutdown().await;
        }
    });

    let base_url = format!("http://{}/v1", addr);
    // connect() 대신 직접 new()를 사용하여 모델 감지 단계를 건너뜀
    let adapter = RigChatAdapter::new(&base_url, "test-model")
        .with_timeout(Duration::from_millis(500));
    
    // 세션 시작 (실제 네트워크 호출이 발생하지 않음 — 메모리 내 세션 맵만 갱신)
    adapter.start_session("s1", "system", None).await.unwrap();

    // 2. 호출 및 타임아웃 에러 검증
    let start = std::time::Instant::now();
    let result = adapter.send_message("s1", "hello").await;
    let elapsed = start.elapsed();

    match result {
        Err(ConversationError::Timeout(d)) => {
            assert_eq!(d, Duration::from_millis(500));
            assert!(elapsed >= Duration::from_millis(500));
            assert!(elapsed < Duration::from_secs(2)); 
        }
        _ => panic!("Expected Timeout error, got {:?}", result),
    }
}

#[tokio::test]
async fn test_rig_chat_stream_timeout() {
    // 1. 연결만 받고 응답하지 않는 서버
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let _ = socket.shutdown().await;
        }
    });

    let base_url = format!("http://{}/v1", addr);
    let adapter = RigChatAdapter::new(&base_url, "test-model")
        .with_timeout(Duration::from_millis(500));

    adapter.start_session("s1", "system", None).await.unwrap();

    // 2. 스트리밍 호출 및 타임아웃 검증
    let mut stream = adapter.send_message_stream("s1", "hello");
    let start = std::time::Instant::now();
    let item = stream.next().await;
    let elapsed = start.elapsed();

    match item {
        Some(Err(ConversationError::Timeout(d))) => {
            assert_eq!(d, Duration::from_millis(500));
            assert!(elapsed >= Duration::from_millis(500));
        }
        _ => panic!("Expected Timeout error in stream, got {:?}", item),
    }
}
