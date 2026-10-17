using System.Text.RegularExpressions;
using Cadetta.SolidWorks.Push;

namespace Cadetta.SolidWorks.Guard
{
    /// <summary>What the guard decided for one locator before any write.</summary>
    public enum GuardAction
    {
        /// <summary>The write may proceed unchanged.</summary>
        Allow,

        /// <summary>The write is refused (R010).</summary>
        Reject,

        /// <summary>The write replaces a native expression (R006), which is listed first.</summary>
        ReplaceExpression,
    }

    /// <summary>The guard's decision with the user-facing reason and, for R006, the expression being replaced.</summary>
    public sealed class GuardVerdict
    {
        /// <summary>The decision.</summary>
        public GuardAction Action { get; }

        /// <summary>The diagnostic code (R010 or R006), or null when allowed.</summary>
        public string? Code { get; }

        /// <summary>The message tail naming the condition, or null when allowed.</summary>
        public string? Reason { get; }

        /// <summary>The native expression the push would replace, or null.</summary>
        public string? OldExpression { get; }

        /// <summary>Creates a verdict.</summary>
        public GuardVerdict(GuardAction action, string? code, string? reason, string? oldExpression)
        {
            Action = action;
            Code = code;
            Reason = reason;
            OldExpression = oldExpression;
        }

        /// <summary>The verdict that lets a write through.</summary>
        public static GuardVerdict Allow { get; } = new GuardVerdict(GuardAction.Allow, null, null, null);
    }

    // frob:ticket 01M4SWMDS8MDHBS4T9JX1R3K6N
    /// <summary>Pre-write checks for a push: R010 for what cannot be written, R006 for a native expression a write would replace (adapters.md 2.1, 2.3).</summary>
    public static class PushGuard
    {
        /// <summary>Diagnostic code: the locator cannot be written.</summary>
        public const string R010 = "R010";

        /// <summary>Diagnostic code: a native expression is replaced.</summary>
        public const string R006 = "R006";

        // a plain number with an optional unit suffix, e.g. 6, 6mm, -1.5e2deg
        private static readonly Regex Literal = new Regex(@"^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?\s*[A-Za-z]*$", RegexOptions.Compiled | RegexOptions.CultureInvariant);

        // exactly one quoted name, e.g. "WIDTH"
        private static readonly Regex Quoted = new Regex("^\"([^\"]+)\"$", RegexOptions.Compiled | RegexOptions.CultureInvariant);

        /// <summary>True when an equation right-hand side is a plain number (with unit), so replacing it loses no structure.</summary>
        public static bool IsLiteral(string rhs) => Literal.IsMatch(rhs.Trim());

        /// <summary>The global named when the right-hand side is exactly one quoted name, else null.</summary>
        public static string? LinkTarget(string rhs)
        {
            var m = Quoted.Match(rhs.Trim());
            return m.Success ? m.Groups[1].Value : null;
        }

        /// <summary>The expression a push would replace for an equation right-hand side: null for a literal or a cadetta link, else the trimmed text.</summary>
        public static string? ForeignExpression(string rhs)
        {
            var t = rhs.Trim();
            return t.Length == 0 || IsLiteral(t) || LinkTarget(t) != null ? null : t;
        }

        /// <summary>Decides whether a value write to a dimension may proceed, and what it would replace.</summary>
        public static GuardVerdict ForDimension(string name, DimensionInfo info)
        {
            if (info.Driven) return Refuse("dimension " + name + " is driven (reference)");
            if (info.Suppressed) return Refuse("dimension " + name + " belongs to a suppressed feature");
            if (info.ReadOnly) return Refuse("dimension " + name + " is read-only (its file or reference cannot be edited)");
            if (info.Conflict) return Refuse("dimension " + name + " has an equation or unit conflict");
            if (info.Expression != null)
            {
                return new GuardVerdict(GuardAction.ReplaceExpression, R006, "dimension " + name + " was driven by the equation " + info.Expression + "; the equation is removed and the value set", info.Expression);
            }
            return GuardVerdict.Allow;
        }

        /// <summary>Decides whether a write to a global replaces a native expression (<paramref name="expression"/> null means none).</summary>
        public static GuardVerdict ForGlobal(string name, string? expression)
        {
            if (expression == null) return GuardVerdict.Allow;
            return new GuardVerdict(GuardAction.ReplaceExpression, R006, "global " + name + " was defined by the expression " + expression + "; it is replaced by a value", expression);
        }

        private static GuardVerdict Refuse(string reason) => new GuardVerdict(GuardAction.Reject, R010, reason, null);
    }
}
