---
name: charter
type: repo
---

# felt-overlay — charter

1. **This is a stretch/fun project, not production infrastructure.**
   Mario's own framing when this was first scoped: "if truly autism
   levels" — self-flagged as maybe-too-much, kept only because it's fun,
   not because it's committed to. Don't over-invest relative to that —
   see `minimality.md`.
2. **Correctness of the mechanics over polish of the content.** A janky
   placeholder sprite that reliably shows/hides/click-throughs is a
   success. A beautiful character that breaks click-through or steals
   focus is a failure, no matter how it looks.
3. **Never block input to whatever's underneath.** Click-through when
   idle/hidden is non-negotiable — this runs on someone's actual desktop
   while they're using it for other things.
4. **No always-on resource hog.** This should sit idle at near-zero CPU
   when hidden — it's meant to appear occasionally, not run a constant
   render loop nobody's looking at.
5. **Don't invent scope.** Build exactly what `repo.md`'s Phase 1
   describes. If something seems missing or wrong, say so — don't
   silently expand the task to "fix" it.
6. **Small, reviewable steps.** Whoever reviews this (a person, not you)
   needs to be able to actually follow what changed and why.
