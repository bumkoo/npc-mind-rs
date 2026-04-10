import React, { useState, useRef, useEffect } from "react";
import type { Npc, ChatMessage, ScenarioTurn, SceneInfo } from "../../types";

interface ChatPanelProps {
  npcId: string;
  partnerId: string;
  npcs: Npc[];
  messages: ChatMessage[];
  scenarioTurns: ScenarioTurn[];
  scenarioIndex: number;
  chatLoading: boolean;
  selectedMsgIdx: number | null;
  sceneInfo: SceneInfo | null;
  onSend: (
    utterance: string,
    pad?: { pleasure: number; arousal: number; dominance: number } | null,
  ) => void;
  onNextTurn: () => void;
  onSelectMsg: (idx: number) => void;
  onEndChat: () => void;
}

function ChatPanel({
  npcId,
  partnerId,
  npcs,
  messages,
  scenarioTurns,
  scenarioIndex,
  chatLoading,
  selectedMsgIdx,
  sceneInfo,
  onSend,
  onNextTurn,
  onSelectMsg,
  onEndChat,
}: ChatPanelProps) {
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const npc = npcs.find((n) => n.id === npcId);
  const partner = npcs.find((n) => n.id === partnerId);
  const hasScenario = scenarioTurns && scenarioTurns.length > 0;
  const nextTurn =
    hasScenario && scenarioIndex < scenarioTurns.length
      ? scenarioTurns[scenarioIndex]
      : null;

  // 테스트 스크립트: 현재 활성 Beat의 대사 목록
  const activeFocus = sceneInfo?.focuses?.find((f) => f.is_active);
  const testScript = activeFocus?.test_script || [];
  const scriptCursor = sceneInfo?.script_cursor || 0;
  const hasScript = testScript.length > 0;
  const nextScriptLine =
    hasScript && scriptCursor < testScript.length
      ? testScript[scriptCursor]
      : null;

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({
      behavior: "smooth",
    });
  }, [messages]);

  const handleSend = () => {
    const text = input.trim();
    if (!text || chatLoading) return;
    onSend(text);
    setInput("");
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div
      className="center"
      style={{
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      {/* Header */}
      <div className="chat-header">
        <div>
          <span className="chat-title">
            💬 {npc?.name || npcId} ↔ {partner?.name || partnerId}
          </span>
          {hasScenario && <span className="chat-mode">시나리오</span>}
        </div>
        <button className="btn small danger" onClick={onEndChat}>
          대화 종료
        </button>
      </div>

      {/* Messages */}
      <div className="chat-messages">
        {messages.map((msg, i) => {
          if (msg.beat_changed) {
            return (
              <React.Fragment key={i}>
                <div className="chat-beat-divider">
                  🔄 Beat 전환
                  {msg.new_focus ? `: ${msg.new_focus}` : ""}
                </div>
                <div
                  className={`chat-msg ${msg.role} ${selectedMsgIdx === i ? "selected" : ""} ${msg.streaming ? "streaming" : ""}`}
                  onClick={() => onSelectMsg(i)}
                >
                  <div className="msg-role">
                    {msg.role === "assistant"
                      ? npc?.name || npcId
                      : msg.role === "user"
                        ? partner?.name || partnerId
                        : "System"}
                  </div>
                  {msg.content}
                  {msg.emotions && (
                    <div className="msg-emotions">
                      📊{" "}
                      {Object.entries(msg.emotions)
                        .map(([k, v]) => `${k} ${v.toFixed(3)}`)
                        .join(" · ")}
                    </div>
                  )}
                </div>
              </React.Fragment>
            );
          }
          return (
            <div
              key={i}
              className={`chat-msg ${msg.role} ${selectedMsgIdx === i ? "selected" : ""} ${msg.streaming ? "streaming" : ""}`}
              onClick={() => onSelectMsg(i)}
            >
              <div className="msg-role">
                {msg.role === "assistant"
                  ? npc?.name || npcId
                  : msg.role === "user"
                    ? partner?.name || partnerId
                    : "System"}
              </div>
              {msg.content}
              {msg.emotions && (
                <div className="msg-emotions">
                  📊{" "}
                  {Object.entries(msg.emotions)
                    .map(([k, v]) => `${k} ${v.toFixed(3)}`)
                    .join(" · ")}
                </div>
              )}
            </div>
          );
        })}
        {chatLoading && (
          <div className="chat-loading">
            <span>{npc?.name || npcId} 응답 중</span>
            <span className="dots"></span>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input area */}
      {hasScenario ? (
        <div className="chat-scenario-bar">
          {nextTurn ? (
            <>
              <span className="turn-preview">
                다음: &quot;{nextTurn.utterance}&quot;
              </span>
              <span className="turn-count">
                {scenarioIndex + 1}/{scenarioTurns.length}
              </span>
              <button
                className="btn small primary"
                onClick={onNextTurn}
                disabled={chatLoading}
              >
                전송 ▶
              </button>
            </>
          ) : (
            <span
              style={{
                fontSize: 11,
                color: "var(--accent2)",
              }}
            >
              ✓ 시나리오 완료 ({scenarioTurns.length}턴)
            </span>
          )}
        </div>
      ) : (
        <div className="chat-input-area">
          {nextScriptLine && (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                padding: "4px 8px",
                background: "var(--bg2)",
                borderRadius: "var(--radius)",
                border: "1px solid var(--accent2)",
                fontSize: 11,
              }}
            >
              <span
                style={{
                  color: "var(--accent2)",
                  fontWeight: 600,
                  whiteSpace: "nowrap",
                }}
              >
                📋 {scriptCursor + 1}/{testScript.length}
              </span>
              <span
                style={{
                  flex: 1,
                  color: "var(--fg)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {nextScriptLine}
              </span>
              <button
                className="btn primary"
                style={{ fontSize: 10, padding: "2px 8px" }}
                disabled={chatLoading}
                onClick={() => onSend(nextScriptLine)}
              >
                스크립트 전송
              </button>
            </div>
          )}
          <div className="chat-input-row">
            <textarea
              rows={2}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={
                nextScriptLine
                  ? "즉흥 대사 입력 또는 위 스크립트 전송..."
                  : "대사를 입력하세요... (Enter로 전송)"
              }
              disabled={chatLoading}
            />
            <button
              className="btn primary"
              onClick={handleSend}
              disabled={chatLoading || !input.trim()}
              style={{ alignSelf: "flex-end" }}
            >
              전송
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default ChatPanel;
