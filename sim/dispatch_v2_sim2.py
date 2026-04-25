"""시뮬레이션 2 — multi-depth cascade가 발생하는 경우

가정: Command::ApplyStimulus → StimulusApplied → BeatTransitioned (cascade) →
       (부수 효과로 EmotionAppraised 트리거) 식의 다층 cascade를 모사합니다.

실제 본인 시스템에서 StimulusPolicy가 BeatTransition을 follow-up으로 발행하고,
그게 다시 어떤 효과를 일으키는 패턴을 단순화한 것입니다.
"""
from collections import deque
from dataclasses import dataclass
from typing import Optional, List

@dataclass
class DomainEvent:
    payload_kind: str
    aggregate_id: str
    id: Optional[int] = None
    sequence: Optional[int] = None
    correlation_id: Optional[int] = None
    cascade_depth: int = 0
    handler_origin: Optional[str] = None


def stimulus_policy(ev):
    """StimulusApplyRequested → StimulusApplied (자기 follow-up) + 큰 변동 시 BeatTransitioned"""
    if ev.payload_kind == "StimulusApplyRequested":
        return [
            DomainEvent(payload_kind="StimulusApplied",
                        aggregate_id="npc:muback",
                        handler_origin="StimulusPolicy"),
            DomainEvent(payload_kind="BeatTransitioned",
                        aggregate_id="scene:muback<->okgyoryong",
                        handler_origin="StimulusPolicy"),
        ]
    return []


def emotion_policy(ev):
    """BeatTransitioned가 새 Focus를 활성화하면 → AppraiseRequested 트리거 (가상 시나리오)"""
    if ev.payload_kind == "BeatTransitioned":
        return [
            DomainEvent(payload_kind="AppraiseRequested",
                        aggregate_id="npc:muback",
                        handler_origin="EmotionPolicy(beat-trigger)"),
        ]
    if ev.payload_kind == "AppraiseRequested":
        return [
            DomainEvent(payload_kind="EmotionAppraised",
                        aggregate_id="npc:muback",
                        handler_origin="EmotionPolicy"),
        ]
    return []


def guide_policy(ev):
    """EmotionAppraised → 새 Guide 생성 follow-up"""
    if ev.payload_kind == "EmotionAppraised":
        return [
            DomainEvent(payload_kind="GuideGenerated",
                        aggregate_id="npc:muback",
                        handler_origin="GuidePolicy"),
        ]
    return []


TRANSACTIONAL_HANDLERS = [
    (10, "EmotionPolicy",  ["AppraiseRequested", "BeatTransitioned"], emotion_policy),
    (15, "StimulusPolicy", ["StimulusApplyRequested"],                stimulus_policy),
    (20, "GuidePolicy",    ["EmotionAppraised"],                      guide_policy),
]


class EventStore:
    def __init__(self):
        self.events = []
        self._next_id = 1

    def next_id(self):
        v = self._next_id
        self._next_id += 1
        return v

    def next_sequence(self, agg):
        return sum(1 for e in self.events if e.aggregate_id == agg) + 1


class CommandDispatcher:
    def __init__(self):
        self.event_store = EventStore()
        self.command_seq = 1

    def dispatch_v2(self, initial):
        cid = self.command_seq
        self.command_seq += 1

        print(f"=== dispatch_v2 (cid={cid}) ===")
        print(f"   초기: {initial.payload_kind} on {initial.aggregate_id}")

        queue = deque([(0, initial)])
        staging = []
        sorted_handlers = sorted(TRANSACTIONAL_HANDLERS, key=lambda h: h[0])

        while queue:
            depth, event = queue.popleft()
            event.cascade_depth = depth
            indent = "    " * depth
            print(f"\n{indent}[depth={depth}] {event.payload_kind} on {event.aggregate_id}")
            for prio, name, interest, fn in sorted_handlers:
                if event.payload_kind not in interest:
                    continue
                print(f"{indent}   handler: {name} (prio={prio})")
                fus = fn(event)
                for fu in fus:
                    print(f"{indent}      -> follow-up: {fu.payload_kind} on {fu.aggregate_id}")
                    queue.append((depth + 1, fu))
            staging.append(event)

        # Commit
        committed = []
        for ev in staging:
            ev.id = self.event_store.next_id()
            ev.sequence = self.event_store.next_sequence(ev.aggregate_id)
            ev.correlation_id = cid
            self.event_store.events.append(ev)
            committed.append(ev)
        return committed


if __name__ == "__main__":
    d = CommandDispatcher()
    initial = DomainEvent(payload_kind="StimulusApplyRequested",
                          aggregate_id="npc:muback")
    committed = d.dispatch_v2(initial)

    print("\n" + "=" * 90)
    print("  EventStore (id 오름차순)")
    print("=" * 90)
    header = f"{'id':>3} | {'aggregate_id':28} | {'seq':>3} | {'cid':>3} | {'dep':>3} | {'payload':22} | origin"
    print(header)
    print("-" * len(header) * 2)
    for ev in committed:
        origin = ev.handler_origin or "(initial cmd)"
        print(f"{ev.id:>3} | {ev.aggregate_id:28} | {ev.sequence:>3} | "
              f"{ev.correlation_id:>3} | {ev.cascade_depth:>3} | "
              f"{ev.payload_kind:22} | {origin}")
