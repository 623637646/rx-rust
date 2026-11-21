# Domain Docs

How the engineering skills should read and write this repo's domain documentation.

## Before exploring, read these

- **`GLOSSARY.md`** at the repo root, or
- **`GLOSSARY-MAP.md`** at the repo root if it exists: it points at one `GLOSSARY.md` per context. Read each one relevant to the topic.
- **`docs/decisions/`**: read decision records that touch the area you're about to work in. This repository keeps accepted and rejected design decisions here.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

Single-context repo:

```
/
├── GLOSSARY.md
├── docs/decisions/
│   └── <numbered design decisions>
└── src/
```

## Write decision records

The `docs/adr/` path in engineering skills maps to `docs/decisions/` for both reads and writes in this repository.

For a new topic, scan the existing records and use the highest number plus one, with a four-digit prefix: `NNNN-slug.md`. Follow the [decision-record conventions](../decisions/README.md) for content, status and history, and add the new record to that index.

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `GLOSSARY.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal: either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag decision conflicts

If your output contradicts an existing design decision, surface it explicitly rather than silently overriding it.
