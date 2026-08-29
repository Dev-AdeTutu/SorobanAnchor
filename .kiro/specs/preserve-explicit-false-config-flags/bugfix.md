# Bugfix Requirements Document

## Introduction

The boolean defaulting logic in `src/config.rs` uses a truthiness-style fallback
that collapses `None` and `Some(false)` into the same code path, replacing an
explicitly configured `false` with the documented default `true`. This means
operators cannot reliably disable security or feature flags (e.g.
`require_signature_verification`, `nonce_required`, `enable_metrics`) by setting
them to `false` in a config file — the runtime silently overrides the value.

The fix is a one-expression change: the defaulting path must distinguish between
`None` (field absent → apply default) and `Some(false)` (field present → honour
the explicit value), so that `false` is preserved as configured.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN an `Option<bool>` config field is set to `false` in the config file THEN the system applies the default `true`, discarding the operator's explicit opt-out.

1.2 WHEN the defaulting expression evaluates a `Some(false)` value with a truthiness-style fallback THEN the system treats it identically to `None` and substitutes `true`.

### Expected Behavior (Correct)

2.1 WHEN an `Option<bool>` config field is set to `false` in the config file THEN the system SHALL preserve `false` as the effective value without substitution.

2.2 WHEN an `Option<bool>` config field is absent from the config file THEN the system SHALL apply the documented default value (e.g. `true`) only for that `None` case.

### Unchanged Behavior (Regression Prevention)

3.1 WHEN an `Option<bool>` config field is set to `true` in the config file THEN the system SHALL CONTINUE TO use `true` as the effective value.

3.2 WHEN an `Option<bool>` config field is absent from the config file THEN the system SHALL CONTINUE TO apply the documented default value for that field.

3.3 WHEN non-boolean config fields are parsed and validated THEN the system SHALL CONTINUE TO behave identically to before the fix.

3.4 WHEN no configuration schema change is made THEN the system SHALL CONTINUE TO accept the same set of valid config files as before.
