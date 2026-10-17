using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Text.Json;
using Cadetta.SolidWorks.Guard;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Save;

namespace Cadetta.SolidWorks.Push
{
    /// <summary>
    /// The `push` step for one document: guards, writes globals, dimensions,
    /// properties and frames, rebuilds and restores on R004, and returns a status
    /// per locator (adapters.md 2.3, 3.4, 3.6, 4; solidworks.md 4, 5).
    /// </summary>
    // frob:ticket 01M4SV2TR8ENTA87J1DDKCF816
    public sealed class PushService
    {
        /// <summary>Diagnostic code: document dirty, locked or read-only.</summary>
        public const string R009 = "R009";

        /// <summary>Diagnostic code: driven or suppressed dimension, equation or unit conflict.</summary>
        public const string R010 = "R010";

        /// <summary>Diagnostic code: bound item missing natively.</summary>
        public const string R003 = "R003";

        /// <summary>Diagnostic code: a push replaced a native expression.</summary>
        public const string R006 = "R006";

        private readonly Log _log;

        /// <summary>Creates a service logging to <paramref name="log"/>.</summary>
        public PushService(Log log)
        {
            _log = log;
        }

        private sealed class Entry
        {
            public ChangedParam Param = null!;
            public NativeKind Kind;
            public string Name = "";
            public PushNumber? Number;
            public string? Text;
            public double[]? Frame;
            public string? FrameJson;
            public DimTolerance? WireTolerance;
            public bool Done;
            public LocatorStatus? Preset;
        }

        private sealed class Run
        {
            public IPushDocument Doc = null!;
            public PushOptions Options = null!;
            public PushOutcome Out = null!;
            public List<Action> Undo = new List<Action>();
            public HashSet<string> GlobalsHandled = new HashSet<string>(StringComparer.Ordinal);
            public bool Wrote;
        }

        /// <summary>Pushes <paramref name="changes"/> into <paramref name="doc"/>; every locator gets a status.</summary>
        public PushOutcome Push(IPushDocument doc, IReadOnlyList<ChangedParam> changes, PushOptions options)
        {
            var run = new Run { Doc = doc, Options = options, Out = new PushOutcome() };
            _log.Info("push " + doc.Path + ": " + changes.Count + " locators");
            if (doc.ReadOnly || doc.UserDirty)
            {
                var why = doc.ReadOnly ? "is read-only or locked" : "holds unsaved edits not made by cadetta";
                _log.Warn(R009 + ": " + doc.Path + " " + why);
                run.Out.Diagnostics.Add(PushDiagnostics.Make(R009, "error", doc.Path + " " + why + "; nothing was pushed", null));
                foreach (var c in changes)
                {
                    Status(run, c.Locator, PushStatus.Rejected, null, null, null);
                }
                run.Out.AnyFailed = changes.Count > 0;
                return run.Out;
            }

            var entries = new List<Entry>();
            foreach (var c in changes)
            {
                entries.Add(Parse(run, c));
            }
            var statuses = new Dictionary<Entry, LocatorStatus>();
            foreach (var e in entries.Where(x => !x.Done).OrderBy(x => (int)x.Kind))
            {
                Apply(run, e, entries, statuses);
            }
            Finish(run, entries, statuses);
            return run.Out;
        }

        /// <summary>Pushes, then runs the save step under <see cref="PushOptions.Save"/> through <paramref name="save"/>.</summary>
        public PushOutcome PushAndSave(IPushDocument doc, IReadOnlyList<ChangedParam> changes, PushOptions options, SaveService save, ISaveTarget target)
        {
            var outcome = Push(doc, changes, options);
            outcome.Save = save.Run(options.Save, outcome.AnyFailed, new[] { target });
            foreach (var d in outcome.Save.Diagnostics)
            {
                outcome.Diagnostics.Add(PushDiagnostics.Make("R009", "warning", d, null));
            }
            return outcome;
        }

        // ---- classification and parsing -------------------------------------------------

        private static readonly string[] Prefixes = { "global:", "dimension:", "property:", "frame:" };

        private Entry Parse(Run run, ChangedParam p)
        {
            var e = new Entry { Param = p, Name = p.Locator, WireTolerance = p.Tolerance };
            e.Kind = Classify(run.Doc, p, out var name);
            e.Name = name;
            var prec = run.Options.Precision;
            if (e.Kind == NativeKind.Frame)
            {
                if (!ParseFrame(p.Value, prec, e, out var err))
                {
                    Reject(run, e, err.Code, err.Message);
                }
                return e;
            }
            if (e.Kind == NativeKind.Property && p.Value is JsonElement pv && pv.ValueKind == JsonValueKind.String && !PushNumber.FromText(pv.GetString() ?? "", prec).Ok)
            {
                e.Text = pv.GetString() ?? "";
                return e;
            }
            e.Number = PushNumber.Parse(p.Value, prec);
            if (!e.Number.Ok)
            {
                Reject(run, e, e.Number.Code ?? R010, e.Number.Error ?? "value rejected");
            }
            else if (e.Kind == NativeKind.Property)
            {
                e.Text = e.Number.Text;
            }
            return e;
        }

        private static NativeKind Classify(IPushDocument doc, ChangedParam p, out string name)
        {
            name = p.Locator;
            NativeKind? forced = null;
            for (var i = 0; i < Prefixes.Length; i++)
            {
                if (name.StartsWith(Prefixes[i], StringComparison.Ordinal))
                {
                    name = name.Substring(Prefixes[i].Length);
                    forced = (NativeKind)i;
                    break;
                }
            }
            if (forced.HasValue) return forced.Value;
            if (p.Mode == PushMode.Frame) return NativeKind.Frame;
            if (p.Mode == PushMode.Ref || name.IndexOf('@') >= 0) return NativeKind.Dimension;
            if (doc.GetGlobal(name).HasValue) return NativeKind.Global;
            return doc.GetProperty(name) != null ? NativeKind.Property : NativeKind.Global;
        }

        private sealed class ParseError
        {
            public string Code = R010;
            public string Message = "";
        }

        private static bool ParseFrame(JsonElement? value, int precision, Entry e, out ParseError err)
        {
            err = new ParseError();
            if (value == null || value.Value.ValueKind != JsonValueKind.Array || value.Value.GetArrayLength() != 3)
            {
                err.Message = "a frame value is three rows of four numbers";
                return false;
            }
            var m = new double[12];
            var k = 0;
            foreach (var row in value.Value.EnumerateArray())
            {
                if (row.ValueKind != JsonValueKind.Array || row.GetArrayLength() != 4)
                {
                    err.Message = "a frame value is three rows of four numbers";
                    return false;
                }
                foreach (var cell in row.EnumerateArray())
                {
                    var n = PushNumber.Parse(cell, precision);
                    if (!n.Ok)
                    {
                        err.Code = n.Code ?? R010;
                        err.Message = n.Error ?? "entry rejected";
                        return false;
                    }
                    m[k++] = n.Value;
                }
            }
            e.Frame = m;
            e.FrameJson = value.Value.GetRawText();
            return true;
        }

        // ---- applying ---------------------------------------------------------------------

        private void Apply(Run run, Entry e, List<Entry> all, Dictionary<Entry, LocatorStatus> statuses)
        {
            LocatorStatus st;
            switch (e.Kind)
            {
                case NativeKind.Global: st = ApplyGlobal(run, e); break;
                case NativeKind.Dimension: st = ApplyDimension(run, e, all); break;
                case NativeKind.Property: st = ApplyProperty(run, e); break;
                default: st = ApplyFrame(run, e); break;
            }
            statuses[e] = st;
            _log.Info("locator " + e.Param.Locator + " -> " + st.Status);
        }

        private LocatorStatus ApplyGlobal(Run run, Entry e)
        {
            var num = e.Number!;
            var res = WriteGlobal(run, e.Name, num, out var old);
            return Make(e.Param.Locator, res, old, num.Text);
        }

        /// <summary>Writes a global once per push; returns the status of that write.</summary>
        private string WriteGlobal(Run run, string name, PushNumber num, out string? oldText)
        {
            var doc = run.Doc;
            var prior = doc.GetGlobal(name);
            oldText = prior.HasValue ? Fmt(prior.Value, run.Options.Precision) : null;
            if (!run.GlobalsHandled.Add(name))
            {
                _log.Debug("global " + name + " already written in this push");
                return PushStatus.Skipped;
            }
            if (prior.HasValue && Same(prior.Value, num.Value, run.Options.Precision))
            {
                _log.Debug("global " + name + " unchanged");
                return PushStatus.Skipped;
            }
            var expression = doc.GetGlobalExpression(name);
            var gv = PushGuard.ForGlobal(name, expression);
            if (!doc.SetGlobal(name, num.Value))
            {
                Diag(run, R010, "error", "global " + name + " refused the write", null);
                return PushStatus.Rejected;
            }
            run.Wrote = true;
            if (gv.Action == GuardAction.ReplaceExpression)
            {
                Diag(run, R006, "warning", gv.Reason!, null);
            }
            if (prior.HasValue)
            {
                var p = prior.Value;
                if (expression != null)
                {
                    run.Undo.Add(() => doc.RestoreEquation(name, expression));
                }
                else
                {
                    run.Undo.Add(() => doc.SetGlobal(name, p));
                }
            }
            else
            {
                run.Undo.Add(() => doc.DeleteGlobal(name));
            }
            return PushStatus.Pushed;
        }

        private LocatorStatus ApplyProperty(Run run, Entry e)
        {
            var doc = run.Doc;
            var prior = doc.GetProperty(e.Name);
            var text = e.Text ?? "";
            if (prior == text)
            {
                return Make(e.Param.Locator, PushStatus.Skipped, Quote(prior), Quote(text));
            }
            if (!doc.SetProperty(e.Name, text))
            {
                Diag(run, R010, "error", "property " + e.Name + " refused the write", e.Param.Path);
                return Make(e.Param.Locator, PushStatus.Rejected, Quote(prior), null);
            }
            run.Wrote = true;
            if (prior != null) run.Undo.Add(() => doc.SetProperty(e.Name, prior));
            else run.Undo.Add(() => doc.DeleteProperty(e.Name));
            return Make(e.Param.Locator, PushStatus.Pushed, Quote(prior), Quote(text));
        }

        private LocatorStatus ApplyFrame(Run run, Entry e)
        {
            var doc = run.Doc;
            var old = doc.GetFrame(e.Name);
            if (old == null && !run.Options.CreateDatums)
            {
                Diag(run, R003, "error", "coordinate system " + e.Name + " is missing and datums are verify-only", e.Param.Path);
                return Make(e.Param.Locator, PushStatus.Skipped, null, null);
            }
            var oldJson = old == null ? null : FrameJson(old, run.Options.Precision);
            if (old != null && SameFrame(old, e.Frame!, run.Options.Precision))
            {
                return Make(e.Param.Locator, PushStatus.Skipped, oldJson, e.FrameJson);
            }
            if (!doc.SetFrame(e.Name, e.Frame!, run.Options.CreateDatums))
            {
                Diag(run, R010, "error", "coordinate system " + e.Name + " refused the write", e.Param.Path);
                return Make(e.Param.Locator, PushStatus.Rejected, oldJson, null);
            }
            run.Wrote = true;
            if (old != null) run.Undo.Add(() => doc.SetFrame(e.Name, old, false));
            else run.Undo.Add(() => doc.DeleteFrame(e.Name));
            return Make(e.Param.Locator, PushStatus.Pushed, oldJson, e.FrameJson);
        }

        private LocatorStatus ApplyDimension(Run run, Entry e, List<Entry> all)
        {
            var doc = run.Doc;
            var prec = run.Options.Precision;
            var loc = e.Param.Locator;
            var info = doc.GetDimension(e.Name);
            if (info == null)
            {
                Diag(run, R003, "error", "dimension " + e.Name + " does not exist", e.Param.Path);
                return Make(loc, PushStatus.Skipped, null, null);
            }
            var oldText = Fmt(info.Value, prec);
            var verdict = PushGuard.ForDimension(e.Name, info);
            if (verdict.Action == GuardAction.Reject)
            {
                Diag(run, R010, "error", verdict.Reason!, e.Param.Path);
                return Make(loc, PushStatus.Rejected, oldText, null);
            }
            var num = e.Number!;
            ToleranceSpec? tol = null;
            if (e.WireTolerance != null)
            {
                var built = BuildTolerance(e.WireTolerance, num, prec);
                if (built.Error != null)
                {
                    Diag(run, built.Code, "error", built.Error, e.Param.Path);
                    return Make(loc, PushStatus.Rejected, oldText, null);
                }
                tol = built.Spec;
            }

            var changed = false;
            string? newText;
            var isRef = e.Param.Mode == PushMode.Ref;
            if (verdict.Action == GuardAction.ReplaceExpression)
            {
                var expr = verdict.OldExpression;
                Diag(run, R006, "warning", verdict.Reason!, e.Param.Path);
                if (!doc.RestoreEquation(e.Name, null))
                {
                    Diag(run, R010, "error", "dimension " + e.Name + " refused to drop its equation", e.Param.Path);
                    return Make(loc, PushStatus.Rejected, oldText, null);
                }
                run.Wrote = true;
                changed = true;
                run.Undo.Add(() => doc.RestoreEquation(e.Name, expr));
                info.Expression = null;
                info.LinkedGlobal = null;
            }
            var undoMark = run.Undo.Count;
            if (isRef)
            {
                var target = ResolveTarget(run, e, all);
                var gStatus = WriteGlobal(run, target, num, out _);
                if (gStatus == PushStatus.Rejected) return Make(loc, PushStatus.Rejected, oldText, null);
                changed |= gStatus == PushStatus.Pushed;
                if (!string.Equals(info.LinkedGlobal, target, StringComparison.Ordinal))
                {
                    var prior = info.LinkedGlobal;
                    if (!doc.SetDimensionLink(e.Name, target))
                    {
                        Diag(run, R010, "error", "dimension " + e.Name + " refused the link to " + target, e.Param.Path);
                        return Make(loc, PushStatus.Rejected, oldText, null);
                    }
                    run.Wrote = true;
                    changed = true;
                    run.Undo.Add(() => doc.SetDimensionLink(e.Name, prior));
                }
                newText = num.Text;
            }
            else
            {
                if (info.LinkedGlobal != null)
                {
                    var linked = info.LinkedGlobal;
                    Diag(run, R006, "warning", "dimension " + e.Name + " was linked to " + linked + "; the link is replaced by a value", e.Param.Path);
                    if (!doc.SetDimensionLink(e.Name, null))
                    {
                        Diag(run, R010, "error", "dimension " + e.Name + " refused to drop its link", e.Param.Path);
                        return Make(loc, PushStatus.Rejected, oldText, null);
                    }
                    run.Wrote = true;
                    changed = true;
                    run.Undo.Add(() => doc.SetDimensionLink(e.Name, linked));
                }
                if (info.LinkedGlobal != null || !Same(info.Value, num.Value, prec))
                {
                    var priorValue = info.Value;
                    if (!doc.SetDimensionValue(e.Name, num.Value))
                    {
                        Diag(run, R010, "error", "dimension " + e.Name + " refused the write", e.Param.Path);
                        return Make(loc, PushStatus.Rejected, oldText, null);
                    }
                    run.Wrote = true;
                    changed = true;
                    run.Undo.Add(() => doc.SetDimensionValue(e.Name, priorValue));
                }
                newText = num.Text;
            }

            if (tol != null && !tol.Equals(info.Tolerance))
            {
                var priorTol = info.Tolerance;
                if (!doc.SetDimensionTolerance(e.Name, tol))
                {
                    Diag(run, R010, "error", "dimension " + e.Name + " refused the tolerance", e.Param.Path);
                    return Make(loc, PushStatus.Rejected, oldText, newText);
                }
                run.Wrote = true;
                changed = true;
                run.Undo.Add(() => doc.SetDimensionTolerance(e.Name, priorTol));
            }
            var st = Make(loc, changed ? PushStatus.Pushed : PushStatus.Skipped, oldText, newText);
            if (tol != null)
            {
                st.Tolerance = tol.ToProtocol(decimal.Parse(num.Text ?? "0", CultureInfo.InvariantCulture));
            }
            _log.Debug("dimension " + e.Name + " undo entries " + (run.Undo.Count - undoMark));
            return st;
        }

        /// <summary>The global a ref dimension links to: ref_to, else the first global bound to the path, else the path's own created global (adapters.md 3.5).</summary>
        private static string ResolveTarget(Run run, Entry e, List<Entry> all)
        {
            if (!string.IsNullOrEmpty(e.Param.RefTo)) return e.Param.RefTo!;
            foreach (var other in all)
            {
                if (other.Kind == NativeKind.Global && other.Param.Path == e.Param.Path)
                {
                    return other.Name;
                }
            }
            return NameMangler.Mangle(e.Param.Path, run.Options.NamePrefix);
        }

        // ---- tolerance --------------------------------------------------------------------

        private sealed class ToleranceResult
        {
            public ToleranceSpec? Spec;
            public string Code = R010;
            public string? Error;
        }

        private static ToleranceResult BuildTolerance(DimTolerance t, PushNumber value, int precision)
        {
            var res = new ToleranceResult();
            var v = decimal.Parse(value.Text ?? "0", CultureInfo.InvariantCulture);
            bool Num(string? text, string what, out decimal d)
            {
                d = 0;
                if (text == null)
                {
                    res.Error = "tolerance " + t.Form + " lacks " + what;
                    return false;
                }
                var n = PushNumber.FromText(text, precision);
                if (!n.Ok)
                {
                    res.Code = n.Code ?? R010;
                    res.Error = n.Error;
                    return false;
                }
                d = decimal.Parse(n.Text!, CultureInfo.InvariantCulture);
                return true;
            }
            switch (t.Form)
            {
                case "symmetric":
                    if (Num(t.T, "t", out var half)) res.Spec = new ToleranceSpec(ToleranceKind.Symmetric, -half, half);
                    break;
                case "bilateral":
                    if (Num(t.Lower, "lower", out var lo) && Num(t.Upper, "upper", out var up)) res.Spec = new ToleranceSpec(ToleranceKind.Bilateral, lo, up);
                    break;
                case "limits":
                    if (Num(t.Min, "min", out var mn) && Num(t.Max, "max", out var mx)) res.Spec = new ToleranceSpec(ToleranceKind.Limit, mn - v, mx - v);
                    break;
                case "fit":
                    if (string.IsNullOrEmpty(t.Class))
                    {
                        res.Error = "tolerance fit lacks class";
                    }
                    else if (char.IsUpper(t.Class![0]))
                    {
                        res.Spec = new ToleranceSpec(ToleranceKind.Fit, hole: t.Class);
                    }
                    else
                    {
                        res.Spec = new ToleranceSpec(ToleranceKind.Fit, shaft: t.Class);
                    }
                    break;
                default:
                    res.Error = "unknown tolerance form '" + t.Form + "'";
                    break;
            }
            return res;
        }

        // ---- finishing --------------------------------------------------------------------

        private void Finish(Run run, List<Entry> entries, Dictionary<Entry, LocatorStatus> statuses)
        {
            var rebuildOk = true;
            if (run.Wrote)
            {
                rebuildOk = run.Doc.Rebuild();
                if (!rebuildOk)
                {
                    RestoreAfterFailure(run, statuses);
                }
            }
            else
            {
                _log.Info("nothing changed; no rebuild");
            }
            foreach (var e in entries)
            {
                if (e.Preset != null) run.Out.Statuses.Add(e.Preset);
                else if (statuses.TryGetValue(e, out var s)) run.Out.Statuses.Add(s);
            }
            run.Out.AnyFailed = run.Out.Statuses.Any(s => s.Status == PushStatus.Failed || s.Status == PushStatus.Rejected);
            _log.Info("push done: " + run.Out.Statuses.Count + " statuses, failed=" + run.Out.AnyFailed);
        }

        private void RestoreAfterFailure(Run run, Dictionary<Entry, LocatorStatus> statuses)
        {
            run.Out.RebuildFailed = true;
            _log.Warn(AtomicCode + ": rebuild failed after push; restoring snapshot");
            for (var i = run.Undo.Count - 1; i >= 0; i--)
            {
                run.Undo[i]();
            }
            var restored = run.Doc.Rebuild();
            var msg = restored
                ? AtomicCode + ": rebuild failed; the pre-push values were restored"
                : AtomicCode + ": rebuild failed and the restore also failed; the document may be inconsistent";
            Diag(run, AtomicCode, "error", msg, null);
            foreach (var s in statuses.Values)
            {
                if (s.Status == PushStatus.Pushed) s.Status = PushStatus.Failed;
            }
        }

        private const string AtomicCode = Restore.AtomicPush.Code;

        // ---- helpers ----------------------------------------------------------------------

        private void Reject(Run run, Entry e, string code, string message)
        {
            e.Done = true;
            Diag(run, code, "error", message, e.Param.Path);
            e.Preset = Make(e.Param.Locator, PushStatus.Rejected, null, null);
            _log.Warn(code + ": " + e.Param.Locator + ": " + message);
        }

        private static void Diag(Run run, string code, string severity, string message, string? path)
        {
            run.Out.Diagnostics.Add(PushDiagnostics.Make(code, severity, message, path));
        }

        private static void Status(Run run, string locator, string status, string? old, string? now, DimTolerance? tol)
        {
            var s = Make(locator, status, old, now);
            s.Tolerance = tol;
            run.Out.Statuses.Add(s);
        }

        private static LocatorStatus Make(string locator, string status, string? old, string? now)
        {
            return new LocatorStatus { Locator = locator, Status = status, Old = Json(old), New = Json(now) };
        }

        private static string? Quote(string? text) => text == null ? null : JsonSerializer.Serialize(text);

        private static JsonElement? Json(string? raw)
        {
            if (raw == null) return null;
            using (var doc = JsonDocument.Parse(raw))
            {
                return doc.RootElement.Clone();
            }
        }

        private static string Fmt(double d, int precision) => PushNumber.Trim(Math.Round((decimal)d, precision));

        private static bool Same(double a, double b, int precision) => Math.Abs(a - b) < 0.5 * Math.Pow(10, -precision);

        private static bool SameFrame(double[] a, double[] b, int precision)
        {
            for (var i = 0; i < 12; i++)
            {
                if (!Same(a[i], b[i], precision)) return false;
            }
            return true;
        }

        private static string FrameJson(double[] m, int precision)
        {
            var rows = new List<string>();
            for (var r = 0; r < 3; r++)
            {
                rows.Add("[" + string.Join(",", Enumerable.Range(0, 4).Select(c => Fmt(m[r * 4 + c], precision))) + "]");
            }
            return "[" + string.Join(",", rows) + "]";
        }
    }
}
