# ADR 0012: Action options as declarative data

Status: accepted (owner approval 2026-11-04; implemented in S-117)
Date: 2026-11-04

## Context

Kawaro actions carry their values in `[action.params]`, an untyped `toml::Table` baked into `actions/*.toml`. Nothing in the hive, the CLI or any agent can change one value for one run: `Request::Run` carries an action id and files only. This single gap blocks three things at once: the customization UI the owner asked for, `kawaro run ACTION --set key=value`, and the `options` argument of the MCP tools (ADR 0011). The research is in `docs/research/agent-integration-and-expansion.md` (section 5) and `docs/research/image-editing-and-preview.md`.

Facts from the repo at 0.5.0:

- 45 actions across `actions/*.toml`, parsed by `Registry` and validated by `action_impl::validate`.
- The UI builds the hive from the registry, the CLI lists actions, and the daemon runs the same `ActionDef` through the queue. Only `core` reads action TOML today.
- The engines read `action.params` as a table and validate their own values (quality, scale, dpi and so on).
- A future preview and stage 2 UI need to know the type, range, unit, labels, choices and presets of every field, not just one baked value.
- The core crate already depends on `serde`, `serde_json`, `toml` and `thiserror`, so the schema and the JSON export add no dependency and no MSRV change.

The contract in this block is S-117. It deliberately stops at data and core. The stage 2 UI, the live preview and the IPC change are separate stories (S-118, S-119, S-120).

## Options

| Option | Cost | What it gives | Verdict |
| --- | --- | --- | --- |
| Keep `[action.params]` fixed and add one action per preset | Catalog growth, duplicated ids, no per-run values | Nothing an agent can use | Rejected |
| Describe fields in Rust code, one builder per action | Typing and validation for free | Actions are data shipped as TOML, and a user action file could not declare its own fields; the UI and CLI would read Rust, not the registry | Rejected |
| Declarative `[action.options]` in the same action TOML, parsed and validated by core | New module in core, no dependency, one file per action | One source of truth for UI, CLI and MCP; user actions can carry options; JSON Schema export is mechanical | Chosen |
| A sidecar schema file per action (for example `image.to-webp.schema.json`) | Two files to keep in step | Easier validation of creative shapes | Rejected: drift is the failure mode we are removing, and an action is one file today |
| Build the UI first with ad hoc widgets per action | Visible value soonest | The CLI and MCP would each invent their own parsing, which is exactly the drift this record prevents | Rejected: data first |

## Decision

1. **One optional `[action.options]` table in the same TOML file as the action.** `[action.params]` stays the values a plain click runs. The options table describes what may change; it does not replace the params table. `ActionDef` gains `pub options: Option<OptionsSpec>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, so an action without options serializes exactly as before.

2. **A closed type set.** `kind` is `none` or `form`, and a field type is exactly one of `percent`, `px`, `px-pair`, `enum`, `bool`, `int`, `float`, `string`, all in kebab-case. An unknown type is an action load error, never a free text field. The set is closed on purpose: a new type needs a new value validator and a new UI control, so it deserves a review.

3. **Fields are data.** A field carries `id`, `type`, `label` (Arabic and English), and optionally `default`, `min`, `max`, `step`, `unit`, `values` (enum choices with bilingual labels), `maps_to`, `clears` and `no_enlarge`. The spec carries `kind`, `readout` tokens, `remember`, `suffix_template`, the fields and the presets. Presets are data too: an id plus a table of field values.

4. **Merge semantics: base params, then explicit values, then clears.** `resolve(base, explicit_fields)` starts from a clone of the base table. For every field that appears in the explicit table it validates the value, removes every key in `field.clears` from the output and inserts the value under `field.param_key()` (which is `maps_to` when set, `id` otherwise). An explicit key that matches no field is `OptionsError::Unknown`. `resolve_params` is the same operation for callers that name explicit values by param key; it also accepts a field id. When a key could match one field's id and another field's param key, `resolve` prefers the id and `resolve_params` prefers the param key, so each caller gets the reading its contract promises.

5. **Defaults are not applied by `resolve`.** The base `[action.params]` table is the only source of run defaults, and it is what a plain click already uses. A field `default` exists for the JSON Schema and for the future UI (a form opens with the default in the control); `resolve` never invents a value that the base does not have. This avoids two competing default sources and makes a resolved table traceable to the base plus the caller's explicit values.

6. **Validation lives in core, once.** `resolve` checks the TOML type of every explicit value, the range when `min` or `max` is set (the reason contains `below` or `above`), enum membership, and the `px-pair` pattern `^[1-9][0-9]*x[1-9][0-9]*$`. `OptionsSpec::validate` checks the spec itself at load: `kind = "none"` allows no fields and no presets; field ids are unique and non empty; enum fields have values and a default that is one of them when a default is present; a default matches its kind and its range; `min <= max`; two fields cannot share a param key; a `clears` entry cannot name the field's own param key; preset keys are field ids and pass the same value rules. A bad spec fails the action load as `RegistryError::Parse` and the message carries the action id. Note that this rule makes `kind = "form"` mandatory for every options block that declares a field, including a block with a single quality field.

7. **JSON Schema is an export, not a second contract.** `OptionsSpec::params_schema()` returns `{"type":"object","properties":{...},"additionalProperties":false}` with property keys equal to param keys. Types map as `int`, `percent` and `px` to `integer`, `float` to `number`, `bool` to `boolean`, `enum` to a string with an `enum` of the choice ids, `string` to a string, and `px-pair` to a string with the pattern. `minimum`, `maximum`, `default` (converted from TOML) and a `description` made of the English label plus `(unit)` when a unit is set are included. There is no `required` list: every field is optional and the base params carry the values.

8. **One source of truth for the UI, the CLI and MCP.** Only `core` parses action TOML. The MCP tool input schema is this export, the CLI `--set` runs `resolve_params`, and the stage 2 UI will build controls from the same fields. No consumer keeps a second copy of the option list.

9. **No UI in this block.** S-117 writes the data model, the validation, the export and the first built-in specs. The hive, the settings window, the daemon and `Request::Run` are untouched. The stage 2 UI and the live preview are S-119 on top of the preview worker S-118; the additive `Request::Run` params change and the CLI `--set` and `--schema` surface are S-120.

10. **The wire change is additive and comes later.** `Request::Run` gains `params: Option<toml::Table>` with `#[serde(default)]` in S-120, the daemon merges it and rejects an unknown key as `BadParam`, and an old client that omits the field keeps working. S-117 deliberately does not touch IPC, so the daemon cannot yet run a customized action.

11. **Declared but not enforced in this block.** `step` is schema data for a future slider, `no_enlarge` is declared for the resize and crop work, `readout` names what a future preview must show, `remember` says the UI may remember the last values per action, and `suffix_template` is for output naming rendered from resolved values. `resolve` reads none of them; only `validate` and the schema export do. The built-in specs in S-117 use only the parts that have a consumer today.

12. **The first built-in specs are small and real.** `image.to-webp`, `image.to-jpg`, `image.to-avif` and `image.compress` declare an integer `quality` (1 to 100, the current param as default, unit `%`). `image.resize-50` is the first `form` spec: readout `dimensions`, `ratio`, `percent`, `output_name`, and two fields, `scale` (percent, clears `max_side`) and `max_side` (px, clears `scale`). `pdf.compress` declares the Ghostscript preset (screen, ebook, printer, prepress, default ebook) and `pdf.to-images` declares `dpi` (72 to 600, default 150). Every label and choice carries Arabic and English. No other action changes.

## Consequences

- The UI, the CLI and MCP can share one description of a per-run value without a second parser. A new action option is one TOML block, not three code changes.
- User action files can declare their own options with the same rules, because the schema lives in the action file and is validated at load.
- The change is compatible in both directions: an action without options serializes as before, and no IPC message changes in this block.
- The option model is flat in this block. There is no `visible_when` (it was proposed in the research and is not in the contract), no cross-field rule beyond `clears`, and no arithmetic dependency between fields. A future "keep ratio" checkbox needs an explicit extension, not a TOML trick.
- `resolve` returning the base untouched when nothing is explicit is a feature, but a caller that passes an empty base gets an empty result. MCP and the CLI must pass `action.params` as the base; that obligation is in S-121 and S-120.
- The engine still receives an untyped `toml::Table` and keeps its own validation. A value can pass the option schema and still be rejected by the engine; that is deliberate, because only the engine knows its real limits (for example AVIF quality floor).
- The closed type set will pinch as soon as crop, page ranges or target file size arrive. Widening it is an ADR amendment plus a UI control, which is the intended review point.
- A broken `[action.options]` in a user action file fails that file's load with the action id in the message. The daemon already ignores a failed user action load with a warning, so the built-ins stay available.
- No UI means the owner cannot see or change an option from the hive yet; the data is in place for S-119, and MCP is the first visible consumer.

## Risks

1. **Schema drift.** The whole point is one parser. A consumer that reads `actions/*.toml` directly, or copies the type mapping, reintroduces the problem; review must reject that.
2. **Default confusion.** Two defaults exist by design (`[action.params]` for a run, field `default` for the schema and the form). A run that shows the form value and then applies the param value, or the reverse, is a bug report. S-119 must resolve explicit values, never apply field defaults.
3. **Contract creep in the schema.** `kind = "none"` with fields is invalid, so every block with a field must say `kind = "form"`. A reviewer who reads only "optional table" may miss it.
4. **Error text as API.** The `OptionsError` display strings are asserted by tests and read by MCP users; changing them is a breaking change for tests and for agent prompts.
5. **JSON Schema consumers.** The schema has no `required`, no `title` and no `examples`. Some hosts validate strictly, so the export must stay valid JSON Schema draft 2020-12; the tests parse it, not just compare strings.
6. **Step and no_enlarge are dead data this block.** They are declared because the storage is stable; if the stage 2 semantics differ the fields change with it.

## Deferred

- The hive customization UI, the readout, presets and remembering: S-119, after the preview worker S-118.
- Live preview and shared image geometry between preview and engine: S-118, consumed by S-119.
- `Request::Run` with params, daemon merge and `BadParam`, and the CLI `kawaro run --set K=V` and `kawaro actions --id X --schema`: S-120.
- `visible_when`, cross-field validation, step enforcement, per-file options, and arithmetic fields.
- Rendering `suffix_template` into output names.
- Typed params at the `Engine` boundary.
