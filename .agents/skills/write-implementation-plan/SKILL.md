---
name: write-implementation-plan
description: Create or revise a concise, repository-grounded implementation plan as a self-contained HTML document in `.plans/`. Use when Codex needs to turn a discovery or grilling discussion into an engineer-ready technical plan, document current and target behavior, define spec requirements, design architecture, split work into reviewable PR increments, or improve an existing plan artifact.
---

# Write Implementation Plan

Produce a technical handoff that helps the user align on what will be built and judge whether another agent can implement it. Write for an engineer reading a plan, not for a customer evaluating a product.

Do not implement the product while producing the plan unless the user separately asks for implementation.

## Follow the planning workflow

### 1. Recover the decisions

- Read the full discovery or grilling discussion and any plan already in progress.
- Separate agreed decisions, unresolved questions, constraints, and ideas that were rejected.
- Preserve the user's language where it names a product concept clearly.
- Ask only questions whose answers would materially change the plan. Batch related questions instead of asking them one at a time.

### 2. Ground the plan in the repository

- Read repository instructions, architecture documents, decision records, module maps, relevant source files, and existing tests before proposing a design.
- Use `rg` and `rg --files` to trace the current behavior, data ownership, entry points, persistence, and dependency direction.
- Reuse the repository's names for domains and concepts.
- Identify the owning domain for every new behavior. Keep adapters and entry points thin.
- Name only files and modules that exist or that the plan explicitly proposes creating. Do not invent paths based on convention alone.
- Call out any conclusion that remains an inference rather than a verified fact.

### 3. Draft the required sections

Include each section once:

1. **Overview**
   - State the outcome, fixed decisions, important constraints, and out-of-scope work.
2. **Behavior change**
   - Explain how the system behaves today.
   - Explain how it will behave after the changes.
   - Describe differences through observable behavior, not feature claims.
3. **Technical design**
   - Show domain ownership, dependency direction, data flow, lifecycle, and trust boundaries.
   - Add a diagram only when it makes one of those relationships easier to understand.
   - Explain the diagram in normal prose.
4. **Requirements**
   - Give each testable requirement a stable identifier.
   - Write the requirement as a natural explanation of what the system must do and what the user sees.
5. **Implementation sequence**
   - Split the work into small, ordered increments that could each be a pull request.
   - Preserve the sequence even when the user may implement all increments together.
6. **Risks and constraints**
   - Cover the real technical limits, migrations, compatibility concerns, trust assumptions, and failure modes.
7. **Acceptance journeys**
   - Describe how a person can verify the result from start to finish.

### 4. Review the technical plan

Before finalizing, ask two or three independent subagents to inspect bounded parts of the proposed design when the environment permits it. Choose reviewers that match the change, such as backend architecture, UI behavior, migration, filesystem safety, or external integration.

Give reviewers the source material and proposed plan, not a leading diagnosis. Reconcile their findings against the user's decisions and the repository. Keep control of the final design; do not let a reviewer expand the goal or replace an agreed product decision.

If subagents are unavailable, perform separate architecture and user-journey review passes yourself and disclose that limitation.

At minimum, review for:

- domain ownership and dependency direction;
- lifecycle, concurrency, and partial-failure behavior;
- migration and backward compatibility;
- permissions, ownership checks, and trust boundaries;
- support for every client or platform in scope;
- consistency between requirements, PR increments, and acceptance journeys.

Adjust the plan for verified issues. Leave speculative or out-of-scope suggestions out.

### 5. Write the HTML artifact

Save the document as `.plans/<topic>-plan.html`. Revise the existing file when the user is iterating on a plan.

Make it:

- static and self-contained, with no external fonts, scripts, images, or stylesheets;
- dark, restrained, responsive, accessible, and printable;
- easy to scan with a compact table of contents and clear section hierarchy;
- comfortable to read at desktop and narrow widths;
- free of decorative motion and visual elements that do not carry information.

Use tables for exact mappings and diagrams for architecture or sequence. Prefer prose and short lists when the relationship is already simple.

### 6. Validate the artifact

- Open the saved file in the Codex in-app browser unless the user requests another browser.
- Inspect desktop and narrow layouts.
- Check navigation, anchors, text wrapping, tables, diagrams, horizontal overflow, focus states, and print behavior.
- Check the console for errors.
- Correct any problem and reload the saved artifact before handing it off.

## Write like an engineer

Use direct, natural sentences that sound like one engineer explaining an agreed design to another.

Avoid:

- marketing slogans, feature pitches, oversized heroes, and claims about how impressive the feature is;
- theatrical headings such as “A managed retrieval conduit” when “Technical design” is clearer;
- compressed specification language that sounds robotic;
- unexplained domain jargon;
- repeating the same decision in an overview, card, requirement, and callout.

Prefer:

> Just Notes changes a guide folder only when it finds the ownership file it created. If the folder belongs to something else, the app leaves it untouched and shows a conflict in Settings.

Avoid:

> A valid manifest proves ownership. Existing unowned files, directories, or foreign symlinks become Conflict and are never overwritten.

Conciseness means removing repetition and decorative copy. It does not mean removing the subjects, verbs, or context that make a sentence human and clear.

## Define useful PR increments

For every increment, include:

- **Outcome:** what becomes possible or safe after the increment;
- **Scope:** the behavior added or changed;
- **Abstractions:** the high-level domain, service, state, or adapter being introduced or extended, including what it owns;
- **Files:** the existing files to update and the new or obsolete files to create or remove.

Keep this at design level. Do not include function signatures or line-by-line implementation instructions.

Do not add a repeated “Proof” or test column to every increment. Put shared verification commands once, then cover meaningful results through the acceptance journeys.

## Specify acceptance as journeys

Write each acceptance case with:

- **Action:** what the user or developer does;
- **How to verify:** the visible check, test, or inspection;
- **Expected outcome:** the state or behavior that proves the increment works.

For user-facing work, follow the real journey through the interface. For backend-only work, describe the developer setup, how to exercise the boundary, and the expected observable result. Do not disguise internal test cases as a user journey.

## Final check

Before handing off the plan, confirm that:

- every agreed decision appears once and no rejected idea has returned;
- current behavior is supported by repository evidence;
- target behavior, non-goals, and failure behavior are explicit;
- the design names domain ownership and preserves dependency direction;
- requirements are identifiable, testable, and written naturally;
- each increment names its abstractions and files;
- acceptance journeys cover both user-facing and backend-only changes;
- reviewers' valid corrections are incorporated;
- the HTML has been checked in the browser at desktop and narrow widths;
- a future agent can implement the work without guessing at scope or architecture.
