# Working knowledge

Working knowledge is the last resort. Add an entry only when all four hold:

1. More than one ticket or later work needs it.
2. No home in [What goes where](map-rules.md#what-goes-where) owns it, including the brief, design, findings or checkpoint of the ticket it concerns.
3. It is current, not history or progress.
4. It fits in a few sentences — the CLI refuses entries over 1,000 characters.

Say where the entry applies and cite supporting tickets or artifacts; evidence references are citations, not a checklist of files to reread. When its content later lands in its owning home, archive the entry rather than keeping a copy. Agents maintain entries through the CLI.

The execution projection shows a sample and the active count; use `knowledge get` for the rest. `appliesTo` says which handoffs should include an entry. Active knowledge does not block completion.

Writes take one JSON object, from a file passed with `--input <file>` or on stdin, and require `--expect <map revision>`, the compare-and-swap; `get` needs neither.

```
wayfinder knowledge get [--id k-xxxxxx] [--status active|archived|all]
wayfinder knowledge add      --expect <revision> --input <file>  # {text,appliesTo?,ticketIds?,artifacts?,reason?}
wayfinder knowledge edit     --expect <revision> --input <file>  # {id,text,appliesTo?,ticketIds?,artifacts?,reason}
wayfinder knowledge archive  --expect <revision> --input <file>  # {id,reason}
wayfinder knowledge restore  --expect <revision> --input <file>  # {id,reason}
wayfinder knowledge merge    --expect <revision> --input <file>  # {id,sourceIds,text,appliesTo?,reason}
wayfinder knowledge from-fog --expect <revision> --input <file>  # {fogIds,text,appliesTo?,ticketIds?,artifacts?,reason}
```

`get --id` returns one entry with its full history. Preserve evidence when correcting or merging; archive only with a concrete reason. `from-fog` converts only the fog ids you name and preserves the original fog as provenance. Never bulk-convert entries by wording or age.

When resolving work that changes settled facts, review the affected knowledge: correct refuted statements, merge true duplicates, archive demonstrably obsolete guidance, and ticket actionable follow-ups before removing their reminders. Retain unrelated active knowledge without re-investigating it. Record material corrections on the owning ticket: its brief while open, an amendment once resolved. Cleanup does not authorize changing product decisions or hiding inconvenient constraints.
