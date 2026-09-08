---
name: simplify
description: >-
  Clean up changed code while preserving intended behavior. Use when the user
  asks to simplify or improve code quality through reuse, simplification,
  efficiency, and abstraction reviews. Correctness bug hunting is outside scope.
---

# Simplify

Improve the quality of changed code while preserving intended behavior. Focus on cleanup; correctness bug hunting is outside this skill’s scope.

Assume a Git codebase and an available subagent tool.

## Establish scope

Honor the user’s explicit target; otherwise review changes relevant to the current task. Choose comparison revisions from repository and conversation context. Include staged, unstaged, and relevant untracked changes.

Use this scope consistently across reviewers. Read surrounding code as needed to understand behavior and find existing implementations.

## Review with subagents

Assign each lens below to an independent subagent. Run reviews in parallel within available capacity.

Give every reviewer the same scope, access to the diff, and relevant task context. Reviewers return findings without editing files.

- **Reuse:** Identify new code that duplicates existing functionality. Search shared utilities and adjacent code, and name the existing implementation to use.
- **Simplification:** Identify redundant or derivable state, repeated logic, unnecessary nesting or indirection, and dead code introduced or left behind by the changes. Name the simpler equivalent.
- **Efficiency:** Identify wasted computation, repeated I/O, unnecessary retention, or avoidable blocking. Explain the cost in its actual execution context and name a cheaper alternative.
- **Abstraction:** Check whether the change belongs at the right layer. Identify special cases that would become unnecessary by improving the underlying mechanism. Propose a deeper change only when it reduces overall complexity.

Each finding should include `file`, `line`, a short summary, the concrete cost, and a proposed improvement. Return no findings when no worthwhile improvement is supported.

## Apply and verify

Wait for all reviews. Deduplicate findings that concern the same code or underlying mechanism, then validate the remaining findings against the surrounding code and intended behavior.

Apply worthwhile improvements within scope. Skip false positives, changes to intended behavior, and proposals requiring broader work. If the user requested review only, report proposals instead of editing.

Run checks appropriate to the changes. Finish with a brief summary of what improved, what was meaningfully deferred or skipped, and what verification completed.
