# Plan: Pretty-print forward/backward in Effect Flow DAG

## Problem

The effect flow DAG displays raw Lean expression representations (e.g., `#0 * 3`) instead of pretty-printed Lean source code for the `forward` and `backward` interpretations.

## Current Status (after commit d834f7d)

The backend now uses `Option Expr` for forward/backward and pretty-prints them in Builder.lean.
However, two issues remain:

1. **`forward` is empty for some rules** - e.g., `pureRule` only sets `backward := valueExpr` but not `forward`
2. **`#0` instead of variable names** - de Bruijn indices are not resolved to human-readable names

## Remaining Fixes in `~/Code/lean-dag`

### Fix 1: Set both `forward` and `backward` in MonadPatterns.lean

The `pureRule` currently only sets `backward`. Update to set both:

```lean
/-- Rule for pure/return: `pure a` -/
def pureRule : EffectRule where
  name := `LeanDag.EffectFlow.MonadPatterns.pure
  priority := 80
  appliesTo := isPure
  analyze e _ := do
    let e := unwrapMdata e
    let args := e.getAppArgs
    let valueExpr := if args.size > 0 then some args[args.size - 1]! else none
    return some {
      kind := effectReturn
      forward := valueExpr   -- Pretty-print the returned value
      backward := valueExpr  -- Also show as backward interpretation
    }
```

Similarly for other rules:
- `stateGetRule`: could set `forward` to show `s` (state)
- `statePutRule`: set `forward` to the value being written
- `throwRule`: set `forward` to the error value

### Fix 2: Pass LocalContext to pretty-printer in Builder.lean

The `#0` issue is because `ppExprWithInfos` needs the `LocalContext` to resolve bound variable names (de Bruijn indices → names).

In `Builder.lean`, the `ppCtx` needs to include the local context from the TermInfo:

```lean
def effectNodeToGraphNode (node : EffectNode) (ppCtx : PPContext) (lctx : LocalContext) (text : FileMap)
    : IO GraphNode := do
  -- Use lctx-aware pretty-printing
  let ppCtxWithLctx := ppCtx  -- Need to incorporate lctx here
  let exprStr := (← ppExprWithInfos ppCtxWithLctx node.expr).fmt.pretty
  -- ...
```

The `PPContext` should be constructed with the correct `LocalContext`:

```lean
-- In computeEffectFlowDag or where PPContext is created:
let lctx := termInfo.lctx.sanitizeNames.run' {options := {}}
let ppCtx := ctx.toPPContext lctx
```

Check how `DataFlow/Builder.lean` handles this - it uses `ctx.toPPContext lctx` after extracting the local context from TermInfo.

### Fix 3: Store LocalContext with EffectNode (if needed)

If the expressions in forward/backward come from different contexts than the main expression, we may need to store the `LocalContext` alongside each expression in `EffectNode`.

## Frontend (lean-tui) Status

The frontend in `src/effect_flow/types.rs` correctly handles plain strings. No changes needed once the backend sends properly pretty-printed strings.

## Testing

After making the lean-dag changes:

1. Rebuild lean-dag: `lake build`
2. Test with `samples/monadic.lean`
3. Verify:
   - `fwd:` shows the value (e.g., `n * 3`) not empty
   - `bwd:` shows readable names (e.g., `n * 3`) not `#0 * 3`

