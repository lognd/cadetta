using System.Collections.Generic;
using Cadetta.SolidWorks.Logging;

namespace Cadetta.SolidWorks.Restore
{
    /// <summary>Outcome of one atomic push.</summary>
    public sealed class AtomicPushResult
    {
        /// <summary>True when the rebuild succeeded and the new values stand.</summary>
        public bool Rebuilt { get; set; }

        /// <summary>True when a failed rebuild was followed by a successful restore of the old values.</summary>
        public bool Restored { get; set; }

        /// <summary>The R004 diagnostic message, or null when the push succeeded.</summary>
        public string? Diagnostic { get; set; }
    }

    /// <summary>Snapshot, push, rebuild and restore on R004 for one document (adapters.md 2.3).</summary>
    public static class AtomicPush
    {
        /// <summary>The diagnostic code for a rebuild error after push.</summary>
        public const string Code = "R004";

        /// <summary>Pushes <paramref name="changes"/>; on rebuild failure sets the snapshotted old values and rebuilds again.</summary>
        public static AtomicPushResult Run(IRestorableDocument doc, IReadOnlyList<KeyValuePair<string, double>> changes, Log log)
        {
            var snapshot = new List<KeyValuePair<string, double>>();
            foreach (var change in changes)
            {
                var old = doc.GetValue(change.Key);
                if (old.HasValue)
                {
                    snapshot.Add(new KeyValuePair<string, double>(change.Key, old.Value));
                }
                else
                {
                    log.Debug("no snapshot for unresolved locator " + change.Key);
                }
            }
            log.Info("snapshotted " + snapshot.Count + " locators");

            foreach (var change in changes)
            {
                if (!doc.SetValue(change.Key, change.Value))
                {
                    log.Warn("write refused for " + change.Key);
                }
            }
            if (doc.Rebuild())
            {
                log.Info("rebuild succeeded; push stands");
                return new AtomicPushResult { Rebuilt = true };
            }

            log.Warn(Code + ": rebuild failed after push; restoring snapshot");
            var restoreOk = true;
            foreach (var old in snapshot)
            {
                if (!doc.SetValue(old.Key, old.Value))
                {
                    log.Error("restore write refused for " + old.Key);
                    restoreOk = false;
                }
            }
            restoreOk = doc.Rebuild() && restoreOk;
            if (restoreOk)
            {
                log.Info("snapshot restored");
                return new AtomicPushResult { Restored = true, Diagnostic = Code + ": rebuild failed; the pre-push values were restored" };
            }
            log.Error(Code + ": restore failed");
            return new AtomicPushResult { Diagnostic = Code + ": rebuild failed and the restore also failed; the document may be inconsistent" };
        }
    }
}
