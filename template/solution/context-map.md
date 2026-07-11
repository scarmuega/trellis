---
provenance: authored
owner: <owner>
---
# Context map

Internal bounded contexts and external contexts, with DDD relations. External
contexts include vendors that provide systems; how their artifacts resolve is the
instance's tooling, not the framework's.

| context | kind     | relation                                                  | notes |
|---------|----------|-----------------------------------------------------------|-------|
| {bc}    | internal | —                                                         |       |
| {vendor}| external | conformist / customer-supplier / acl / published-language |       |
