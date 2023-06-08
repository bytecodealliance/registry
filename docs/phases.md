At this stage of the SIG Registries project, we've come to a general
agreement to focus on our [MVP user stories][user-stories].

As background, recall the WebAssembly CG defines a series of
[phases][cg-phases] for standardizing new proposals. Briefly, these
are:

1. Phase 0 (Pre-proposal)
1. Phase 1 (Feature proposal)
1. Phase 2 (Spec text)
1. Phase 3 (Implementation)
1. Phase 4 (Standardize)
1. Phase 5 (Standardized)

In the context of the current stage of the Registries SIG, Phases 4
and 5, focusing on standardization, are fairly far afield. Similarly,
Phase 0 is intended to pre-socialize subject matter, and is probably
not needing as an explicit phase for our current purposes.

Here, then, are the phases we will use, including exit criteria from
each phase, and what kinds of design inputs we are seeking in each
phase.

1. Phase 1 (creating a proposal)
    - Decide to work on a feature in support of specific user story(s)
    - People with a specific interest in the proposal (proposal participants)
      collaborate between group meetings to sketch out
      the general shape of the proposal with details of that
      don't impact the general shape being out of scope
    - Don't aim to define or fix all details
    - Updates on notable developments are given to the group
      during the regularly scheduled meeting
    - Design decisions will not be driven forward in the meeting
    - Exit: participants in the proposal design present
      the completed general shape to the group, and the
      entire group agrees on the general shape
    - Input:
        - no: bikeshedding on details
        - yes: on general shape
1. Phase 2 (requirements and details)
    - This is the point in the process at which to
      raise and discuss details of the design
    - Like in the previous phase, work happens by proposal
      participants outside the meeting, and progress is
      reported and discussed in the meeting
    - Exit: group agrees on the details spelled out in
      a design that fully enables implementations
    - Input:
        - no: general shape (this was fixed in Phase 1)
        - yes: all the design details
1. Phase 3 (implementation)
    - Engineers produce an implementation of the
      design
    - Input:
        - no bikeshedding of details fixed in Phases 1 and 2
        - yes to implementation-specific details

TODO: Agree on a mode of determining consensus

[user-stories]: https://docs.google.com/document/d/1QV0iXQBEqnE9CtNAhwH-oD7PBRnfeREj2nWZmw_zO8M/edit#heading=h.gqmuqgumciwt
[cg-phases]: https://github.com/WebAssembly/meetings/blob/main/process/phases.md
