name: Feature request
description: Propose a new capability for AURA
title: "[feature] "
labels: [enhancement]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem
      description: What problem would this feature solve?
    validations:
      required: true
  - type: textarea
    id: proposal
    attributes:
      label: Proposed solution
    validations:
      required: true
  - type: textarea
    id: spec
    attributes:
      label: Spec impact
      description: Does this change RFC 001? Link the relevant section.
    validations:
      required: false
