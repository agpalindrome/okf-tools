---
type: Attested Computation
title: Revenue
runtime: bigquery
parameters:
  - { name: year, type: integer, required: true }
---

# Computation

```sql
SELECT SUM(amount) AS revenue FROM finance.recognized_revenue WHERE fiscal_year = @year
```
