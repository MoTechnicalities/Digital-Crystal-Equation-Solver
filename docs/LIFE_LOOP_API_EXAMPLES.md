# Life Loop API Examples (Issue-Ready)

This document provides copy/paste JSON request and response examples for the three Life Loop routes:
- `POST /v1/life-loop/tick`
- `GET /v1/life-loop/state`
- `GET /v1/life-loop/health`

Use these examples directly in GitHub issues, integration tickets, and API contract validation tasks.

## 1) `POST /v1/life-loop/tick`

Purpose:
- upsert one or more goals
- process exactly one deterministic loop cycle
- return action, outcome, identity, and health snapshot

### Request Example

```json
{
  "goal_updates": [
    {
      "goal_id": "goal-algebra-001",
      "description": "Maintain reliable linear equation solving",
      "priority": 90,
      "confidence": 0.72
    }
  ],
  "observation": {
    "kind": "linear_equation",
    "variable": "x",
    "a": 2.0,
    "b": -4.0,
    "note": "baseline deterministic solve",
    "timestamp_ms": 1710000000000
  },
  "simulate_only": false,
  "timestamp_ms": 1710000000000
}
```

### Response Example

```json
{
  "cycle_index": 3,
  "selected_goal_id": "goal-algebra-001",
  "action": {
    "kind": "solve_linear",
    "rationale": "observation carries linear coefficients; execute deterministic solver",
    "signature": "goal-algebra-001|LinearEquation|x|2|-4"
  },
  "outcome": "success",
  "solver_response": {
    "object": "csif.solver.result",
    "engine": "signed_i8_plus_intent_v2",
    "decision_label": "solved_linear_equation",
    "stop_reason": "PathFound",
    "solved_value": 2.0,
    "contradiction_metric": 0.0,
    "route_audit": {
      "selected_path": [
        "subtract_b",
        "divide_by_a"
      ],
      "checkpoint_signature": "route:subtract_b>divide_by_a",
      "explanation": "deterministic linear isolate"
    }
  },
  "identity": {
    "cycle_index": 3,
    "active_goal_count": 0,
    "completed_goal_count": 1,
    "adaptation_events": 0
  },
  "health": {
    "knowledge_score": 0.875,
    "auditability_score": 1.0,
    "success_ratio": 1.0,
    "contradiction_load": 0.0,
    "active_goal_count": 0,
    "adaptation_events": 0
  }
}
```

## 2) `GET /v1/life-loop/state`

Purpose:
- inspect full persisted life-loop state for replay, audits, and debugging

### Response Example

```json
{
  "cycle_index": 3,
  "goals": [
    {
      "goal_id": "goal-algebra-001",
      "description": "Maintain reliable linear equation solving",
      "priority": 90,
      "confidence": 0.77,
      "status": "completed",
      "success_count": 2,
      "failure_count": 1,
      "created_at_ms": 1709999999000,
      "updated_at_ms": 1710000000000
    }
  ],
  "episodes": [
    {
      "cycle_index": 2,
      "goal_id": "goal-algebra-001",
      "action": {
        "kind": "solve_linear",
        "rationale": "observation carries linear coefficients; execute deterministic solver",
        "signature": "goal-algebra-001|LinearEquation|x|2|-4"
      },
      "outcome": "success",
      "summary": "solved_linear_equation",
      "stop_reason": "PathFound",
      "solved_value": 2.0,
      "timestamp_ms": 1710000000000
    }
  ],
  "action_failures": {
    "goal-algebra-001|LinearEquation|x|0|4": 2
  },
  "adaptation_events": 1,
  "last_health": {
    "knowledge_score": 0.81,
    "auditability_score": 1.0,
    "success_ratio": 0.66,
    "contradiction_load": 0.33,
    "active_goal_count": 0,
    "adaptation_events": 1
  }
}
```

## 3) `GET /v1/life-loop/health`

Purpose:
- read current health/nurture dimensions without retrieving full episode history

### Response Example

```json
{
  "knowledge_score": 0.81,
  "auditability_score": 1.0,
  "success_ratio": 0.66,
  "contradiction_load": 0.33,
  "active_goal_count": 0,
  "adaptation_events": 1
}
```

## Suggested Issue Template Snippet

```markdown
### API Route
POST /v1/life-loop/tick

### Request JSON
(paste from docs/LIFE_LOOP_API_EXAMPLES.md)

### Expected Response JSON
(paste from docs/LIFE_LOOP_API_EXAMPLES.md)

### Determinism Assertions
- Same state snapshot + same request -> byte-stable response JSON
- Same state snapshot + same request -> identical next persisted state
```
