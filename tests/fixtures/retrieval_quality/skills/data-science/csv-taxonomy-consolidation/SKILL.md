---
name: csv-taxonomy-consolidation
description: Consolidate multiple CSV spreadsheets into a normalized taxonomy with deduplication, column mapping, hierarchy alignment, and audit reports.
tags: [csv, taxonomy, spreadsheet, deduplication, pandas, hierarchy]
---
# CSV Taxonomy Consolidation
## When to Use
Use when the user has multiple CSV files or spreadsheets that need schema mapping, duplicate merging, taxonomy hierarchy cleanup, or reproducible audit reports.
## Do Not Use When
Do not use for plotting figures, PDF conversion, GitHub workflow work, or prose rewriting.
## Required Inputs
Input CSV paths, key columns, target taxonomy schema, merge rules, and output format.
## Workflow
1. Profile columns, encodings, row counts, and duplicate keys.
2. Map source columns to a canonical taxonomy schema.
3. Merge records with deterministic conflict rules and hierarchy validation.
4. Write consolidated CSV plus an audit report.
