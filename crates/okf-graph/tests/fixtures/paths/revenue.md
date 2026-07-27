---
type: Attested Computation
title: Revenue
runtime: bigquery
computation: references/revenue.sql
executor:
  resource: references/run.py
attester:
  resource: references/check.py
sources:
  - resource: https://wiki.example/revenue
  - resource: all queries in BigQuery project X
  - resource: /overview.md
---

# Definition

Recognized revenue, computed by the referenced query.
