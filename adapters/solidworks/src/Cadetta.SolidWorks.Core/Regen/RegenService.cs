using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;
using Cadetta.SolidWorks.Protocol;
using Cadetta.SolidWorks.Push;

namespace Cadetta.SolidWorks.Regen
{
    // frob:ticket 01M4SV2TVM50WJRZ42ZHXVD96N
    /// <summary>The `regen` step: rebuilds each target and maps rebuild errors to R004 naming the feature (adapters.md 5, lints.md R004).</summary>
    public sealed class RegenService
    {
        /// <summary>The regen-failed diagnostic code.</summary>
        public const string RegenFailed = "R004";

        /// <summary>The unsaved-edits / read-only refusal code (OQ-8).</summary>
        public const string Refused = "R009";

        private readonly IRegenHost _host;
        private readonly Log _log;

        /// <summary>Creates a service that opens targets through <paramref name="host"/>.</summary>
        public RegenService(IRegenHost host, Log log)
        {
            _host = host;
            _log = log;
        }

        /// <summary>Regenerates each target file in order; one status per target, R004 or R009 diagnostics for the failures.</summary>
        public RegenResult Regen(IReadOnlyList<string> targets)
        {
            var result = new RegenResult();
            foreach (var target in targets)
            {
                using (var handle = _host.Open(target))
                {
                    var ok = RegenOne(handle.Document, target, result);
                    result.Targets.Add(new TargetStatus { Target = target, Status = ok ? RegenState.Ok : RegenState.Failed });
                }
            }
            _log.Info("regen done: " + result.Targets.Count + " targets, " + result.Diagnostics.Count + " diagnostics");
            return result;
        }

        private bool RegenOne(IRegenDocument doc, string target, RegenResult result)
        {
            if (doc.ReadOnly || doc.UserDirty)
            {
                var why = doc.ReadOnly ? "is read-only or checked out elsewhere" : "has unsaved changes the adapter did not make";
                _log.Warn(Refused + ": " + target + " " + why + "; not regenerating");
                result.Diagnostics.Add(PushDiagnostics.Make(Refused, "error", target + " " + why + "; regen refused", null));
                return false;
            }
            var errors = doc.Rebuild();
            if (errors.Count == 0)
            {
                _log.Info("regen " + target + " ok");
                return true;
            }
            foreach (var e in errors)
            {
                var msg = e.Feature.Length > 0
                    ? "rebuild failed at feature '" + e.Feature + "': " + e.Message
                    : "rebuild failed: " + e.Message;
                _log.Warn(RegenFailed + ": " + target + ": " + msg);
                result.Diagnostics.Add(PushDiagnostics.Make(RegenFailed, "error", msg, null));
            }
            return false;
        }
    }
}
