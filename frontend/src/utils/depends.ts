/**
 * Evaluate a depends_on / mandatory_depends_on / read_only_depends_on expression.
 *
 * If the expression starts with "eval:", the remainder is evaluated as a JS expression
 * with `doc` in scope. Otherwise, the expression is treated as a field name and the
 * result is the truthiness of `doc[expr]`.
 */
export function evaluateDependsOn(
  expr: string,
  doc: Record<string, unknown>,
): boolean {
  if (!expr) return false;

  if (expr.startsWith("eval:")) {
    const code = expr.slice(5);
    try {
      return !!new Function("doc", `return (${code})`)(doc);
    } catch {
      return false;
    }
  }

  // Simple field name — truthy check
  return !!doc[expr];
}
