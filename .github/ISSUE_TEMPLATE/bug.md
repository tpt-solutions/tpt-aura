name: Bug report
description: Report a defect in the AURA implementation
title: "[bug] "
labels: [bug]
body:
  - type: textarea
    id: what
    attributes:
      label: What happened?
      description: A clear and concise description of the bug.
    validations:
      required: true
  - type: textarea
    id: repro
    attributes:
      label: Steps to reproduce
      description: How can we reproduce the problem?
    validations:
      required: true
  - type: textarea
    id: expected
    attributes:
      label: Expected behavior
    validations:
      required: true
  - type: input
    id: toolchain
    attributes:
      label: Rust toolchain / OS
      placeholder: "rustc 1.97, Windows 11"
    validations:
      required: false
