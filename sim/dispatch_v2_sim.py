"""
dispatch_v2 BFS 시뮬레이션 — `Command::EndDialogue { muback ↔ okgyoryong }` 1회
참고: src/application/command/dispatcher.rs (BFS 루프), priority.rs (상수)
"""
from collections import deque
from dataclasses import dataclass, field
from typing import Optional, List, Callable

@dataclass
class DomainEvent:
    payload_kind: str
    aggregate_id: str
    id: Optional[int] = None
    sequence: Optional[int] = None
    correlation_id: Optional[int] = None
    cascade_depth: int = 0
    handler_origin: Optional[str] = None

def relationship_policy(ev):
    """DialogueEndRequested 받으면 RelationshipUpdated + SceneEnded follow-up 발행"""
    if ev.payload_kind == "DialogueEndRequested":
        return [
            DomainEvent(payload_kind="RelationshipUpdated",
                        aggregate_id="rel:muback->okgyoryong",
                        handler_origin="RelationshipPolicy"),
            DomainEvent(payload_kind="SceneEnded",
                        aggregate_id="scene:muback<->okgyoryong",
                        handler_origin="RelationshipPolicy"),
        ]
    return []

TRANSACTIONAL_HANDLERS = [
    (5,  "ScenePolicy",        ["SceneStartRequested"], lambda ev: []),
    (10, "EmotionPolicy",      ["AppraiseRequested"],   lambda ev: []),
    (15, "StimulusPolicy",     ["StimulusApplyRequested"], lambda ev: []),
    (20, "GuidePolicy",        ["GuideRequested"],      lambda ev: []),
    (30, "RelationshipPolicy", ["DialogueEndRequested", "RelationshipUpdateRequested"],
         relationship_policy),
]

INLINE_HANDLERS = [
    (10, "EmotionProjectionHandler",      ["EmotionAppraised", "StimulusApplied"]),
    (20, "RelationshipProjectionHandler", ["RelationshipUpdated"]),
    (30, "SceneProjectionHandler",        ["SceneEnded"]),
    (50, "RelationshipMemoryHandler",     ["RelationshipUpdated"]),
    (60, "SceneConsolidationHandler",     ["SceneEnded"]),
]


# ---------------------------------------------------------------------------
# EventStore (in-memory)
# ---------------------------------------------------------------------------

class EventStore:
    def __init__(self):
        self.events: List[DomainEvent] = []
        self._next_id = 1

    def next_id(self) -> int:
        v = self._next_id
        self._next_id += 1
        return v

    def next_sequence(self, aggregate_id: str) -> int:
        return sum(1 for e in self.events if e.aggregate_id == aggregate_id) + 1

    def append(self, ev: DomainEvent):
        self.events.append(ev)


# ---------------------------------------------------------------------------
# Dispatcher 시뮬레이션
# ---------------------------------------------------------------------------

class CommandDispatcher:
    def __init__(self):
        self.event_store = EventStore()
        self.command_seq = 1

    def dispatch_v2(self, initial_event: DomainEvent):
        """
        본인 코드의 dispatch_v2 BFS 루프를 그대로 따라합니다.
        반환: 이번 cmd가 만든 committed events (cid 부착됨)
        """
        cid = self.command_seq
        self.command_seq += 1

        print(f"=== dispatch_v2 시작 (cid = {cid}) ===")
        print(f"초기 이벤트: {initial_event.payload_kind} on {initial_event.aggregate_id}")

        # BFS 큐: (cascade_depth, event)
        queue = deque([(0, initial_event)])
        staging = []

        # priority 오름차순으로 정렬된 transactional handlers
        sorted_handlers = sorted(TRANSACTIONAL_HANDLERS, key=lambda h: h[0])

        while queue:
            depth, event = queue.popleft()
            event.cascade_depth = depth

            print(f"\n[BFS pop] depth={depth} kind={event.payload_kind} agg={event.aggregate_id}")

            for prio, name, interest, handler_fn in sorted_handlers:
                if event.payload_kind not in interest:
                    continue
                print(f"   → 핸들러 실행: {name} (priority={prio})")
                follow_ups = handler_fn(event)
                for fu in follow_ups:
                    print(f"      └ follow-up 발행: {fu.payload_kind} on {fu.aggregate_id}")
                    queue.append((depth + 1, fu))

            staging.append(event)

        # Commit phase
        print(f"\n=== COMMIT 단계 ({len(staging)}개 이벤트) ===")
        committed = []
        for ev in staging:
            ev.id = self.event_store.next_id()
            ev.sequence = self.event_store.next_sequence(ev.aggregate_id)
            ev.correlation_id = cid
            self.event_store.append(ev)
            committed.append(ev)

        # Inline phase 시뮬레이션 (실행만 표시, 추가 이벤트 발행 없음)
        print(f"\n=== INLINE PHASE (priority 오름차순) ===")
        sorted_inline = sorted(INLINE_HANDLERS, key=lambda h: h[0])
        for ev in committed:
            for prio, name, interest in sorted_inline:
                if ev.payload_kind in interest:
                    print(f"   inline[{prio:2}] {name:35} ← {ev.payload_kind}")

        return committed


# ---------------------------------------------------------------------------
# 시뮬레이션 실행
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    dispatcher = CommandDispatcher()

    initial = DomainEvent(
        payload_kind="DialogueEndRequested",
        aggregate_id="npc:muback",
    )
    committed = dispatcher.dispatch_v2(initial)

    # 결과 표
    print("\n" + "=" * 70)
    print("  최종 EventStore 상태 (id 오름차순)")
    print("=" * 70)
    print(f"{'id':>3} | {'aggregate_id':28} | {'seq':>3} | {'cid':>3} | "
          f"{'depth':>5} | payload")
    print("-" * 110)
    for ev in committed:
        print(f"{ev.id:>3} | {ev.aggregate_id:28} | {ev.sequence:>3} | "
              f"{ev.correlation_id:>3} | {ev.cascade_depth:>5} | {ev.payload_kind}")
