---
paths:
  - "src/action/caller.rs"
  - "src/permission.rs"
---

# Tenancy plan

`Caller` has a `workspace` (`Workspace::Personal` today, so `owner_id()` is
the user's id). Teams add `Workspace::Organization { id, role }`, roles
beyond `Owner` in `WorkspaceRole::grants`, an `org_id` on owned tables and a
membership check where `Caller` is built; actions don't change because they
already ask for `owner_id()` and a `Permission`.
