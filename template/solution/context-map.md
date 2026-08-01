---
provenance: authored
owner: <owner>
---
# Context map

Internal bounded contexts and external contexts, with DDD relations. External
contexts include vendors that provide systems; how their artifacts resolve is the
instance's tooling, not the framework's. Each relation is backed by the language
it claims (lint item 23): `published-language` — the publishing context's
`contracts/` holds it; `conformist` / `acl` — `notes` carries a pinned ref to
the upstream language conformed to or translated; `customer-supplier` — the
interface may still be prose, hatched when work touches the seam.

| context | kind     | relation                                                  | notes |
|---------|----------|-----------------------------------------------------------|-------|
| {bc}    | internal | —                                                         |       |
| {vendor}| external | conformist / customer-supplier / acl / published-language | <pinned ref to the upstream language, for conformist/acl> |
