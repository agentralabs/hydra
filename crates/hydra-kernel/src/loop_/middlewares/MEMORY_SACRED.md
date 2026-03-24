# SACRED — DO NOT MODIFY WITHOUT READING THIS ENTIRE FILE

## This Memory Middleware Took 8.6/10 Behavioral Score From 5.5 to 9.0

The memory middleware in `memory.rs` contains 3 inventions that prevent LLM hallucination. They were calibrated through 6+ harness runs. Every word in the EMI/NEC templates is load-bearing. Changing a single sentence can drop the behavioral score by 6 points.

## What Will Break If You Edit Carelessly

### The EMI Template (lines ~46-70)
The "CRITICAL DISTINCTION" block contains 6 rules. Each one fixes a specific hallucination pattern:

| Rule | What It Prevents | Harness Score Without It |
|---|---|---|
| "PRIOR sessions with UNKNOWN users" | LLM claiming personal history with current user | 2.5/10 on mem-a1 |
| "You do NOT know this user personally" | LLM fabricating relationship patterns | 2.5/10 on mem-j1 |
| "GENERAL KNOWLEDGE, NOT personal history" | LLM presenting evidence as shared experience | 6.5/10 on mem-j1 |
| "If asked 'what have WE discussed'" | LLM inventing conversation topics | 1.0/10 on mem-f1 |
| "NEVER say 'based on our previous conversations'" | LLM using relationship language | 2.5/10 on mem-a1 |
| Evidence labeled as "general knowledge" | LLM attributing evidence to wrong source | 7.5/10 on mem-f1 |

### The NEC Template (lines ~72-85)
The Null Evidence Certificate fires when memory returns zero results. Without it:
- LLM fills the void with fabricated history
- Score drops from 9.0 to 1.5 on memory questions

### The Session-Bounded Evidence
Prior-session evidence is labeled distinctly from current-session evidence. Without this distinction, the LLM cannot tell what happened TODAY vs what happened LAST MONTH.

## Before Making ANY Change

1. Read every line of the EMI template. Understand what each sentence prevents.
2. Read the NEC template. Understand why it exists.
3. Run the behavioral harness BEFORE your change: `cargo run -p hydra-harness --bin harness_v2 -- --hours 1`
4. Record the memory scores (mem-f1, mem-a1, mem-j1).
5. Make your change.
6. Run the harness AFTER.
7. If ANY memory score drops by more than 1.0 point, REVERT.

## The Numbers That Matter

```
Before all inventions:  mem-f1: 1.0  mem-a1: 1.5  mem-j1: 2.5  avg: 1.7
After EMI only:         mem-f1: 8.5  mem-a1: 2.5  mem-j1: 7.5  avg: 6.2
After EMI + NEC:        mem-f1: 8.5  mem-a1: 8.5  mem-j1: 9.0  avg: 8.7
After final tuning:     mem-f1: 9.0  mem-a1: 9.0  mem-j1: 9.0  avg: 9.0
```

Each step took multiple harness runs to calibrate. The wording is not arbitrary.

## Safe Changes

- Adding NEW enrichment keys (not modifying existing ones)
- Changing the number of results retrieved (currently 8)
- Adding new checks AFTER the EMI/NEC blocks
- Performance optimizations that don't change the template text

## Unsafe Changes

- Changing ANY word in the EMI "CRITICAL DISTINCTION" block
- Removing or weakening the NEC template
- Changing "UNKNOWN users" to anything else
- Removing "NEVER say 'based on our previous conversations'"
- Changing the evidence labeling from "general knowledge" to anything else
- Moving the memory injection from Tier 0 to a lower tier in prompt.rs

## If You Must Change It

Create `specs/CONTINUATION-MEMORY.md` with:
1. What you want to change and why
2. Current harness scores (run it)
3. Predicted impact on each memory question
4. Rollback plan if scores drop

Then change it. Then verify. This is not optional.
