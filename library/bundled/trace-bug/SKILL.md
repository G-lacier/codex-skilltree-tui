---
name: trace-bug
description: Investigate a reproducible software bug or regression by tracing the first incorrect result to its cause and verifying a correction.
---

# Trace a bug

Use the project map to locate the failing path. Establish the input, expected result, actual result, and a repeatable reproduction.

Compare intermediate values with the expected behavior. Trace the first divergence to its producer, checking parsing, configuration, state transitions, and asynchronous boundaries as relevant. Compare a working case when it helps isolate the difference.

Make a correction supported by the observed cause. Repeat the reproducer and run relevant checks. Add regression coverage for the failing behavior when appropriate, then report the cause, correction, and verification results.
